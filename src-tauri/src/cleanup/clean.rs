use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rayon::prelude::*;
use tauri::AppHandle;

use crate::cleanup::categories;
use crate::cleanup::fs::remove::{self, Counter, Removal};
use crate::cleanup::locate;
use crate::cleanup::model::{CategoryDef, CleanReport, CleanupMode, Failure, Phase};
use crate::cleanup::validate::SecurityGuard;
use crate::common::error::{AppError, Result};

use super::report::{emit_status, emit_status_full, format_number, human_bytes, measure_and_emit};

const TICK: Duration = Duration::from_millis(150);

#[tauri::command]
pub async fn clean_categories(
    app: AppHandle,
    ids: Vec<String>,
    mode: Option<CleanupMode>,
) -> Result<CleanReport> {
    if ids.is_empty() {
        return Ok(CleanReport::default());
    }

    let selected = categories::select(Some(&ids))?;
    let guard = SecurityGuard::for_categories(&ids)?;

    let rehearsal = remove::dry_run();

    let handle =
        tauri::async_runtime::spawn_blocking(move || run(app, selected, guard, mode, rehearsal));
    handle
        .await
        .map_err(|e| AppError::Other(format!("clean task failed: {e}")))?
}

struct JobOutcome {
    category_id: &'static str,
    path: String,
    removal: Removal,
}

fn run(
    app: AppHandle,
    selected: Vec<&'static CategoryDef>,
    guard: SecurityGuard,
    mode: Option<CleanupMode>,
    rehearsal: bool,
) -> Result<CleanReport> {
    let started = Instant::now();
    let mut report = CleanReport::default();

    let jobs: Vec<(usize, PathBuf)> = selected
        .iter()
        .enumerate()
        .flat_map(|(index, category)| {
            locate::resolve_all(category.locators)
                .into_iter()
                .map(move |path| (index, path))
        })
        .collect();
    let category_total = selected.len();

    emit_status(
        &app,
        Phase::Cleaning,
        0,
        category_total,
        "starting".to_string(),
    );

    let counter = Counter::shared();
    let finished = Arc::new(AtomicBool::new(false));

    let ticker = {
        let app = app.clone();
        let counter = counter.clone();
        let finished = finished.clone();
        let label = match jobs.as_slice() {
            [] => String::new(),
            [(_, path)] => path.display().to_string(),
            many => format!("{} folders", many.len()),
        };

        thread::spawn(move || {
            while !finished.load(Ordering::Relaxed) {
                thread::sleep(TICK);
                if finished.load(Ordering::Relaxed) {
                    break;
                }
                let entries = counter.count();
                emit_status_full(
                    &app,
                    Phase::Cleaning,
                    entries as usize,
                    0,
                    true,
                    format!("{label} · {entries} entries"),
                );
            }
        })
    };

    let outcomes: Vec<JobOutcome> = jobs
        .par_iter()
        .map(|(index, path)| process(&selected, &guard, *index, path, mode, &counter))
        .collect();

    finished.store(true, Ordering::Relaxed);
    let _ = ticker.join();

    for outcome in &outcomes {
        report.freed_bytes = report
            .freed_bytes
            .saturating_add(outcome.removal.freed_bytes);
        report.files_removed = report
            .files_removed
            .saturating_add(outcome.removal.files_removed);
        report.blocked = report.blocked.saturating_add(outcome.removal.blocked);
        report
            .failed
            .extend(outcome.removal.failures.iter().cloned());

        eprintln!(
            "[clean] {:<28} {:<52} entries={} files={} retries={} slept={}ms blocked={} freed={} in {}ms",
            outcome.category_id,
            outcome.path,
            outcome.removal.entries,
            outcome.removal.files_removed,
            outcome.removal.retries,
            outcome.removal.slept_ms,
            outcome.removal.blocked,
            human_bytes(outcome.removal.freed_bytes),
            outcome.removal.elapsed_ms,
        );
    }

    remeasure(&app, &selected);

    let verb = if rehearsal { "Would free" } else { "Freed" };
    emit_status(
        &app,
        Phase::Done,
        category_total,
        category_total,
        done_message(verb, &report, started.elapsed()),
    );

    Ok(report)
}

fn done_message(verb: &str, report: &CleanReport, elapsed: Duration) -> String {
    let blocked = report.blocked.min(report.failed.len() as u64);
    let other_failed = (report.failed.len() as u64).saturating_sub(blocked);

    let mut message = format!(
        "{verb} {} · {}",
        human_bytes(report.freed_bytes),
        files(report.files_removed)
    );
    if blocked > 0 {
        message.push_str(&format!(" · {} blocked", format_number(blocked)));
    }
    if other_failed > 0 {
        message.push_str(&format!(" · {other_failed} failed"));
    }
    message.push_str(&format!(" · {:.1}s", elapsed.as_secs_f64()));
    message
}

fn files(count: u64) -> String {
    format!(
        "{} {}",
        format_number(count),
        if count == 1 { "file" } else { "files" }
    )
}

fn remeasure(app: &AppHandle, selected: &[&'static CategoryDef]) {
    let category_total = selected.len();

    for (done, category) in selected.iter().enumerate() {
        let paths = locate::resolve_all(category.locators);
        emit_status_full(
            app,
            Phase::Cleaning,
            done,
            category_total.max(1),
            true,
            format!("{} · measuring", category.label),
        );

        measure_and_emit(app, category, &paths);
    }
}

fn process(
    selected: &[&'static CategoryDef],
    guard: &SecurityGuard,
    category_index: usize,
    path: &Path,
    mode: Option<CleanupMode>,
    counter: &Counter,
) -> JobOutcome {
    let category = selected[category_index];
    let raw = path.to_string_lossy().into_owned();

    let checked = match guard.check(category.id, &raw) {
        Ok(p) => p,
        Err(err) => {
            return JobOutcome {
                category_id: category.id,
                path: raw.clone(),
                removal: Removal {
                    failures: vec![Failure {
                        path: raw,
                        reason: err.to_string(),
                    }],
                    ..Removal::default()
                },
            }
        }
    };

    let remove_root = match mode {
        Some(m) => m == CleanupMode::RemoveDir,
        None => category.mode == CleanupMode::RemoveDir,
    };

    match remove::clean(&checked, remove_root, counter) {
        Ok((removal, _)) => JobOutcome {
            category_id: category.id,
            path: checked.display().to_string(),
            removal,
        },
        Err(err) => JobOutcome {
            category_id: category.id,
            path: checked.display().to_string(),
            removal: Removal {
                failures: vec![Failure {
                    path: checked.display().to_string(),
                    reason: err.to_string(),
                }],
                ..Removal::default()
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleanup::fs::remove::Counter;

    fn report(blocked: u64, failures: usize) -> CleanReport {
        CleanReport {
            blocked,
            failed: (0..failures)
                .map(|i| Failure {
                    path: format!("C:\\p{i}"),
                    reason: "held".to_string(),
                })
                .collect(),
            ..CleanReport::default()
        }
    }

    #[test]
    fn a_guard_rejection_is_not_counted_as_blocked() {
        let dir = tempfile::TempDir::new().unwrap();
        let declared = dir.path().join("allowed");
        let foreign = dir.path().join("foreign");
        std::fs::create_dir_all(&declared).unwrap();
        std::fs::create_dir_all(&foreign).unwrap();

        let guard = SecurityGuard::from_targets(vec![declared]);

        let outcome = process(
            &[crate::cleanup::categories::lookup("system.temp").unwrap()],
            &guard,
            0,
            &foreign,
            None,
            &Counter::none(),
        );

        assert_eq!(outcome.removal.failures.len(), 1);
        assert_eq!(outcome.removal.blocked, 0, "a safety refusal is not a lock");
        assert_eq!(outcome.removal.freed_bytes, 0);
        assert!(foreign.exists(), "nothing may be removed on a refusal");
    }

    #[test]
    fn the_finish_line_reports_blocked_and_failed_disjointly() {
        let line = done_message("Freed", &report(2, 3), Duration::from_millis(2500));
        assert_eq!(line, "Freed 0 B · 0 files · 2 blocked · 1 failed · 2.5s");
    }

    #[test]
    fn the_finish_line_omits_zero_segments_and_agrees_in_singular() {
        assert_eq!(
            done_message("Freed", &report(0, 0), Duration::from_millis(100)),
            "Freed 0 B · 0 files · 0.1s"
        );

        let mut one = report(0, 0);
        one.files_removed = 1;
        one.freed_bytes = 1500;
        assert_eq!(
            done_message("Freed", &one, Duration::from_millis(10)),
            "Freed 1.46 KB · 1 file · 0.0s"
        );
    }

    #[test]
    fn blocked_can_never_exceed_failures() {
        for failures in 0..6u64 {
            let line = done_message(
                "Freed",
                &report(failures + 3, failures as usize),
                Duration::from_millis(100),
            );
            assert!(
                !line.contains("failed"),
                "{line} reported failures for {failures} blocked paths"
            );
        }
    }

    #[test]
    fn the_rehearsal_verb_reaches_the_line() {
        let line = done_message("Would free", &report(0, 0), Duration::from_millis(100));
        assert!(line.starts_with("Would free"), "{line}");
    }
}
