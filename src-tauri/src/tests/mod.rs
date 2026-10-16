//! Cross-cutting tests for path handling, measuring, removal, safety and the
//! catalog, grouped by the module they cover.

mod catalog;
mod inspect;
mod path;
mod remove;
mod size;
mod validate;

use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// `sub/deep`, `empty`, and three files of 1000, 2000 and 3000 bytes.
pub(super) fn fixture() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path();
    fs::create_dir_all(root.join("sub/deep")).unwrap();
    fs::create_dir_all(root.join("empty")).unwrap();
    fs::write(root.join("a.bin"), vec![0u8; 1000]).unwrap();
    fs::write(root.join("sub/b.bin"), vec![0u8; 2000]).unwrap();
    fs::write(root.join("sub/deep/c.bin"), vec![0u8; 3000]).unwrap();
    dir
}

pub(super) fn write_file(path: &Path, size: usize) {
    fs::write(path, vec![0u8; size]).unwrap();
}
