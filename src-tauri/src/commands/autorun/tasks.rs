use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use quick_xml::events::Event;
use quick_xml::Reader;

use super::paths::target_exists;
use super::{AutorunTask, SCOPE_MACHINE, SCOPE_USER};
use crate::error::{AppError, Result};

const TASK_ROOT: &str = r"C:\Windows\System32\Tasks";

/// A GUI process has no console of its own, so Windows would give schtasks a
/// new window for the few milliseconds it runs - a flash on every toggle.
/// This hides the window and leaves the pipes alone.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub(super) fn collect() -> Vec<AutorunTask> {
    let mut tasks = Vec::new();
    collect_tasks(Path::new(TASK_ROOT), "", &mut tasks);
    tasks
}

pub(super) fn set_enabled(folder: &str, name: &str, enabled: bool) -> Result<()> {
    let Some(full) = task_reference(folder, name) else {
        return Err(AppError::Other("scheduled task has no name".to_string()));
    };

    let flag = if enabled { "ENABLE" } else { "DISABLE" };
    let output = Command::new("schtasks.exe")
        .arg("/Change")
        .arg("/TN")
        .arg(&full)
        .arg(format!("/{flag}"))
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| AppError::Other(format!("cannot run schtasks: {e}")))?;

    if output.status.success() {
        return Ok(());
    }

    let reason = decode_console(&output.stderr)
        .or_else(|| decode_console(&output.stdout))
        .filter(|text| is_readable(text))
        .unwrap_or_else(|| "the task is protected or access was denied".to_string());

    Err(AppError::Other(format!("cannot switch {full}: {reason}")))
}

// schtasks reads a forward slash as part of the name and then finds no file.
fn task_reference(folder: &str, name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    // The folder already carries its leading separator; "" and "\" trim to empty.
    let folder = folder.trim();
    let parent = folder.trim_end_matches('\\');
    Some(format!("{parent}\\{name}"))
}

/// Sign-in triggers only, and never Microsoft's own folders.
fn collect_tasks(dir: &Path, folder: &str, out: &mut Vec<AutorunTask>) {
    let Ok(read) = fs::read_dir(dir) else { return };

    for item in read.flatten() {
        let path = item.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        if path.is_dir() {
            if folder_is_listed(file_name) {
                collect_tasks(&path, &format!("{folder}\\{file_name}"), out);
            }
        } else if path.extension().is_none() {
            // Tasks are the extension-less files; anything else is somebody's backup.
            if let Some(task) = fs::read(&path)
                .ok()
                .and_then(|bytes| decode_utf16(&bytes))
                .and_then(|xml| parse_task(&xml, folder, file_name))
            {
                out.push(task);
            }
        }
    }
}

fn folder_is_listed(name: &str) -> bool {
    !name.to_lowercase().contains("microsoft")
}

fn parse_task(xml: &str, folder: &str, file_name: &str) -> Option<AutorunTask> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut at_logon = false;
    let mut enabled = true;
    let mut commands: Vec<String> = Vec::new();

    // <Enabled> is read only inside <Settings>; triggers carry their own copy.
    let mut inside_triggers = false;
    let mut inside_settings = false;
    let mut inside_actions = false;
    let mut inside_exec = false;
    let mut inside_logon = false;
    let mut has_user_id = false;
    let mut current = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Eof) | Err(_) => break,
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_string();
                current = name.clone();
                match name.as_str() {
                    "Triggers" => inside_triggers = true,
                    "Settings" => inside_settings = true,
                    "Actions" => inside_actions = true,
                    "Exec" => inside_exec = true,
                    "LogonTrigger" if inside_triggers => {
                        at_logon = true;
                        inside_logon = true;
                    }
                    "BootTrigger" => {}
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_string();
                match name.as_str() {
                    "Triggers" => inside_triggers = false,
                    "Settings" => inside_settings = false,
                    "Actions" => inside_actions = false,
                    "Exec" => inside_exec = false,
                    "LogonTrigger" => inside_logon = false,
                    _ => {}
                }
                current.clear();
            }
            Ok(Event::Text(e)) => {
                let text = e.into_inner().to_string();
                if inside_settings && current == "Enabled" {
                    enabled = text != "false";
                } else if inside_exec && inside_actions && current == "Command" {
                    commands.push(text);
                } else if inside_exec && inside_actions && current == "Arguments" {
                    if let Some(last) = commands.last_mut() {
                        last.push(' ');
                        last.push_str(&text);
                    }
                } else if inside_logon && current == "UserId" && !text.is_empty() {
                    has_user_id = true;
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.name().as_ref().to_string();
                if inside_triggers && name == "LogonTrigger" {
                    at_logon = true;
                }
            }
            Ok(_) => {}
        }
    }

    if !at_logon {
        return None;
    }
    let command = commands
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" | ");
    if command.is_empty() {
        return None;
    }

    // Existence is decided by the first action: that is the one the entry runs.
    let exists = target_exists(commands.first().map_or("", |part| part.trim()));

    Some(AutorunTask {
        name: file_name.to_string(),
        path: if folder.is_empty() {
            "\\".to_string()
        } else {
            folder.to_string()
        },
        command,
        enabled,
        exists,
        scope: if has_user_id {
            SCOPE_MACHINE
        } else {
            SCOPE_USER
        }
        .to_string(),
    })
}

/// schtasks writes UTF-16 LE with a BOM; task files are UTF-16 with or without
/// one, and read as UTF-8 either turns to noise.
fn decode_console(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    if has_utf16_mark(bytes) {
        return Some(String::from_utf16_lossy(&utf16_units(bytes)));
    }
    Some(String::from_utf8_lossy(bytes).to_string())
}

fn is_readable(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && !trimmed.contains('\u{FFFD}') && trimmed.chars().any(|c| !c.is_control())
}

fn decode_utf16(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf16(&utf16_units(bytes)).ok()?;
    let trimmed = text.trim_end_matches('\0');
    if trimmed.trim().is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn has_utf16_mark(bytes: &[u8]) -> bool {
    matches!(bytes, [0xFF, 0xFE, ..])
}

fn utf16_units(bytes: &[u8]) -> Vec<u16> {
    let body = match bytes {
        [0xFF, 0xFE, rest @ ..] => rest,
        _ => bytes,
    };
    body.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logon_task(trigger: &str, enabled: &str, command: &str, user_id: Option<&str>) -> String {
        let user = user_id
            .map(|id| format!("<UserId>{id}</UserId>"))
            .unwrap_or_default();
        format!(
            r#"<Task xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers><{trigger}>{user}<Enabled>true</Enabled></{trigger}></Triggers>
  <Settings><Enabled>{enabled}</Enabled></Settings>
  <Actions Context="Author"><Exec><Command>{command}</Command></Exec></Actions>
</Task>"#
        )
    }

    #[test]
    fn a_boot_triggered_task_is_not_on_an_autostart_page() {
        let xml = logon_task("BootTrigger", "true", r#""D:\Apps\a.exe""#, None);
        assert!(parse_task(&xml, "", "A").is_none());
    }

    #[test]
    fn a_logon_triggered_task_is_kept() {
        let xml = logon_task("LogonTrigger", "true", r#""D:\Apps\a.exe""#, None);
        let task = parse_task(&xml, "\\ASUS", "A").expect("task should parse");
        assert_eq!(task.name, "A");
        assert_eq!(task.path, "\\ASUS");
        assert_eq!(task.command, r#""D:\Apps\a.exe""#);
        assert!(task.enabled);
        assert_eq!(task.scope, SCOPE_USER);
    }

    #[test]
    fn the_settings_flag_wins_over_the_trigger_flag() {
        let xml = logon_task("LogonTrigger", "false", "C:\\a.exe", None);
        let task = parse_task(&xml, "", "A").expect("task should parse");
        assert!(!task.enabled);
    }

    #[test]
    fn a_task_in_the_root_folder_keeps_one_leading_separator() {
        assert_eq!(
            task_reference("\\", "v2rayNAutoRun_abc").as_deref(),
            Some(r"\v2rayNAutoRun_abc")
        );
    }

    #[test]
    fn a_nested_task_keeps_its_backslashes() {
        assert_eq!(
            task_reference("\\ASUS\\", "P508PowerAgent_sdk").as_deref(),
            Some(r"\ASUS\P508PowerAgent_sdk")
        );
    }

    #[test]
    fn a_folder_without_a_trailing_separator_still_joins_correctly() {
        assert_eq!(
            task_reference("\\ASUS", "Framework Service").as_deref(),
            Some(r"\ASUS\Framework Service")
        );
    }

    #[test]
    fn an_empty_folder_still_yields_an_absolute_reference() {
        assert_eq!(
            task_reference("", "Something").as_deref(),
            Some(r"\Something")
        );
    }

    #[test]
    fn a_task_without_a_name_has_no_reference() {
        assert_eq!(task_reference("\\ASUS\\", "   "), None);
        assert_eq!(task_reference("\\ASUS\\", ""), None);
    }

    #[test]
    fn the_reference_never_contains_a_forward_slash() {
        for (folder, name) in [("\\", "a"), ("\\ASUS\\", "b"), ("\\ASUS", "c")] {
            let reference = task_reference(folder, name).expect("reference");
            assert!(
                !reference.contains('/'),
                "{reference} would be rejected by schtasks"
            );
        }
    }

    #[test]
    fn utf16_with_a_mark_is_decoded_rather_than_mangled() {
        let text = "ОТКАЗАНО В ДОСТУПЕ";
        let mut bytes = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(decode_console(&bytes).as_deref(), Some(text));
        assert!(is_readable(&decode_console(&bytes).unwrap_or_default()));
    }

    #[test]
    fn utf8_streams_still_decode() {
        let bytes = b"ACCESS DENIED";
        assert_eq!(decode_console(bytes).as_deref(), Some("ACCESS DENIED"));
    }

    #[test]
    fn an_empty_stream_yields_nothing() {
        assert_eq!(decode_console(&[]), None);
        assert!(!is_readable(""));
        assert!(!is_readable("   "));
    }

    #[test]
    fn a_mangled_stream_is_not_offered_as_a_reason() {
        assert!(!is_readable("\u{FFFD}\u{FFFD} cannot read"));
    }

    #[test]
    fn a_task_named_after_a_user_counts_as_machine_wide() {
        let xml = logon_task("LogonTrigger", "true", "C:\\a.exe", Some("DOMAIN\\bob"));
        let task = parse_task(&xml, "", "A").expect("task should parse");
        assert_eq!(task.scope, SCOPE_MACHINE);
    }

    #[test]
    fn a_task_without_a_command_is_dropped() {
        let xml = logon_task("LogonTrigger", "true", "", None);
        assert!(parse_task(&xml, "", "A").is_none());
    }

    #[test]
    fn several_actions_are_joined_into_one_line() {
        let xml = r#"<Task><Triggers><LogonTrigger/></Triggers><Settings><Enabled>true</Enabled></Settings>
          <Actions><Exec><Command>C:\one.exe</Command></Exec><Exec><Command>C:\two.exe</Command><Arguments>/x</Arguments></Exec></Actions></Task>"#;
        let task = parse_task(xml, "", "A").expect("task should parse");
        assert_eq!(task.command, r"C:\one.exe | C:\two.exe /x");
    }

    #[test]
    fn the_windows_folder_is_skipped_but_others_are_not() {
        assert!(!folder_is_listed("Microsoft"));
        assert!(!folder_is_listed("microsoft"));
        assert!(folder_is_listed("ASUS"));
        assert!(folder_is_listed("Windows"));
    }

    #[test]
    fn utf16_with_a_bom_and_without_both_decode() {
        let text = "<Task/>";
        let utf16: Vec<u16> = text.encode_utf16().chain([0]).collect();
        let mut with_bom: Vec<u8> = vec![0xFF, 0xFE];
        for unit in &utf16 {
            with_bom.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(decode_utf16(&with_bom).as_deref(), Some(text));

        let without_bom: Vec<u8> = utf16.iter().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(decode_utf16(&without_bom).as_deref(), Some(text));
    }

    #[test]
    fn hiding_the_window_still_delivers_the_output() {
        let output = Command::new("cmd")
            .args(["/c", "echo", "PIPED"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .expect("cmd should run");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("PIPED"),
            "the flag must not cost us the pipes: {stdout:?}"
        );
    }
}
