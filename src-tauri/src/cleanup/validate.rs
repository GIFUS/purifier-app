use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::cleanup::categories;
use crate::common;
use crate::common::error::{AppError, Result};

// The environment cannot change under a running cleaner, so the deny lists are
// read once instead of on every guarded path.
fn protected_roots() -> &'static [PathBuf] {
    static ROOTS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    ROOTS.get_or_init(|| {
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
                    out.push(common::resolve(&value));
                }
            }
        }
        out
    })
}

fn protected_subtrees() -> &'static [PathBuf] {
    static SUBTREES: OnceLock<Vec<PathBuf>> = OnceLock::new();
    SUBTREES.get_or_init(|| {
        let mut out = Vec::new();

        if let Ok(windir) = std::env::var("SystemRoot") {
            let root = common::resolve(&windir);
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
            out.push(common::resolve(&program_data).join("Package Cache"));
        }
        out
    })
}

pub struct SecurityGuard {
    allowed: Vec<PathBuf>,
}

impl SecurityGuard {
    pub fn from_targets(targets: Vec<PathBuf>) -> Self {
        Self {
            allowed: targets
                .iter()
                .map(|p| common::resolve(&p.to_string_lossy()))
                .collect(),
        }
    }

    pub fn for_categories(ids: &[String]) -> Result<Self> {
        let mut targets = Vec::new();
        for id in ids {
            let category = categories::lookup(id)?;
            targets.extend(crate::cleanup::locate::resolve_all(category.locators));
        }
        Ok(Self::from_targets(targets))
    }

    pub fn check(&self, category_id: &str, raw_path: &str) -> Result<PathBuf> {
        categories::lookup(category_id)?;

        let candidate = common::resolve(raw_path);

        if common::is_volume_root(&candidate) {
            return Err(AppError::Unsafe(format!("{candidate:?} is a volume root")));
        }
        for root in protected_roots() {
            if same_path(&candidate, root) {
                return Err(AppError::Unsafe(format!(
                    "{candidate:?} is a protected root"
                )));
            }
        }
        for subtree in protected_subtrees() {
            if common::is_within(&candidate, subtree) {
                return Err(AppError::Unsafe(format!("{candidate:?} is protected")));
            }
        }
        if common::is_reparse_path(&candidate) {
            return Err(AppError::Unsafe(format!(
                "{candidate:?} is a junction or symlink"
            )));
        }

        let matches_any = self
            .allowed
            .iter()
            .any(|base| common::is_within(&candidate, base));
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
