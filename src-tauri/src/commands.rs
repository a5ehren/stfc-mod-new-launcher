use crate::app_state::AppState;
use crate::errors::{ErrorDto, LauncherResult};
use crate::events::ProgressEvent;
use crate::models::LauncherStatus;
use std::future::Future;
use std::path::PathBuf;
use tauri::State;
use tauri::{Emitter, Manager};
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

#[tauri::command]
pub fn set_game_path(
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<crate::models::LauncherStatus> {
    let game_path = PathBuf::from(path);
    let validated = crate::game_locator::GameLocator::new(crate::models::current_platform())
        .validate_manual_root(game_path)
        .map_err(ErrorDto::from)?;

    persist_game_path(&state, &validated)?;

    let status = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher status lock is poisoned".into(),
    })?;
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
pub async fn launch_game(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let platform = crate::models::current_platform();
    let launch_mode = {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        persisted.launch_mode
    };
    let ResolvedGame {
        root: game_path,
        prime_exe,
    } = resolve_game_path(&state, platform).await?;
    let mod_library = state
        .paths
        .mods_dir
        .join(crate::mod_manager::platform_library_name(platform));
    if !mod_library.is_file() {
        ensure_mod_library_installed(&mod_library, || async {
            perform_mod_update(&app, &state).await
        })
        .await?;
    }
    let plan = crate::launch::build_launch_plan(
        platform,
        &game_path,
        &mod_library,
        launch_mode,
        prime_exe.as_deref(),
    )
    .map_err(ErrorDto::from)?;
    state
        .diagnostics
        .info("launch", &format!("launching with mode {:?}", launch_mode))
        .map_err(ErrorDto::from)?;
    emit_progress(
        &app,
        ProgressEvent::message("launch", "starting", "starting game launch"),
    );
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

fn emit_progress(app: &tauri::AppHandle, event: ProgressEvent) {
    let _ = app.emit("launcher://progress", event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::LauncherError;
    use crate::models::{ModChannel, PersistedState};
    use std::sync::atomic::{AtomicBool, Ordering};

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

    #[test]
    fn ensure_mod_library_installed_runs_update_when_missing() {
        let root = tempfile::tempdir().expect("tempdir");
        let mod_library = root.path().join("libstfc-community-mod.dylib");
        let invoked = AtomicBool::new(false);

        tauri::async_runtime::block_on(ensure_mod_library_installed(&mod_library, || async {
            invoked.store(true, Ordering::SeqCst);
            std::fs::write(&mod_library, "mod").expect("write mod");
            Ok(())
        }))
        .expect("ensure installed");

        assert!(invoked.load(Ordering::SeqCst));
        assert!(mod_library.is_file());
    }
}

#[tauri::command]
pub async fn open_config_editor(app: tauri::AppHandle) -> CommandResult<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let existing = app.get_webview_window("config-editor");
    if let Some(window) = existing {
        window.set_focus().map_err(|err| ErrorDto {
            kind: "openConfigEditor".into(),
            message: err.to_string(),
        })?;
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "config-editor", WebviewUrl::App("/".into()))
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
pub async fn update_game(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<bool> {
    let diagnostics = state.diagnostics.clone();
    let staging_dir = state.paths.staging_dir.clone();
    let game_path = resolve_game_path(&state, crate::models::current_platform())
        .await?
        .root;
    let platform = crate::models::current_platform();
    let installed_version = crate::game_locator::installed_version(&game_path).unwrap_or(0);
    let client = reqwest::Client::new();
    let progress_app = app.clone();

    diagnostics
        .info(
            "game_update",
            &format!(
                "checking update plan from installed version {} on {:?}",
                installed_version, platform
            ),
        )
        .map_err(ErrorDto::from)?;
    emit_progress(
        &progress_app,
        ProgressEvent::message(
            "gameUpdate",
            "checking",
            format!("checking game update plan from version {installed_version}"),
        ),
    );

    let Some(plan) = crate::game_updater::fetch_update_plan(&client, platform, installed_version)
        .await
        .map_err(ErrorDto::from)?
    else {
        diagnostics
            .info("game_update", "game is already at the latest known version")
            .map_err(ErrorDto::from)?;
        emit_progress(
            &progress_app,
            ProgressEvent::message(
                "gameUpdate",
                "complete",
                "game is already at the latest known version",
            ),
        );
        return Ok(false);
    };

    let context = crate::game_updater::GameUpdateContext {
        game_root: game_path.clone(),
        xsolla_temp_root: staging_dir.join("xsolla-temp"),
        staging_root: staging_dir.join("xsolla-staging"),
    };
    let progress_app = progress_app.clone();
    crate::game_updater::run_update_plan(&client, &plan, &context, move |event| {
        emit_progress(&progress_app, event);
    })
    .await
    .map_err(ErrorDto::from)?;

    diagnostics
        .info(
            "game_update",
            &format!("completed update plan to version {:?}", plan.target_version),
        )
        .map_err(ErrorDto::from)?;
    Ok(true)
}

#[tauri::command]
pub async fn update_mod(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    perform_mod_update(&app, &state).await
}

async fn perform_mod_update(
    app: &tauri::AppHandle,
    state: &State<'_, AppState>,
) -> CommandResult<()> {
    let platform = crate::models::current_platform();
    let channel = {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        persisted.mod_channel
    };
    let client = reqwest::Client::new();
    let diagnostics = state.diagnostics.clone();
    let progress_app = app.clone();

    diagnostics
        .info(
            "mod_update",
            &format!("checking mod releases for {platform:?} on {channel:?}"),
        )
        .map_err(ErrorDto::from)?;
    emit_progress(
        &progress_app,
        ProgressEvent::message("modUpdate", "checking", "checking mod release channel"),
    );

    let releases = crate::github_releases::fetch_releases(&client)
        .await
        .map_err(ErrorDto::from)?;
    let selected = crate::github_releases::select_release_asset(&releases, platform, channel)
        .map_err(ErrorDto::from)?;

    emit_progress(
        &progress_app,
        ProgressEvent::message(
            "modUpdate",
            "download",
            format!("downloading mod archive {}", selected.archive_name),
        ),
    );
    let archive_bytes = client
        .get(&selected.archive_url)
        .send()
        .await
        .map_err(|source| ErrorDto {
            kind: "network".into(),
            message: source.to_string(),
        })?
        .error_for_status()
        .map_err(|source| ErrorDto {
            kind: "network".into(),
            message: source.to_string(),
        })?
        .bytes()
        .await
        .map_err(|source| ErrorDto {
            kind: "network".into(),
            message: source.to_string(),
        })?;
    let checksum_text = client
        .get(&selected.checksum_url)
        .send()
        .await
        .map_err(|source| ErrorDto {
            kind: "network".into(),
            message: source.to_string(),
        })?
        .error_for_status()
        .map_err(|source| ErrorDto {
            kind: "network".into(),
            message: source.to_string(),
        })?
        .text()
        .await
        .map_err(|source| ErrorDto {
            kind: "network".into(),
            message: source.to_string(),
        })?;
    let expected_checksum =
        crate::mod_manager::parse_sha256(&checksum_text).map_err(ErrorDto::from)?;

    let update_dir = tempfile::Builder::new()
        .prefix("mod-update")
        .tempdir_in(&state.paths.staging_dir)
        .map_err(|err| ErrorDto {
            kind: "operation".into(),
            message: err.to_string(),
        })?;
    let archive_path = update_dir.path().join(&selected.archive_name);
    std::fs::write(&archive_path, &archive_bytes).map_err(|err| ErrorDto {
        kind: "io".into(),
        message: format!("writing {}: {err}", archive_path.display()),
    })?;

    let actual_checksum = crate::mod_manager::sha256_file(&archive_path).map_err(ErrorDto::from)?;
    if actual_checksum != expected_checksum {
        return Err(ErrorDto {
            kind: "invalidData".into(),
            message: format!(
                "checksum mismatch for {}: expected {}, got {}",
                selected.archive_name, expected_checksum, actual_checksum
            ),
        });
    }

    emit_progress(
        &progress_app,
        ProgressEvent::message("modUpdate", "extract", "extracting mod archive"),
    );
    let extract_dir = update_dir.path().join("extract");
    crate::mod_manager::extract_mod_archive(&archive_path, &extract_dir).map_err(ErrorDto::from)?;

    emit_progress(
        &progress_app,
        ProgressEvent::message("modUpdate", "install", "installing mod library"),
    );
    let installed =
        crate::mod_manager::install_staged_library(&extract_dir, &state.paths.mods_dir, platform)
            .map_err(ErrorDto::from)?;

    {
        let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        persisted.installed_mod_version = Some(selected.version.clone());
        persisted.installed_mod_checksum = Some(actual_checksum.clone());
        crate::storage::save_state(&state.paths, &persisted).map_err(ErrorDto::from)?;
    }

    {
        let mut status = state.status.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher status lock is poisoned".into(),
        })?;
        status.mod_status.installed = true;
        status.mod_status.installed_version = Some(selected.version.clone());
        status.mod_status.latest_version = Some(selected.version.clone());
        status.mod_status.update_available = false;
    }

    diagnostics
        .info(
            "mod_update",
            &format!(
                "installed mod release {} to {}",
                selected.version,
                installed.display()
            ),
        )
        .map_err(ErrorDto::from)?;

    emit_progress(
        &progress_app,
        ProgressEvent::message("modUpdate", "complete", "mod update completed"),
    );
    Ok(())
}

async fn ensure_mod_library_installed<F, Fut>(
    mod_library: &std::path::Path,
    update_mod: F,
) -> CommandResult<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = CommandResult<()>>,
{
    if mod_library.is_file() {
        return Ok(());
    }

    update_mod().await?;
    if mod_library.is_file() {
        return Ok(());
    }

    Err(ErrorDto {
        kind: "invalidData".into(),
        message: format!(
            "mod update completed but {} was still missing",
            mod_library.display()
        ),
    })
}

/// A validated game install: its root plus, for WINE prefixes, the `prime.exe`
/// located while validating so launch plan building need not re-scan the prefix.
struct ResolvedGame {
    root: PathBuf,
    prime_exe: Option<PathBuf>,
}

async fn resolve_game_path(
    state: &State<'_, AppState>,
    platform: crate::models::Platform,
) -> CommandResult<ResolvedGame> {
    let persisted_path = {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        persisted.game_path.clone()
    };
    if let Some(path) = persisted_path {
        // Re-validate: the persisted path may now be stale (game moved or
        // uninstalled). For WINE this walk also yields prime.exe, which we carry
        // forward so build_launch_plan does not have to scan the prefix again.
        match platform {
            crate::models::Platform::LinuxWine => {
                if let Some(prime_exe) = crate::game_locator::find_prime_exe_in_wine_prefix(&path) {
                    return Ok(ResolvedGame {
                        root: path,
                        prime_exe: Some(prime_exe),
                    });
                }
            }
            _ => {
                if crate::game_locator::is_valid_game_root(&path, platform) {
                    return Ok(ResolvedGame {
                        root: path,
                        prime_exe: None,
                    });
                }
            }
        }
        // Stale persisted path; fall through to re-discovery rather than running
        // an update/launch against a directory that is no longer a valid install.
    }

    let home_dir = directories::BaseDirs::new()
        .map(|base_dirs| base_dirs.home_dir().to_path_buf())
        .ok_or_else(|| ErrorDto {
            kind: "gamePath".into(),
            message: "game path is not known".into(),
        })?;
    let discovered =
        crate::game_locator::discover_game_root(platform, &home_dir).map_err(ErrorDto::from)?;
    let Some(path) = discovered else {
        return Err(ErrorDto {
            kind: "gamePath".into(),
            message: "game path is not known".into(),
        });
    };

    persist_game_path(state, &path)?;
    Ok(ResolvedGame {
        root: path,
        prime_exe: None,
    })
}

fn persist_game_path(state: &State<'_, AppState>, path: &std::path::Path) -> CommandResult<()> {
    {
        let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        let mut updated = persisted.clone();
        updated.game_path = Some(path.to_path_buf());
        crate::storage::save_state(&state.paths, &updated).map_err(ErrorDto::from)?;
        *persisted = updated;
    }

    let mut status = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher status lock is poisoned".into(),
    })?;
    status.game.known = true;
    status.game.path = Some(path.to_string_lossy().to_string());
    status.game.installed_version = crate::game_locator::installed_version(path);
    Ok(())
}
