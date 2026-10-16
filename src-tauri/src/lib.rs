mod app;
mod autorun;
mod cleanup;
mod common;

#[cfg(test)]
mod tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            app::info::app_info,
            autorun::list_autorun,
            autorun::list_autorun_tasks,
            autorun::set_autorun_enabled,
            autorun::set_autorun_task_enabled,
            cleanup::catalog::list_categories,
            cleanup::catalog::list_groups,
            cleanup::scan::scan_categories,
            cleanup::clean::clean_categories,
            app::reveal::open_in_explorer,
            app::reveal::open_repository,
            app::window::app_ready,
        ])
        .setup(|app| {
            app::window::arm_fallback(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running purifier");
}
