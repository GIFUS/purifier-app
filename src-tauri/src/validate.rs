use std::path::{Path, PathBuf};

use crate::categories;
use crate::error::{AppError, Result};
use crate::fs;

fn protected_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for var in [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PUBLIC",
    ] {
        if let Ok(value) = std::env::var(var) {
            if !value.trim().is_empty() {
                out.push(fs::resolve(&value));
            }
        }
    }
    out
}

fn protected_subtrees() -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Ok(windir) = std::env::var("SystemRoot") {
        let root = fs::resolve(&windir);
        for suffix in [
            "\\System32",
            "\\SysWOW64",
            "\\WinSxS",
            "\\Installer",
            "\\assembly",
            "\\Boot",
            "\\System32\\config",
            "\\System32\\DriverStore",
            "\\servicing",
            "\\Tasks",
        ] {
            out.push(root.join(suffix));
        }
    }
    if let Ok(program_data) = std::env::var("ProgramData") {
        out.push(fs::resolve(&program_data).join("Package Cache"));
    }
    out
}

pub struct SecurityGuard {
    allowed: Vec<PathBuf>,
}

impl SecurityGuard {
    pub fn from_targets(targets: Vec<PathBuf>) -> Self {
        Self {
            allowed: targets
                .iter()
                .map(|p| fs::resolve(&p.to_string_lossy()))
                .collect(),
        }
    }

    pub fn for_categories(ids: &[String]) -> Result<Self> {
        let mut targets = Vec::new();
        for id in ids {
            let category = categories::lookup(id)?;
            targets.extend(crate::locate::resolve_all(category.locators));
        }
        Ok(Self::from_targets(targets))
    }

    pub fn check(&self, category_id: &str, raw_path: &str) -> Result<PathBuf> {
        categories::lookup(category_id)?;

        let candidate = fs::resolve(raw_path);

        if fs::is_volume_root(&candidate) {
            return Err(AppError::Unsafe(format!("{candidate:?} is a volume root")));
        }
        for root in protected_roots() {
            if same_path(&candidate, &root) {
                return Err(AppError::Unsafe(format!(
                    "{candidate:?} is a protected root"
                )));
            }
        }
        for subtree in protected_subtrees() {
            if fs::is_within(&candidate, &subtree) {
                return Err(AppError::Unsafe(format!("{candidate:?} is protected")));
            }
        }
        if fs::is_reparse_path(&candidate) {
            return Err(AppError::Unsafe(format!(
                "{candidate:?} is a junction or symlink"
            )));
        }

        let matches_any = self
            .allowed
            .iter()
            .any(|base| fs::is_within(&candidate, base));
        if !matches_any {
            return Err(AppError::Unsafe(format!(
                "{candidate:?} is not a target of category {category_id}"
            )));
        }
        Ok(candidate)
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy()
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(b.to_string_lossy().trim_end_matches('\\'))
}
