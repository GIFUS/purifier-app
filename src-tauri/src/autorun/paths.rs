use std::path::Path;

use crate::common::path::expand_env;

pub(super) const STARTUP_EXTENSIONS: [&str; 7] = ["lnk", "url", "exe", "bat", "cmd", "vbs", "js"];

/// An unquoted path may contain spaces and nothing marks where it ends, so the
/// widest live candidate wins.
pub(super) fn target_exists(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return true;
    }

    // Quoted: the quotes end the path exactly, whatever follows them.
    if let Some(rest) = trimmed.strip_prefix('"') {
        let inner = rest.split('"').next().unwrap_or_default().trim();
        return inner.is_empty() || exists(inner);
    }

    // A bare name resolves through PATH, which cannot be judged from here.
    if !is_absolute_path(trimmed) {
        return true;
    }

    if exists(trimmed) {
        return true;
    }

    let Some(first_space) = trimmed.find(char::is_whitespace) else {
        return false;
    };
    let head = trimmed[..first_space].trim_end();
    if head.is_empty() {
        return false;
    }
    if exists(head) {
        return true;
    }

    match cut_at_executable_extension(trimmed, first_space) {
        Some(candidate) => exists(&candidate),
        None => false,
    }
}

fn is_absolute_path(path: &str) -> bool {
    path.starts_with('\\')
        || (path.starts_with(|c: char| c.is_ascii_alphabetic())
            && path.as_bytes().get(1) == Some(&b':'))
}

/// A boundary keeps `.executive` from reading as `.exe`.
fn cut_at_executable_extension(text: &str, after: usize) -> Option<String> {
    const EXTENSIONS: [&str; 4] = ["exe", "com", "bat", "cmd"];
    let lower = text.to_lowercase();
    let mut best: Option<usize> = None;

    for extension in EXTENSIONS {
        let needle = format!(".{extension}");
        let mut from = after;
        while from < lower.len() {
            let Some(offset) = lower[from..].find(&needle) else {
                break;
            };
            let start = from + offset;
            let end = start + needle.len();
            let boundary = !matches!(text[end..].chars().next(), Some(c) if c.is_alphanumeric());
            if boundary {
                best = Some(best.map_or(end, |current: usize| current.min(end)));
            }
            from = end.min(from + 1);
            if from <= start {
                from = start + 1;
            }
        }
    }

    best.map(|end| text[..end].trim_end().to_string())
}

fn exists(path: &str) -> bool {
    !path.is_empty() && Path::new(&expand_env(path)).exists()
}

pub(super) fn startup_extension(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        STARTUP_EXTENSIONS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_path_is_checked_exactly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let exe = dir.path().join("Some App.exe");
        std::fs::write(&exe, b"").expect("write exe");

        assert!(target_exists(&format!(r#""{}" --silent"#, exe.display())));
        assert!(!target_exists(&format!(
            r#""{}\missing.exe" --silent"#,
            dir.path().display()
        )));
    }

    #[test]
    fn a_nested_quoted_argument_does_not_cut_the_path_short() {
        let dir = tempfile::tempdir().expect("temp dir");
        let exe = dir.path().join("Discord Update.exe");
        std::fs::write(&exe, b"").expect("write exe");

        let command = format!(
            r#""{}" --processStart Discord.exe --process-start-args "--start-inactive""#,
            exe.display()
        );
        assert!(target_exists(&command));
    }

    #[test]
    fn an_unquoted_path_with_spaces_survives_while_it_is_installed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let exe = dir.path().join("Program Files App.exe");
        std::fs::write(&exe, b"").expect("write exe");

        let command = format!(r"{} --silent", exe.display());
        assert!(target_exists(&command));
    }

    #[test]
    fn an_uninstalled_program_is_missing_even_when_its_folder_survives() {
        let parent = tempfile::tempdir().expect("temp dir");
        let survivor = parent.path().join("ArmouryDevice");
        std::fs::create_dir(&survivor).expect("mkdir");

        let gone = format!(r"{}\dll\SocketServer.exe", survivor.display());
        assert!(exists(&format!(r"{}", survivor.display())));
        assert!(!target_exists(&gone));
    }

    #[test]
    fn a_bare_name_is_left_alone_because_path_lookup_cannot_be_judged() {
        assert!(target_exists("rundll32.exe"));
        assert!(target_exists("wscript.exe something.js"));
    }

    #[test]
    fn an_empty_command_is_not_called_missing() {
        assert!(target_exists(""));
        assert!(target_exists("   "));
    }

    #[test]
    fn an_extension_that_is_only_a_prefix_is_not_cut() {
        assert_eq!(
            cut_at_executable_extension(r".executive\App.exe", 0),
            Some(r".executive\App.exe".to_string())
        );
        assert_eq!(
            cut_at_executable_extension(r".dll\Server.exe", 0),
            Some(r".dll\Server.exe".to_string())
        );
    }

    #[test]
    fn the_earliest_executable_extension_wins() {
        assert_eq!(
            cut_at_executable_extension(r".exe --arg C:\other.com", 0),
            Some(".exe".to_string())
        );
    }
}
