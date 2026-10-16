use std::os::windows::fs::MetadataExt;

use crate::cleanup::model::{TargetInfo, TargetKind};

use super::size;
use crate::common::path as fspath;

pub fn probe(raw: &str) -> TargetInfo {
    let path = fspath::resolve(raw);

    let Ok(link_meta) = std::fs::symlink_metadata(&path) else {
        return TargetInfo::missing(&path);
    };

    let kind = if link_meta.is_dir() {
        TargetKind::Dir
    } else {
        TargetKind::File
    };

    if fspath::is_reparse(&link_meta) {
        return TargetInfo {
            path: path.display().to_string(),
            kind: TargetKind::Dir,
            exists: true,
            size_bytes: 0,
            file_count: 0,
            last_modified: fspath::modified_secs(&link_meta).map(fspath::format_modified),
            reparse: true,
            writable: false,
            locked: false,
            error: None,
        };
    }

    let Ok(meta) = std::fs::metadata(&path) else {
        return TargetInfo {
            path: path.display().to_string(),
            kind,
            exists: true,
            size_bytes: 0,
            file_count: 0,
            last_modified: None,
            reparse: false,
            writable: false,
            locked: false,
            error: Some("cannot read folder metadata".to_string()),
        };
    };

    let (bytes, files, locked) = if meta.is_dir() {
        let r = size::dir_size(&path);
        (r.bytes, r.files, r.had_errors)
    } else {
        (meta.len(), 1, false)
    };

    TargetInfo {
        path: path.display().to_string(),
        kind,
        exists: true,
        size_bytes: bytes,
        file_count: files,
        last_modified: fspath::modified_secs(&meta).map(fspath::format_modified),
        reparse: false,
        writable: meta.file_attributes() & crate::common::path::FILE_ATTRIBUTE_READONLY == 0,
        locked,
        error: None,
    }
}
