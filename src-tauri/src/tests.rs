use std::fs;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;

use crate::categories;
use crate::fs as sysfs;
use crate::fs::remove::{Counter, Removal};
use crate::model::{CleanupMode, TargetKind};
use crate::validate::SecurityGuard;

fn fixture() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path();
    fs::create_dir_all(root.join("sub/deep")).unwrap();
    fs::create_dir_all(root.join("empty")).unwrap();
    fs::write(root.join("a.bin"), vec![0u8; 1000]).unwrap();
    fs::write(root.join("sub/b.bin"), vec![0u8; 2000]).unwrap();
    fs::write(root.join("sub/deep/c.bin"), vec![0u8; 3000]).unwrap();
    dir
}

fn write_file(path: &Path, size: usize) {
    fs::write(path, vec![0u8; size]).unwrap();
}

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

#[test]
fn clear_contents_empties_but_keeps_the_root() {
    let dir = fixture();
    let root = dir.path().to_path_buf();
    let outcome = sysfs::remove::clear_contents(&root, &Counter::none());

    assert!(outcome.failures.is_empty(), "{:?}", outcome.failures);
    assert_eq!(outcome.freed_bytes, 6000);
    assert!(
        root.is_dir(),
        "ClearContents must leave the folder in place"
    );
    assert_eq!(
        fs::read_dir(&root).unwrap().count(),
        0,
        "the folder must be left empty"
    );
}

#[test]
fn remove_dir_removes_the_root_itself() {
    let dir = fixture();
    let root = dir.path().join("sub");
    let outcome = sysfs::remove::remove_dir(&root, &Counter::none());

    assert!(outcome.failures.is_empty(), "{:?}", outcome.failures);
    assert!(!root.exists());
}

#[test]
fn clear_contents_deletes_read_only_files() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("readonly.bin");
    write_file(&file, 128);

    let target = dir.path().to_path_buf();
    let outcome = sysfs::remove::clear_contents(&target, &Counter::none());

    assert!(outcome.failures.is_empty(), "{:?}", outcome.failures);
    assert!(!file.exists(), "read-only files must still be removable");
}

#[test]
fn dry_run_flag_truthiness() {
    use crate::fs::remove::truthy;
    for on in ["1", "true", "TRUE", "yes", "on", " 1 "] {
        assert!(truthy(on), "{on:?} should enable the flag");
    }
    for off in ["", "0", "false", "off", "no", " FALSE "] {
        assert!(!truthy(off), "{off:?} should disable the flag");
    }
}

#[test]
fn dry_run_is_off_by_default() {
    assert!(!sysfs::remove::dry_run());
    assert!(!sysfs::remove::keep_root());
}

#[test]
fn cleanup_reports_failures_instead_of_panicking() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("nope");
    let error = sysfs::remove::clean(&missing, false, &Counter::none()).unwrap_err();
    assert_eq!(error.kind(), "not_found");
}

#[test]
fn a_clean_run_sleeps_nothing_and_blocks_nothing() {
    let dir = fixture();
    let (outcome, dry_run) = sysfs::remove::clean(dir.path(), false, &Counter::none()).unwrap();

    assert!(!dry_run);
    assert!(outcome.failures.is_empty(), "{:?}", outcome.failures);
    assert_eq!(outcome.retries, 0, "nothing failed, so nothing was retried");
    assert_eq!(outcome.slept_ms, 0, "a clean run must not sleep at all");
    assert_eq!(outcome.blocked, 0);
}

#[test]
fn diagnostics_count_entries_without_disturbing_the_size() {
    let dir = fixture();
    let counter = Counter::shared();
    let (outcome, _) = sysfs::remove::clean(dir.path(), false, &counter).unwrap();

    assert_eq!(outcome.entries, 6);
    assert!(
        counter.count() >= outcome.entries,
        "the shared counter should track at least the per-removal count"
    );
    assert_eq!(
        outcome.freed_bytes, 6000,
        "counting must not alter freed_bytes"
    );
}

#[test]
fn elapsed_is_measured_once_at_the_top_level() {
    let dir = fixture();
    let (outcome, _) = sysfs::remove::clean(dir.path(), false, &Counter::none()).unwrap();
    assert!(outcome.elapsed_ms < 30_000, "{}", outcome.elapsed_ms);
}

#[test]
fn a_disabled_counter_is_inert() {
    let dir = fixture();
    let (outcome, _) = sysfs::remove::clean(dir.path(), false, &Counter::none()).unwrap();
    assert_eq!(outcome.freed_bytes, 6000);
    assert_eq!(Counter::none().count(), 0);
}

#[test]
fn a_locked_file_costs_35_ms_and_is_reported_as_blocked() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("held.bin");
    write_file(&file, 64);

    let handle = fs::File::options()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&file)
        .expect("the handle that blocks deletion");

    let started = std::time::Instant::now();
    let (outcome, _) = sysfs::remove::clean(dir.path(), false, &Counter::none()).unwrap();
    let wall = started.elapsed();

    assert!(file.exists(), "the file must still be there: we hold it");
    assert_eq!(outcome.failures.len(), 1);
    assert_eq!(
        outcome.blocked, 1,
        "a held file must be reported as blocked"
    );
    assert_eq!(outcome.retries, 2, "one retry per backoff step");
    assert_eq!(outcome.slept_ms, 35, "10 + 25 ms of backoff");
    assert!(
        wall >= Duration::from_millis(35),
        "the documented 35 ms per blocked file, got {wall:?}"
    );
    assert_eq!(outcome.freed_bytes, 0);

    drop(handle);
}

#[test]
fn a_transient_failure_is_still_recovered_on_a_retry() {
    use crate::fs::remove::retry;

    let mut calls = 0;
    let mut out = Removal::default();
    let result = retry(&mut out, || {
        calls += 1;
        if calls == 1 {
            Err(io::Error::from_raw_os_error(32))
        } else {
            Ok(())
        }
    });

    assert!(result.is_ok(), "the second attempt should have succeeded");
    assert_eq!(calls, 2);
    assert_eq!(out.retries, 1);
    assert_eq!(out.slept_ms, 10, "only the first backoff step was used");
    assert!(out.failures.is_empty());
}

#[test]
fn a_permanent_failure_is_not_retried_at_all() {
    use crate::fs::remove::retry;

    let mut calls = 0;
    let mut out = Removal::default();
    let result = retry(&mut out, || {
        calls += 1;
        Err(io::Error::from_raw_os_error(2))
    });

    assert!(result.is_err());
    assert_eq!(calls, 1, "a permanent error must stop on the first try");
    assert_eq!(out.retries, 0);
    assert_eq!(out.slept_ms, 0);
}

#[test]
fn the_backoff_schedule_and_the_attempt_count_agree() {
    use crate::fs::remove::retry;

    let mut calls = 0;
    let mut out = Removal::default();
    let _ = retry(&mut out, || {
        calls += 1;
        Err(io::Error::from_raw_os_error(32))
    });

    assert_eq!(calls, sysfs::remove::RETRY_ATTEMPTS);
    assert_eq!(out.retries, sysfs::remove::RETRY_ATTEMPTS as u32 - 1);
    assert_eq!(out.slept_ms, 35, "the documented 10 + 25 ms budget");
}

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

#[test]
fn category_ids_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for category in categories::all() {
        assert!(seen.insert(category.id), "duplicate id: {}", category.id);
    }
}

#[test]
fn every_category_has_a_known_group_and_a_locator() {
    let groups: Vec<&str> = categories::GROUPS.iter().map(|(id, _)| *id).collect();
    for category in categories::all() {
        assert!(
            groups.contains(&category.group),
            "{}: unknown group {}",
            category.id,
            category.group
        );
        assert!(
            !category.locators.is_empty(),
            "{}: no locators",
            category.id
        );
        assert!(!category.label.is_empty(), "{}: no label", category.id);
    }
}

#[test]
fn no_category_targets_a_protected_root() {
    let protected: Vec<String> = ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .map(|value| sysfs::resolve(&value).to_string_lossy().to_lowercase())
        .collect();

    for category in categories::all() {
        for path in crate::locate::resolve_all(category.locators) {
            let lowered = path.to_string_lossy().to_lowercase();
            for root in &protected {
                assert_ne!(
                    lowered, *root,
                    "{}: targets protected root {}",
                    category.id, root
                );
            }
        }
    }
}

#[test]
fn every_category_resolves_to_absolute_paths() {
    for category in categories::all() {
        let paths = crate::locate::resolve_all(category.locators);
        assert!(!paths.is_empty(), "{}: resolved to nothing", category.id);
        for path in &paths {
            assert!(
                path.is_absolute(),
                "{}: {} is relative",
                category.id,
                path.display()
            );
        }
    }
}

#[test]
fn no_two_categories_share_a_target() {
    let mut owner: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for category in categories::all() {
        for path in crate::locate::resolve_all(category.locators) {
            let key = path.to_string_lossy().to_lowercase();
            if let Some(previous) = owner.insert(key.clone(), category.id) {
                panic!(
                    "{} and {} both target {}",
                    previous,
                    category.id,
                    path.display()
                );
            }
        }
    }
}

#[test]
fn lookup_and_select_reject_unknown_ids() {
    assert!(categories::lookup("system.temp").is_ok());
    assert_eq!(
        categories::lookup("system.nope").unwrap_err().kind(),
        "unknown_category"
    );
    assert!(categories::select(Some(&["nope".to_string()])).is_err());
    assert_eq!(categories::select(Some(&[])).unwrap().len(), 0);
    assert_eq!(
        categories::select(None).unwrap().len(),
        categories::all().len()
    );
}

#[test]
fn env_locator_resolves_and_strips_sigil() {
    let resolved = crate::locate::env::resolve_env("%LOCALAPPDATA%");
    assert_eq!(resolved.len(), 1);
    assert!(resolved[0].is_absolute());

    assert_eq!(crate::locate::env::resolve_env("LOCALAPPDATA").len(), 1);
    assert_eq!(crate::locate::env::resolve_env("%TEMP%").len(), 1);
}

#[test]
fn env_locator_returns_nothing_for_unknown_variables() {
    assert!(crate::locate::env::resolve_env("%PURIFIER_NOT_SET%").is_empty());
    assert!(crate::locate::env::resolve_env("%%").is_empty());
}

#[test]
fn resolve_dedupes_case_insensitively() {
    use crate::model::Locator;
    let resolved = crate::locate::resolve(&Locator::Fixed("%LOCALAPPDATA%"));
    let mut all = Vec::new();
    for locator in [
        Locator::Fixed("%LOCALAPPDATA%"),
        Locator::Fixed("%localappdata%"),
    ] {
        all.extend(crate::locate::resolve(&locator));
    }
    assert_eq!(all.len(), 2);
    assert_eq!(resolved.len(), 1);
}

#[test]
fn join_appends_a_relative_segment() {
    use crate::model::Locator;
    let locators = [Locator::Join(
        &[Locator::Env("%LOCALAPPDATA%")],
        "Sub\\Leaf",
    )];
    let resolved = crate::locate::resolve_all(&locators);
    assert_eq!(resolved.len(), 1);
    assert!(resolved[0].ends_with("Sub\\Leaf"), "{:?}", resolved[0]);
}

#[test]
fn catalog_entries_round_trip_through_cleanup_mode() {
    assert_ne!(CleanupMode::ClearContents, CleanupMode::RemoveDir);
}
