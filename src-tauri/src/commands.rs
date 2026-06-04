use crate::app_state::AppState;
use tauri::Manager;
use crate::errors::{ErrorDto, LauncherResult};
use crate::models::LauncherStatus;
use std::path::PathBuf;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

pub type CommandResult<T> = Result<T, ErrorDto>;

#[tauri::command]
pub fn get_launcher_status(state: State<'_, AppState>) -> CommandResult<LauncherStatus> {
    let guard = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn validate_game_path(path: String) -> CommandResult<crate::models::GameStatus> {
    let locator = crate::game_locator::GameLocator::new(crate::models::current_platform());
    let validated = locator
        .validate_manual_root(PathBuf::from(path))
        .map_err(ErrorDto::from)?;
    Ok(crate::models::GameStatus {
        known: true,
        installed_version: crate::game_locator::installed_version(&validated),
        update_available: false,
        path: Some(validated.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub fn set_mod_channel(
    state: State<'_, AppState>,
    channel: crate::models::ModChannel,
) -> CommandResult<crate::models::LauncherStatus> {
    {
        let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        save_mod_channel_update(&mut persisted, channel, |updated| {
            crate::storage::save_state(&state.paths, updated)
        })
        .map_err(ErrorDto::from)?;
    }

    let mut status = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher status lock is poisoned".into(),
    })?;
    status.mod_status.channel = channel;
    Ok(status.clone())
}

fn save_mod_channel_update<F>(
    persisted: &mut crate::models::PersistedState,
    channel: crate::models::ModChannel,
    save: F,
) -> LauncherResult<()>
where
    F: FnOnce(&crate::models::PersistedState) -> LauncherResult<()>,
{
    let mut updated = persisted.clone();
    updated.mod_channel = channel;
    save(&updated)?;
    *persisted = updated;
    Ok(())
}

#[tauri::command]
pub async fn open_logs(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    state
        .diagnostics
        .ensure_logs_dir()
        .map_err(|err| open_logs_error(err.to_string()))?;
    let path = state.diagnostics.logs_dir().to_path_buf();
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|err| open_logs_error(err.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn get_windows_legacy_cleanup_plan(
    game_root: String,
) -> CommandResult<crate::migration::LegacyCleanupPlan> {
    crate::migration::plan_windows_legacy_cleanup(std::path::Path::new(&game_root))
        .map_err(ErrorDto::from)
}

#[tauri::command]
pub fn apply_managed_migration(
    state: State<'_, AppState>,
    game_root: String,
    remove_stale_dll: bool,
) -> CommandResult<()> {
    let plan = crate::migration::plan_windows_legacy_cleanup(std::path::Path::new(&game_root))
        .map_err(ErrorDto::from)?;
    let moved = crate::migration::apply_file_moves(&plan, &state.paths).map_err(ErrorDto::from)?;
    state
        .diagnostics
        .info("migration", &format!("moved {} legacy files", moved.len()))
        .map_err(ErrorDto::from)?;
    if remove_stale_dll {
        let removed = crate::migration::remove_stale_dll(&plan).map_err(ErrorDto::from)?;
        if let Some(path) = removed {
            state
                .diagnostics
                .info(
                    "migration",
                    &format!("removed stale DLL {}", path.display()),
                )
                .map_err(ErrorDto::from)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn read_raw_config(state: State<'_, AppState>) -> CommandResult<String> {
    crate::config_service::ConfigService::new(state.paths.config_file.clone())
        .read_config()
        .map_err(ErrorDto::from)
}

#[tauri::command]
pub fn save_raw_config(state: State<'_, AppState>, text: String) -> CommandResult<()> {
    crate::config_service::ConfigService::new(state.paths.config_file.clone())
        .write_config(&text)
        .map_err(ErrorDto::from)
}

#[tauri::command]
pub async fn open_raw_config(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<()> {
    let config_service = crate::config_service::ConfigService::new(state.paths.config_file.clone());
    config_service
        .write_config(&config_service.read_config().map_err(ErrorDto::from)?)
        .map_err(ErrorDto::from)?;
    app.opener()
        .open_path(
            state.paths.config_file.to_string_lossy().to_string(),
            None::<&str>,
        )
        .map_err(|err| ErrorDto {
            kind: "openRawConfig".into(),
            message: err.to_string(),
        })?;
    Ok(())
}

#[tauri::command]
pub fn launch_game(_app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    let game_path = persisted.game_path.clone().ok_or_else(|| ErrorDto {
        kind: "gamePath".into(),
        message: "game path is not known".into(),
    })?;
    let platform = crate::models::current_platform();
    let mod_library = state
        .paths
        .mods_dir
        .join(crate::mod_manager::platform_library_name(platform));
    let plan =
        crate::launch::build_launch_plan(platform, &game_path, &mod_library, persisted.launch_mode)
            .map_err(ErrorDto::from)?;
    state
        .diagnostics
        .info(
            "launch",
            &format!("launching with mode {:?}", persisted.launch_mode),
        )
        .map_err(ErrorDto::from)?;
    crate::launch::run_launch_plan(&plan).map_err(ErrorDto::from)
}

#[tauri::command]
pub async fn check_launcher_update(
    app: tauri::AppHandle,
) -> CommandResult<Option<crate::self_update::LauncherUpdateInfo>> {
    crate::self_update::check_for_launcher_update(app)
        .await
        .map_err(ErrorDto::from)
}

fn open_logs_error(message: impl Into<String>) -> ErrorDto {
    ErrorDto {
        kind: "openLogs".into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::LauncherError;
    use crate::models::{ModChannel, PersistedState};

    #[test]
    fn open_logs_error_uses_command_kind() {
        let error = open_logs_error("directory unavailable");

        assert_eq!(error.kind, "openLogs");
        assert_eq!(error.message, "directory unavailable");
    }

    #[test]
    fn failed_mod_channel_save_keeps_persisted_state_unchanged() {
        let mut persisted = PersistedState {
            mod_channel: ModChannel::Stable,
            installed_mod_version: Some("v1.2.3".into()),
            ..PersistedState::default()
        };

        let error = save_mod_channel_update(&mut persisted, ModChannel::Prerelease, |_| {
            Err(LauncherError::Operation {
                context: "test save".into(),
                message: "disk unavailable".into(),
            })
        })
        .expect_err("save failure returned");

        assert!(matches!(error, LauncherError::Operation { .. }));
        assert_eq!(persisted.mod_channel, ModChannel::Stable);
        assert_eq!(persisted.installed_mod_version.as_deref(), Some("v1.2.3"));
    }

    #[test]
    fn successful_mod_channel_save_updates_persisted_state() {
        let mut persisted = PersistedState::default();

        save_mod_channel_update(&mut persisted, ModChannel::Prerelease, |updated| {
            assert_eq!(updated.mod_channel, ModChannel::Prerelease);
            Ok(())
        })
        .expect("save succeeds");

        assert_eq!(persisted.mod_channel, ModChannel::Prerelease);
    }
}

#[tauri::command]
pub async fn open_config_editor(
    app: tauri::AppHandle,
) -> CommandResult<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    
    let existing = app.get_webview_window("config-editor");
    if let Some(window) = existing {
        window.set_focus().map_err(|err| ErrorDto {
            kind: "openConfigEditor".into(),
            message: err.to_string(),
        })?;
        return Ok(());
    }
    
    WebviewWindowBuilder::new(
        &app,
        "config-editor",
        WebviewUrl::App("/".into()),
    )
    .title("STFC Mod Config")
    .inner_size(980.0, 720.0)
    .build()
    .map_err(|err| ErrorDto {
        kind: "openConfigEditor".into(),
        message: err.to_string(),
    })?;
    Ok(())
}

#[tauri::command]
pub fn update_game(_app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    let game_path = persisted.game_path.clone().ok_or_else(|| ErrorDto {
        kind: "gamePath".into(),
        message: "game path is not known".into(),
    })?;
    
    // For now, just launch the game - the update logic would go here
    let platform = crate::models::current_platform();
    let mod_library = state
        .paths
        .mods_dir
        .join(crate::mod_manager::platform_library_name(platform));
    let plan =
        crate::launch::build_launch_plan(platform, &game_path, &mod_library, persisted.launch_mode)
            .map_err(ErrorDto::from)?;
    crate::launch::run_launch_plan(&plan).map_err(ErrorDto::from)
}

#[tauri::command]
pub fn update_mod(_app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    // Placeholder - mod update logic would go here
    state
        .diagnostics
        .info("update_mod", "mod update requested")
        .map_err(ErrorDto::from)?;
    Ok(())
}
