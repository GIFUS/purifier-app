use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::error::Result;

const LABEL: &str = "main";

static READY: AtomicBool = AtomicBool::new(false);

const FALLBACK: Duration = Duration::from_millis(4000);

pub fn show(app: &AppHandle) {
    READY.store(true, Ordering::Relaxed);
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.show();
    }
}

pub fn arm_fallback(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(FALLBACK);
        if READY.load(Ordering::Relaxed) {
            return;
        }
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.show();
        }
    });
}

#[tauri::command]
pub fn app_ready(app: AppHandle) -> Result<()> {
    show(&app);
    Ok(())
}
