use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::common::error::{AppError, Result};

const REPOSITORY_URL: &str = "https://github.com/GIFUS/purifier-app";

#[tauri::command]
pub fn open_in_explorer(app: AppHandle, path: String) -> Result<()> {
    let trimmed = path.trim().trim_matches('"').to_string();
    if trimmed.is_empty() {
        return Err(AppError::Other("path is empty".to_string()));
    }
    app.opener()
        .open_path(trimmed, None::<&str>)
        .map_err(|e| AppError::Other(format!("cannot open explorer: {e}")))
}

// Takes no url argument on purpose: a parameter would let the webview ask the
// app to open arbitrary locations, including local paths.
#[tauri::command]
pub fn open_repository(app: AppHandle) -> Result<()> {
    app.opener()
        .open_url(REPOSITORY_URL, None::<&str>)
        .map_err(|e| AppError::Other(format!("cannot open the repository: {e}")))
}
