use std::path::Path;

use tempfile::TempDir;

use crate::cleanup::fs as sysfs;

use super::{fixture, write_file};

#[test]
fn dir_size_totals_nested_files() {
    let dir = fixture();
    let result = sysfs::size::dir_size(dir.path());
    assert_eq!(result.bytes, 6000);
    assert_eq!(result.files, 3);
    assert!(!result.had_errors);
}

#[test]
fn dir_size_of_missing_path_is_zero() {
    let result = sysfs::size::dir_size(Path::new("C:\\definitely\\not\\here"));
    assert_eq!(result.bytes, 0);
    assert_eq!(result.files, 0);
}

#[test]
fn dir_size_counts_a_single_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("solo.bin");
    write_file(&file, 4096);
    let result = sysfs::size::dir_size(&file);
    assert_eq!(result.bytes, 4096);
    assert_eq!(result.files, 1);
}

#[test]
fn dir_size_does_not_follow_junctions() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write_file(&root.join("a.bin"), 100);

    let junction = root.join("loop");
    let created = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !created {
        eprintln!("skipping: could not create a junction");
        return;
    }

    let result = sysfs::size::dir_size(root);
    assert_eq!(result.bytes, 100, "junction contents must not be counted");
    assert_eq!(result.skipped_reparse, 1);
}
