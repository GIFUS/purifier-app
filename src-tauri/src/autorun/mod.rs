//! Autostart entries: the Run keys, the two Startup folders, and the sign-in tasks.
//! `scope` and `source` are matched by the frontend, so their values are the
//! serialized shape.

mod paths;
mod registry;
mod tasks;

use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use crate::common::error::{AppError, Result};
use paths::{startup_extension, target_exists};
use registry::{
    approved_states, decode, is_enabled, listed_sources, open_run_key, values_of, write_approved,
    REGISTRY_SOURCES,
};

const STARTUP_USER: &str = "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Startup";
const STARTUP_MACHINE: &str = "%PROGRAMDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Startup";

const SCOPE_USER: &str = "user";
const SCOPE_MACHINE: &str = "machine";

const SOURCE_RUN: &str = "run";
const SOURCE_RUNONCE: &str = "run_once";
const SOURCE_POLICY_RUN: &str = "policy_run";
const SOURCE_RUN32: &str = "run32";
const SOURCE_FOLDER: &str = "startup_folder";

#[derive(Debug, Clone, Serialize)]
pub struct AutorunEntry {
    pub source: String,
    pub scope: String,
    pub path: String,
    pub name: String,
    pub command: String,
    pub enabled: bool,
    pub exists: bool,
}

impl AutorunEntry {
    fn new(
        source: &str,
        scope: &str,
        path: &str,
        name: String,
        command: String,
        enabled: bool,
    ) -> Self {
        Self {
            source: source.to_string(),
            scope: scope.to_string(),
            path: path.to_string(),
            exists: target_exists(&command),
            name,
            command,
            enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AutorunTask {
    pub name: String,
    pub path: String,
    pub command: String,
    pub enabled: bool,
    pub exists: bool,
    pub scope: String,
}

#[tauri::command]
pub fn list_autorun() -> Result<Vec<AutorunEntry>> {
    let mut entries = Vec::new();

    for source in listed_sources() {
        let approved = approved_states(source.machine, source.approved_cell);
        let Ok(run) = open_run_key(source) else {
            continue;
        };

        for (name, value) in values_of(&run) {
            let Some(command) = decode(&value) else {
                continue;
            };
            let enabled = is_enabled(&approved, &name);
            entries.push(AutorunEntry::new(
                source.source,
                source.scope,
                source.run_key,
                name,
                command,
                enabled,
            ));
        }
    }

    for (scope, dir, cell_hive) in folder_sources() {
        let approved = approved_states(cell_hive, "StartupFolder");
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };

        let mut found: Vec<(String, String)> = read
            .flatten()
            .map(|item| item.path())
            .filter(|path| path.is_file())
            .filter(|path| startup_extension(path))
            .filter_map(|path| {
                let name = path.file_name()?.to_str()?.to_string();
                Some((name, path.to_string_lossy().to_string()))
            })
            .collect();
        found.sort_by_key(|entry| entry.0.to_lowercase());

        for (name, command) in found {
            let enabled = is_enabled(&approved, &name);
            entries.push(AutorunEntry::new(
                SOURCE_FOLDER,
                scope,
                &dir.to_string_lossy(),
                name,
                command,
                enabled,
            ));
        }
    }

    entries.sort_by(|a, b| {
        a.scope
            .cmp(&b.scope)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
pub fn set_autorun_enabled(
    source: String,
    scope: String,
    path: String,
    name: String,
    enabled: bool,
) -> Result<()> {
    if name.trim().is_empty() {
        return Err(AppError::Other("autostart entry has no name".to_string()));
    }

    match source.as_str() {
        SOURCE_RUN | SOURCE_RUNONCE | SOURCE_POLICY_RUN | SOURCE_RUN32 => {
            // Two RunOnce keys share a source and scope, so the path tells them apart.
            let Some(entry) = REGISTRY_SOURCES.iter().find(|s| {
                s.source == source && s.scope == scope && s.run_key.eq_ignore_ascii_case(&path)
            }) else {
                return Err(AppError::Other(format!(
                    "unknown autostart location: {path}"
                )));
            };
            if !entry.toggleable {
                return Err(AppError::Other(format!(
                    "cannot switch {}: {}",
                    entry.name(),
                    entry.reason
                )));
            }
            write_approved(entry.machine, entry.approved_cell, &name, enabled)
        }
        SOURCE_FOLDER => {
            let cell_hive = folder_hive(&scope)
                .ok_or_else(|| AppError::Other(format!("unknown autostart scope: {scope}")))?;
            write_approved(cell_hive, "StartupFolder", &name, enabled)
        }
        other => Err(AppError::Other(format!(
            "unknown autostart source: {other}"
        ))),
    }
}

#[tauri::command]
pub fn list_autorun_tasks() -> Result<Vec<AutorunTask>> {
    let mut tasks = tasks::collect();
    tasks.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(tasks)
}

#[tauri::command]
pub fn set_autorun_task_enabled(path: String, name: String, enabled: bool) -> Result<()> {
    tasks::set_enabled(&path, &name, enabled)
}

fn folder_sources() -> Vec<(&'static str, PathBuf, bool)> {
    vec![
        (SCOPE_USER, crate::common::resolve(STARTUP_USER), false),
        (SCOPE_MACHINE, crate::common::resolve(STARTUP_MACHINE), true),
    ]
}

/// The common Startup folder keeps its state under HKLM and the per-user one
/// under HKCU; a blob in the wrong hive leaves the item running.
fn folder_hive(scope: &str) -> Option<bool> {
    folder_sources()
        .into_iter()
        .find(|(name, _, _)| *name == scope)
        .map(|(_, _, machine)| machine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_startup_folder_scope_keeps_its_state_in_its_own_hive() {
        assert_eq!(folder_hive(SCOPE_USER), Some(false));
        assert_eq!(folder_hive(SCOPE_MACHINE), Some(true));
        assert_eq!(folder_hive("something_else"), None);
    }
}
