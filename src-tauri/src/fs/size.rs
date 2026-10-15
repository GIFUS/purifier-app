use std::path::Path;

use walkdir::WalkDir;

#[derive(Debug, Default, Clone, Copy)]
pub struct SizeResult {
    pub bytes: u64,
    pub files: u64,
    pub skipped_reparse: u64,
    pub had_errors: bool,
}

pub fn dir_size(root: &Path) -> SizeResult {
    let mut out = SizeResult::default();

    if root.is_file() {
        if let Ok(meta) = root.metadata() {
            out.bytes = meta.len();
            out.files = 1;
        }
        return out;
    }

    for entry in WalkDir::new(root).follow_links(false).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                out.had_errors = true;
                continue;
            }
        };

        if entry.file_type().is_symlink() || super::path::is_reparse_path(entry.path()) {
            out.skipped_reparse += 1;
            continue;
        }

        let Ok(meta) = entry.metadata() else {
            out.had_errors = true;
            continue;
        };

        if meta.is_file() {
            out.files += 1;
            out.bytes = out.bytes.saturating_add(meta.len());
        }
    }
    out
}
