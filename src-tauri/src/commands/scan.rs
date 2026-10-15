use std::path::PathBuf;
use std::time::{Duration, Instant};

use rayon::prelude::*;
use tauri::{AppHandle, Emitter};

use crate::categories;
use crate::error::{AppError, Result};
use crate::fs::inspect;
use crate::locate;
use crate::model::{CategoryDef, Phase, ScanEntry, ScanProgress, Status, TargetInfo};

use super::events;

#[tauri::command]
pub async fn scan_categories(
    app: AppHandle,
    ids: Option<Vec<String>>,
    quiet: Option<bool>,
) -> Result<Vec<ScanEntry>> {
    let selected = categories::select(ids.as_deref())?;
    let total = selected.len();
    let quiet = quiet.unwrap_or(false);

    let handle = tauri::async_runtime::spawn_blocking(move || run(app, selected, total, quiet));
    handle
        .await
        .map_err(|e| AppError::Other(format!("scan task failed: {e}")))?
}

fn run(
    app: AppHandle,
    selected: Vec<&'static CategoryDef>,
    total: usize,
    quiet: bool,
) -> Result<Vec<ScanEntry>> {
    let started = Instant::now();
    let mut entries = Vec::with_capacity(selected.len());

    if !quiet {
        emit_status(
            &app,
            Phase::Scanning,
            0,
            total,
            "Resolving folders".to_string(),
        );
    }

    for (index, category) in selected.into_iter().enumerate() {
        let paths: Vec<PathBuf> = locate::resolve_all(category.locators);
        if !quiet {
            emit_status(
                &app,
                Phase::Scanning,
                index,
                total,
                category.label.to_string(),
            );
        }

        let count = paths.len();
        let targets = measure_all(&paths);
        for (position, target) in targets.iter().enumerate() {
            let _ = app.emit(
                events::TARGET,
                ScanProgress {
                    category_id: category.id,
                    category_label: category.label,
                    group: category.group,
                    index: position,
                    total: count,
                    target: target.clone(),
                },
            );
        }

        entries.push(ScanEntry {
            category: *category,
            targets,
            error: None,
        });
    }

    let scanned: u64 = entries
        .iter()
        .flat_map(|e| e.targets.iter())
        .map(|t| t.size_bytes)
        .sum();
    if !quiet {
        emit_status(
            &app,
            Phase::Done,
            total,
            total,
            format!(
                "{} in {}",
                human_bytes(scanned),
                elapsed_text(started.elapsed())
            ),
        );
    }

    Ok(entries)
}

fn measure_all(paths: &[PathBuf]) -> Vec<TargetInfo> {
    if paths.len() <= 1 {
        return paths
            .iter()
            .map(|p| inspect::probe(&p.to_string_lossy()))
            .collect();
    }
    paths
        .par_iter()
        .map(|p| inspect::probe(&p.to_string_lossy()))
        .collect()
}

pub(super) fn emit_status(
    app: &AppHandle,
    phase: Phase,
    done: usize,
    total: usize,
    message: String,
) {
    emit_status_full(app, phase, done, total, false, message);
}

pub(super) fn emit_status_full(
    app: &AppHandle,
    phase: Phase,
    done: usize,
    total: usize,
    indeterminate: bool,
    message: String,
) {
    let _ = app.emit(
        events::STATUS,
        Status {
            phase,
            done,
            total,
            indeterminate,
            message,
        },
    );
}

pub(super) fn format_number(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

fn elapsed_text(elapsed: Duration) -> String {
    let millis = elapsed.as_millis();
    if millis < 1_000 {
        format!("{millis}ms")
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

pub(super) fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_runs_report_whole_milliseconds() {
        assert_eq!(elapsed_text(Duration::ZERO), "0ms");
        assert_eq!(elapsed_text(Duration::from_micros(11_565)), "11ms");
        assert_eq!(elapsed_text(Duration::from_millis(999)), "999ms");
    }

    #[test]
    fn a_second_and_over_reports_tenths() {
        assert_eq!(elapsed_text(Duration::from_millis(1_000)), "1.0s");
        assert_eq!(elapsed_text(Duration::from_millis(2_500)), "2.5s");
        assert_eq!(elapsed_text(Duration::from_millis(61_000)), "61.0s");
    }
}
