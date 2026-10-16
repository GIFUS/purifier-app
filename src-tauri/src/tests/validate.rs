use std::path::PathBuf;

use crate::cleanup::validate::SecurityGuard;
use crate::common as sysfs;

use super::fixture;

#[test]
fn guard_accepts_a_declared_target() {
    let dir = fixture();
    let guard = SecurityGuard::from_targets(vec![dir.path().to_path_buf()]);
    let checked = guard
        .check("system.temp", &dir.path().to_string_lossy())
        .expect("declared target must pass");
    assert_eq!(checked, sysfs::resolve(&dir.path().to_string_lossy()));
}

#[test]
fn guard_rejects_an_unknown_category() {
    let dir = fixture();
    let guard = SecurityGuard::from_targets(vec![dir.path().to_path_buf()]);
    let error = guard
        .check("no.such.category", &dir.path().to_string_lossy())
        .unwrap_err();
    assert_eq!(error.kind(), "unknown_category");
}

#[test]
fn guard_rejects_a_path_outside_the_allowlist() {
    let dir = fixture();
    let guard = SecurityGuard::from_targets(vec![dir.path().join("sub")]);

    let inside = guard
        .check(
            "system.temp",
            &dir.path().join("sub/deep").to_string_lossy(),
        )
        .expect("a child of a declared target is fine");
    assert!(inside.ends_with("deep"));

    let error = guard
        .check("system.temp", &dir.path().to_string_lossy())
        .unwrap_err();
    assert_eq!(error.kind(), "unsafe");
}

#[test]
fn guard_never_accepts_the_parent_of_a_target() {
    let dir = fixture();
    let guard = SecurityGuard::from_targets(vec![dir.path().join("sub")]);
    let error = guard
        .check("system.temp", &dir.path().to_string_lossy())
        .unwrap_err();
    assert_eq!(error.kind(), "unsafe");
}

#[test]
fn guard_rejects_volume_roots_even_with_a_widened_allowlist() {
    let guard = SecurityGuard::from_targets(vec![PathBuf::from("C:\\Windows")]);
    assert_eq!(
        guard.check("system.temp", "C:\\").unwrap_err().kind(),
        "unsafe"
    );

    let guard = SecurityGuard::from_targets(vec![PathBuf::from("C:\\")]);
    assert_eq!(
        guard.check("system.temp", "C:\\").unwrap_err().kind(),
        "unsafe"
    );
}

#[test]
fn guard_rejects_a_protected_root_by_name() {
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let guard = SecurityGuard::from_targets(vec![PathBuf::from(&windir)]);
    let error = guard.check("system.temp", &windir).unwrap_err();
    assert_eq!(error.kind(), "unsafe");
}
