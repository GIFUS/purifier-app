use std::path::PathBuf;

pub fn resolve_env(var: &str) -> Vec<PathBuf> {
    let name = var.trim().trim_matches('%');
    if name.is_empty() {
        return Vec::new();
    }
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => vec![crate::fs::resolve(&value)],
        _ => Vec::new(),
    }
}

pub const LOCAL_APPDATA: &str = "%LOCALAPPDATA%";
pub const APPDATA: &str = "%APPDATA%";
pub const PROGRAMDATA: &str = "%PROGRAMDATA%";
pub const TEMP: &str = "%TEMP%";
pub const USERPROFILE: &str = "%USERPROFILE%";
