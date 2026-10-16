use std::path::PathBuf;
use std::time::Instant;

use tauri::AppHandle;

use crate::cleanup::categories;
use crate::cleanup::locate;
use crate::cleanup::model::{CategoryDef, Phase, ScanEntry};
use crate::common::error::{AppError, Result};

use super::report::{elapsed_text, emit_status, human_bytes, measure_and_emit};

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

        let targets = measure_and_emit(&app, category, &paths);

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
