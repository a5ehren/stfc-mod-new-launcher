mod app_state;
mod commands;
mod config_service;
mod diagnostics;
pub mod errors;
pub mod events;
mod game_locator;
mod game_updater;
mod github_releases;
mod instance_backup;
mod instance_launch;
mod instance_users;
mod launch;
mod migration;
mod mod_manager;
pub mod models;
mod provisioning;
mod rsync_patch;
mod self_update;
mod storage;
#[cfg(target_os = "windows")]
mod windows_targets;
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
            commands::set_game_path,
            commands::open_logs,
            commands::get_windows_legacy_cleanup_plan,
            commands::apply_managed_migration,
            commands::read_raw_config,
            commands::save_raw_config,
            commands::open_raw_config,
            commands::launch_game,
            commands::open_config_editor,
            commands::update_game,
            commands::update_mod,
            commands::check_mod_update,
            commands::check_game_update,
            commands::install_launcher_update,
            commands::check_launcher_update,
            commands::mi_wizard_plan,
            commands::mi_provision,
            commands::mi_set_enabled,
            commands::mi_start_instance,
            commands::mi_stop_instance,
            commands::mi_instance_status,
            commands::mi_backup_instance,
            commands::mi_restore_instance,
            commands::mi_remove_instance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
