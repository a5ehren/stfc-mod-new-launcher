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
    // Linux/WebKitGTK: WebKitGTK 2.42+ composites via a DMA-BUF renderer that
    // allocates a GBM buffer the size of the window. On systems without a working
    // GPU/DRM/GBM stack (VMs, containers, headless/remote X, Xvfb, some Wayland+EGL
    // setups) `gbm_bo_create` fails with EINVAL and prints
    // "Failed to create GBM buffer of size <W>x<H>: Invalid argument", leaving the
    // webview blank. Disable the DMA-BUF renderer and fall back to shared-memory
    // compositing. Only set when the user hasn't explicitly chosen, so power users
    // can override via the environment.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // Linux/WebKitGTK: the in-process GTK file chooser (tauri-plugin-dialog's
    // folder picker) enumerates directories via GIO and queries `standard::size`
    // on GFileInfo objects that were created without that attribute, producing
    // benign but noisy GLib-GIO-CRITICAL warnings ("GFileInfo created without
    // standard::size" / "g_file_info_get_size: should not be reached") on
    // stderr during dev. Route the file dialog through xdg-desktop-portal so the
    // enumeration happens in the portal process and the criticals stay out of our
    // stderr. Real desktop users have a portal installed; on portal-less hosts
    // (CI / build-only devcontainers) GTK transparently falls back to the
    // in-process chooser. Only set when the user hasn't explicitly chosen, so
    // power users can override via the environment.
    #[cfg(target_os = "linux")]
    if std::env::var_os("GTK_USE_PORTAL").is_none() {
        std::env::set_var("GTK_USE_PORTAL", "1");
    }

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
            commands::check_launcher_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
