//! Progress reporting shared by `scan_categories` and `clean_categories`: the
//! status stream, the per-target stream, and the text both commands print.
//! Cleaning used to import these from scanning, which made the two commands
//! depend on each other for something neither owns.

use std::path::PathBuf;
use std::time::Duration;

use rayon::prelude::*;
use tauri::{AppHandle, Emitter};

use crate::cleanup::fs::inspect;
use crate::cleanup::model::{CategoryDef, Phase, ScanProgress, Status, TargetInfo};

use super::events;

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

/// Measures every path and announces each one as it is reported, so rows appear
/// per category instead of only when the whole pass returns.
pub(super) fn measure_and_emit(
    app: &AppHandle,
    category: &CategoryDef,
    paths: &[PathBuf],
) -> Vec<TargetInfo> {
    let targets = measure(paths);
    for (index, target) in targets.iter().enumerate() {
        let _ = app.emit(
            events::TARGET,
            ScanProgress {
                category_id: category.id,
                category_label: category.label,
                group: category.group,
                index,
                total: targets.len(),
                target: target.clone(),
            },
        );
    }
    targets
}

fn measure(paths: &[PathBuf]) -> Vec<TargetInfo> {
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

pub(super) fn elapsed_text(elapsed: Duration) -> String {
    let millis = elapsed.as_millis();
    if millis < 1_000 {
        format!("{millis}ms")
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
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
