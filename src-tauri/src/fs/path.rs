use std::path::{Component, Path, PathBuf};

use std::os::windows::fs::MetadataExt;

pub const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
pub const FILE_ATTRIBUTE_READONLY: u32 = 0x1;

pub fn expand_env(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(offset) => {
                let name = &after[..offset];
                match std::env::var(name) {
                    Ok(value) => out.push_str(&value),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[offset + 1..];
            }
            None => {
                out.push('%');
                rest = after;
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn normalize(input: &str) -> PathBuf {
    let unified = expand_env(input).replace('/', "\\");
    let trimmed = unified.trim();

    let (root, rest) = split_root(trimmed);
    let mut tail: Vec<&str> = Vec::new();
    for segment in rest.split('\\') {
        match segment {
            "" | "." => {}
            ".." => {
                tail.pop();
            }
            other => tail.push(other),
        }
    }

    if tail.is_empty() {
        PathBuf::from(root)
    } else {
        PathBuf::from(format!(
            "{}\\{}",
            root.trim_end_matches('\\'),
            tail.join("\\")
        ))
    }
}

fn split_root(path: &str) -> (&str, &str) {
    for prefix in ["\\\\?\\UNC\\", "\\\\.\\UNC\\"] {
        if let Some(after) = path.strip_prefix(prefix) {
            if let Some((server, after_server)) = after.split_once('\\') {
                let share_end = match after_server.find('\\') {
                    Some(index) => prefix.len() + server.len() + 1 + index,
                    None => path.len(),
                };
                let split_at = (share_end + 1).min(path.len());
                return (&path[..split_at], &path[split_at..]);
            }
            return (path, "");
        }
    }

    for prefix in ["\\\\?\\", "\\\\.\\"] {
        if let Some(after) = path.strip_prefix(prefix) {
            let _ = prefix;
            return split_root(after);
        }
    }

    if let Some(after) = path.strip_prefix("\\\\") {
        if let Some((server, after_server)) = after.split_once('\\') {
            let split_at = (2 + server.len() + 1 + after_server.len() + 1).min(path.len());
            return (&path[..split_at], &path[split_at..]);
        }
        return (path, "");
    }

    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        if bytes.len() >= 3 && bytes[2] == b'\\' {
            return (&path[..3], &path[3..]);
        }
        return (&path[..2], &path[2..]);
    }

    if let Some(rest) = path.strip_prefix('\\') {
        return ("\\", rest);
    }
    (path, "")
}

pub fn strip_verbatim(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();

    if let Some(rest) = text.strip_prefix("\\\\?\\UNC\\") {
        return PathBuf::from(format!("\\\\{rest}"));
    }
    match text.strip_prefix("\\\\?\\") {
        Some(rest) => PathBuf::from(rest),
        None => path.to_path_buf(),
    }
}

pub fn verbatim(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();

    if text.starts_with("\\\\?\\") || text.starts_with("\\\\.\\") || !path.is_absolute() {
        return path.to_path_buf();
    }
    if let Some(rest) = text.strip_prefix("\\\\") {
        return PathBuf::from(format!("\\\\?\\UNC\\{rest}"));
    }
    PathBuf::from(format!("\\\\?\\{text}"))
}

pub fn resolve(input: &str) -> PathBuf {
    let normalized = normalize(input);
    strip_verbatim(&normalized.canonicalize().unwrap_or(normalized))
}

pub fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

pub fn is_reparse_path(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| is_reparse(&m))
        .unwrap_or(false)
}

pub fn is_volume_root(path: &Path) -> bool {
    let mut components = path.components();
    matches!(
        (components.next(), components.next(), components.next()),
        (Some(Component::Prefix(_)), Some(Component::RootDir), None)
            | (Some(Component::RootDir), None, None)
    )
}

pub fn is_within(candidate: &Path, base: &Path) -> bool {
    let candidate_path = normalize(&candidate.to_string_lossy());
    let base_path = normalize(&base.to_string_lossy());
    if !base_path.has_root() || is_volume_root(&base_path) {
        return false;
    }
    candidate_path.starts_with(&base_path)
}

pub fn modified_secs(metadata: &std::fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

pub fn format_modified(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (hour, minute) = (rem / 3600, (rem % 3600) / 60);

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02} {hour:02}:{minute:02}")
}
