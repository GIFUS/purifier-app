use std::fs;

use tempfile::TempDir;

use crate::cleanup::fs as sysfs;
use crate::cleanup::model::TargetKind;

use super::{fixture, write_file};

#[test]
fn probe_reports_a_displayable_path() {
    let dir = fixture();
    let info = sysfs::inspect::probe(&dir.path().to_string_lossy());
    assert!(
        !info.path.starts_with("\\\\?\\"),
        "TargetInfo.path is rendered and clicked: {}",
        info.path
    );
}

#[test]
fn probe_reports_size_and_kind_for_a_folder() {
    let dir = fixture();
    let info = sysfs::inspect::probe(&dir.path().to_string_lossy());
    assert!(info.exists);
    assert_eq!(info.kind, TargetKind::Dir);
    assert_eq!(info.size_bytes, 6000);
    assert_eq!(info.file_count, 3);
    assert!(!info.reparse);
}

#[test]
fn probe_reports_a_missing_folder_without_erroring() {
    let info = sysfs::inspect::probe("C:\\definitely\\not\\here");
    assert!(!info.exists);
    assert_eq!(info.size_bytes, 0);
    assert!(info.error.is_none());
}

#[test]
fn probe_never_traverses_a_junction() {
    let dir = TempDir::new().unwrap();
    let outer = dir.path().join("outer");
    let inner = dir.path().join("inner");
    fs::create_dir_all(&outer).unwrap();
    fs::create_dir_all(&inner).unwrap();
    write_file(&inner.join("payload.bin"), 50_000);

    let junction = outer.join("j");
    let created = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&inner)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !created {
        eprintln!("skipping: could not create a junction");
        return;
    }

    let info = sysfs::inspect::probe(&outer.to_string_lossy());
    assert!(info.exists);
    assert!(!info.reparse, "the probe target itself is a real folder");
    assert_eq!(info.size_bytes, 0, "the junction must not be traversed");
    assert_eq!(info.file_count, 0);
}
