mod categories;
mod commands;
mod error;
mod fs;
mod locate;
mod model;
mod validate;

#[cfg(test)]
mod tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::autorun::list_autorun,
            commands::autorun::list_autorun_tasks,
            commands::autorun::set_autorun_enabled,
            commands::autorun::set_autorun_task_enabled,
            commands::catalog::list_categories,
            commands::catalog::list_groups,
            commands::scan::scan_categories,
            commands::clean::clean_categories,
            commands::reveal::open_in_explorer,
            commands::reveal::open_repository,
            commands::window::app_ready,
        ])
        .setup(|app| {
            commands::window::arm_fallback(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running purifier");
}
