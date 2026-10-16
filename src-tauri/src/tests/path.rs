use std::path::{Path, PathBuf};

use crate::common as sysfs;

use super::fixture;

#[test]
fn expand_env_substitutes_known_variables() {
    let expected = std::env::var("TEMP").expect("TEMP is set on Windows");
    let expanded = sysfs::path::expand_env("%TEMP%");
    assert_eq!(
        expanded.trim_end_matches('\\'),
        expected.trim_end_matches('\\')
    );
    assert_eq!(
        sysfs::path::expand_env("x\\%TEMP%\\y"),
        format!("x\\{expanded}\\y")
    );
}

#[test]
fn expand_env_leaves_unknown_variables_alone() {
    assert_eq!(
        sysfs::path::expand_env("%PURIFIER_NOT_A_REAL_VAR%"),
        "%PURIFIER_NOT_A_REAL_VAR%"
    );
    assert_eq!(sysfs::path::expand_env("100%"), "100%");
}

#[test]
fn normalize_collapses_redundant_segments() {
    assert_eq!(
        sysfs::path::normalize("C:\\Windows\\Temp\\..\\Temp"),
        PathBuf::from("C:\\Windows\\Temp")
    );
    assert_eq!(
        sysfs::path::normalize("C:/Windows//Temp/./"),
        PathBuf::from("C:\\Windows\\Temp")
    );
}

#[test]
fn resolve_returns_a_path_without_the_verbatim_prefix() {
    let dir = fixture();
    let resolved = sysfs::resolve(&dir.path().to_string_lossy());
    assert!(
        !resolved.to_string_lossy().starts_with("\\\\?\\"),
        "resolve leaked a verbatim prefix: {resolved:?}"
    );
}

#[test]
fn resolve_is_idempotent() {
    let dir = fixture();
    let once = sysfs::resolve(&dir.path().to_string_lossy());
    let twice = sysfs::resolve(&once.to_string_lossy());
    assert_eq!(once, twice, "resolving a resolved path changed it");
}

#[test]
fn strip_verbatim_unwraps_drive_and_unc_paths() {
    assert_eq!(
        sysfs::path::strip_verbatim(Path::new("\\\\?\\C:\\a\\b")),
        PathBuf::from("C:\\a\\b")
    );
    assert_eq!(
        sysfs::path::strip_verbatim(Path::new("\\\\?\\UNC\\srv\\share\\a")),
        PathBuf::from("\\\\srv\\share\\a")
    );
    assert_eq!(
        sysfs::path::strip_verbatim(Path::new("C:\\a")),
        PathBuf::from("C:\\a")
    );
}

#[test]
fn verbatim_round_trips_with_strip_verbatim() {
    for input in ["C:\\a\\b", "\\\\srv\\share\\a"] {
        let plain = PathBuf::from(input);
        assert_eq!(
            sysfs::path::strip_verbatim(&sysfs::path::verbatim(&plain)),
            plain
        );
    }

    let already = PathBuf::from("\\\\?\\C:\\a");
    assert_eq!(
        sysfs::path::verbatim(&already),
        already,
        "must not double-prefix"
    );

    let relative = PathBuf::from("\\foo");
    assert_eq!(sysfs::path::verbatim(&relative), relative);
}

#[test]
fn is_within_treats_verbatim_and_plain_paths_as_the_same() {
    assert!(sysfs::is_within(
        Path::new("\\\\?\\C:\\Windows\\Temp"),
        Path::new("C:\\Windows")
    ));
    assert!(sysfs::is_within(
        Path::new("C:\\Windows\\Temp"),
        Path::new("\\\\?\\C:\\Windows")
    ));
    assert!(!sysfs::is_within(
        Path::new("\\\\?\\C:\\Windows\\Temp"),
        Path::new("C:\\Users")
    ));
}

#[test]
fn is_within_is_component_based_not_string_based() {
    assert!(!sysfs::is_within(
        Path::new("C:\\Windows2"),
        Path::new("C:\\Windows")
    ));
    assert!(sysfs::is_within(
        Path::new("C:\\Windows\\Temp"),
        Path::new("C:\\Windows")
    ));
    assert!(sysfs::is_within(
        Path::new("C:\\Windows"),
        Path::new("C:\\Windows")
    ));
}

#[test]
fn is_within_rejects_a_root_only_base() {
    assert!(!sysfs::is_within(
        Path::new("C:\\Windows\\Temp"),
        Path::new("C:\\")
    ));
    assert!(!sysfs::is_within(Path::new("C:\\Temp"), Path::new("")));
}

#[test]
fn is_volume_root_detects_bare_roots() {
    assert!(sysfs::is_volume_root(Path::new("C:\\")));
    assert!(sysfs::is_volume_root(Path::new("D:/")));
    assert!(!sysfs::is_volume_root(Path::new("C:\\Windows")));
    assert!(!sysfs::is_volume_root(Path::new("C:\\Windows\\Temp")));
}
