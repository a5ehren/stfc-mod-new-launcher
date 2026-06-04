use crate::app_state::AppState;
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
