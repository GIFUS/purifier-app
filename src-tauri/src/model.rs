use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub enum Locator {
    Fixed(&'static str),
    Env(&'static str),
    Pattern {
        base: &'static str,
        pattern: &'static str,
    },
    Join(&'static [Locator], &'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Danger {
    Safe,
    Caution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupMode {
    ClearContents,
    RemoveDir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Dir,
    File,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CategoryDef {
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub danger: Danger,
    pub mode: CleanupMode,
    pub locators: &'static [Locator],
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetInfo {
    pub path: String,
    pub kind: TargetKind,
    pub exists: bool,
    pub size_bytes: u64,
    pub file_count: u64,
    pub last_modified: Option<String>,
    pub reparse: bool,
    pub writable: bool,
    pub locked: bool,
    pub error: Option<String>,
}

impl TargetInfo {
    pub fn missing(path: impl AsRef<std::path::Path>) -> Self {
        Self {
            path: path.as_ref().display().to_string(),
            kind: TargetKind::Dir,
            exists: false,
            size_bytes: 0,
            file_count: 0,
            last_modified: None,
            reparse: false,
            writable: false,
            locked: false,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanEntry {
    pub category: CategoryDef,
    pub targets: Vec<TargetInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub category_id: &'static str,
    pub category_label: &'static str,
    pub group: &'static str,
    pub index: usize,
    pub total: usize,
    pub target: TargetInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CleanReport {
    pub failed: Vec<Failure>,
    pub freed_bytes: u64,
    pub files_removed: u64,
    pub blocked: u64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Scanning,
    Cleaning,
    Done,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub phase: Phase,
    pub done: usize,
    pub total: usize,
    pub indeterminate: bool,
    pub message: String,
}
