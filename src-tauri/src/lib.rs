mod app_state;
mod commands;
mod config_service;
mod diagnostics;
pub mod errors;
pub mod events;
mod game_locator;
mod game_updater;
mod github_releases;
mod launch;
mod migration;
mod mod_manager;
pub mod models;
mod rsync_patch;
mod self_update;
mod storage;
mod xsolla;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
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
            commands::open_logs,
            commands::get_windows_legacy_cleanup_plan,
            commands::apply_managed_migration,
            commands::read_raw_config,
            commands::save_raw_config,
            commands::open_raw_config,
            commands::launch_game,
            commands::check_launcher_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
