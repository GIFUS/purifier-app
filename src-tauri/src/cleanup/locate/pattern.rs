use std::path::PathBuf;

pub fn resolve_pattern(base: &str, pattern: &str) -> Vec<PathBuf> {
    let base_path = crate::common::resolve(base);
    if !base_path.exists() {
        return Vec::new();
    }

    let full = base_path.join(pattern);
    let raw = full.to_string_lossy().into_owned();
    match glob::glob(&raw) {
        Ok(entries) => entries.flatten().filter(|p| p.is_file()).collect(),
        Err(_) => Vec::new(),
    }
}
