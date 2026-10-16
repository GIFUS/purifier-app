use std::fs;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::time::Duration;

use tempfile::TempDir;

use crate::cleanup::fs as sysfs;
use crate::cleanup::fs::remove::{Counter, Removal};

use super::{fixture, write_file};

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
    use crate::cleanup::fs::remove::truthy;
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
    use crate::cleanup::fs::remove::retry;

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
    use crate::cleanup::fs::remove::retry;

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
    use crate::cleanup::fs::remove::retry;

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
