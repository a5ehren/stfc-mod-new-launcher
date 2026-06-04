mod app_state;
mod commands;
mod diagnostics;
pub mod errors;
pub mod events;
mod game_locator;
mod github_releases;
mod mod_manager;
pub mod models;
mod storage;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let app_state = app_state::AppState::new()?;
            app.manage(app_state);
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_launcher_status,
            commands::validate_game_path,
            commands::set_mod_channel,
            commands::open_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
