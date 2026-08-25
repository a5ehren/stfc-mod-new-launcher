use crate::app_state::AppState;
use crate::errors::{ErrorDto, LauncherError, LauncherResult};
use crate::events::ProgressEvent;
use crate::models::LauncherStatus;
use std::future::Future;
use std::path::PathBuf;
use tauri::Emitter;
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
        latest_version: None,
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
    let mi_enabled = {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        persisted.multi_instance.enabled
    };
    game_path_lock_check(mi_enabled)?;
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

// --- Multi-instance mode (spec: docs/superpowers/specs/2026-08-24-multi-instance-mode-design.md)

/// F3: while multi-instance mode is enabled, game_path is pinned to the
/// shared install — the manual game-path picker must not repoint it.
fn game_path_lock_check(multi_instance_enabled: bool) -> Result<(), ErrorDto> {
    if multi_instance_enabled {
        Err(ErrorDto {
            kind: "gamePath".into(),
            message: "game path is locked to the shared install while multi-instance mode is enabled; disable multi-instance first".into(),
        })
    } else {
        Ok(())
    }
}

/// F2b: under multi-instance mode the shared install is the only legal game
/// root — a stale persisted path must error rather than re-discover the
/// original install from the primary user's home (split-brain).
fn home_rediscovery_allowed(multi_instance_enabled: bool) -> bool {
    !multi_instance_enabled
}

fn remove_guard(has_backup: bool, force: bool) -> LauncherResult<()> {
    if has_backup || force {
        Ok(())
    } else {
        Err(LauncherError::InvalidData {
            context: "removing instance".into(),
            message: "no account backup exists; run a backup first or pass force".into(),
        })
    }
}

fn update_gate(running: &[bool]) -> LauncherResult<()> {
    if running.iter().any(|r| *r) {
        Err(LauncherError::InvalidData {
            context: "updating game".into(),
            message: "stop all game instances before updating (they share one install)".into(),
        })
    } else {
        Ok(())
    }
}

fn mi_not_enabled() -> ErrorDto {
    ErrorDto {
        kind: "invalidData".into(),
        message: "multi-instance mode is not enabled".into(),
    }
}

/// Locks persisted state, requires multi-instance mode enabled, and returns
/// the registered instance with `name` (managed-registry enforcement, FR-8.1).
fn managed_instance(
    state: &State<'_, AppState>,
    name: &str,
) -> CommandResult<crate::models::Instance> {
    let persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    if !persisted.multi_instance.enabled {
        return Err(mi_not_enabled());
    }
    persisted
        .multi_instance
        .instances
        .iter()
        .find(|i| i.name == name)
        .filter(|i| crate::instance_users::is_managed(&persisted.multi_instance, &i.os_username))
        .cloned()
        .ok_or_else(|| ErrorDto {
            kind: "invalidData".into(),
            message: format!("unknown instance {name:?}"),
        })
}

/// Resolves an instance name to (os_username, is_base). The reserved "base"
/// name targets the primary account — the OS user the launcher runs as — and
/// bypasses the managed registry (it is never a provisioned service user).
fn resolve_instance_target(
    state: &State<'_, AppState>,
    name: &str,
) -> CommandResult<(String, bool)> {
    if name == crate::instance_users::BASE_INSTANCE_NAME {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        if !persisted.multi_instance.enabled {
            return Err(mi_not_enabled());
        }
        let username = crate::instance_users::current_username().map_err(ErrorDto::from)?;
        return Ok((username, true));
    }
    Ok((managed_instance(state, name)?.os_username, false))
}

/// Copies persisted.multi_instance into the status snapshot after a mutation.
fn sync_multi_instance_status(state: &State<'_, AppState>) -> CommandResult<()> {
    let mi = {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        persisted.multi_instance.clone()
    };
    let mut status = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher status lock is poisoned".into(),
    })?;
    status.multi_instance = mi;
    Ok(())
}

fn instance_shared_root(mi: &crate::models::MultiInstanceState) -> CommandResult<PathBuf> {
    mi.shared_game_root.clone().ok_or_else(|| ErrorDto {
        kind: "invalidData".into(),
        message: "multi-instance shared game root is not configured".into(),
    })
}

#[tauri::command]
pub fn mi_wizard_plan(state: State<'_, AppState>) -> CommandResult<crate::models::WizardPlanDto> {
    let persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    let shared_root = crate::provisioning::default_shared_root(crate::models::current_platform());
    let status =
        crate::provisioning::relocation_status(persisted.game_path.as_deref(), &shared_root);
    Ok(crate::models::WizardPlanDto {
        needs_relocation: matches!(
            status,
            crate::provisioning::RelocationStatus::NeedsMove { .. }
        ),
        game_source: persisted.game_path.clone(),
        shared_root,
        existing_names: persisted
            .multi_instance
            .instances
            .iter()
            .map(|i| i.name.clone())
            .collect(),
    })
}

#[tauri::command]
pub async fn mi_provision(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    names: Vec<String>,
) -> CommandResult<crate::models::MultiInstanceState> {
    if names.is_empty() {
        return Err(ErrorDto {
            kind: "invalidData".into(),
            message: "no instance names given".into(),
        });
    }
    for name in &names {
        crate::instance_users::validate_instance_name(name).map_err(ErrorDto::from)?;
    }
    let (game_source, shared_root) = {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        (
            persisted.game_path.clone(),
            crate::provisioning::default_shared_root(crate::models::current_platform()),
        )
    };
    emit_progress(
        &app,
        ProgressEvent::message(
            "mi_provision",
            "elevating",
            "Provisioning instances (admin password required)\u{2026}",
        ),
    );
    #[cfg(target_os = "macos")]
    let primary_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    #[cfg(target_os = "macos")]
    let script = crate::provisioning::macos_provision_script(
        &primary_user,
        game_source.as_deref().unwrap_or(std::path::Path::new("")),
        &shared_root,
        &names,
    );
    #[cfg(target_os = "windows")]
    let script = {
        let mut passwords = Vec::with_capacity(names.len());
        for _ in &names {
            passwords.push(crate::provisioning::generate_password().map_err(ErrorDto::from)?);
        }
        for (name, pw) in names.iter().zip(&passwords) {
            crate::provisioning::store_windows_password(
                &crate::instance_users::os_username(name),
                pw,
            )
            .map_err(ErrorDto::from)?;
        }
        crate::provisioning::windows_provision_script(
            game_source.as_deref().unwrap_or(std::path::Path::new("")),
            &shared_root,
            &names,
            &passwords,
        )
    };
    tauri::async_runtime::spawn_blocking(move || crate::provisioning::run_elevated(&script))
        .await
        .map_err(|err| ErrorDto {
            kind: "operation".into(),
            message: format!("provision task failed: {err}"),
        })?
        .map_err(ErrorDto::from)?;
    // F2a: verify the relocated payload before re-pinning game_path (D-2) —
    // a partial ditto must not enable the mode. Skipped when the game already
    // lived at the shared root (no relocation happened).
    if let Some(source) = &game_source {
        if source != &shared_root && source.is_dir() {
            let source_count = crate::provisioning::file_count(source).map_err(ErrorDto::from)?;
            let shared_count =
                crate::provisioning::file_count(&shared_root).map_err(ErrorDto::from)?;
            if source_count != shared_count {
                return Err(ErrorDto {
                    kind: "operation".into(),
                    message: format!(
                        "shared install verification failed: {} has {shared_count} files but the source {} has {source_count}; re-run the wizard",
                        shared_root.display(),
                        source.display()
                    ),
                });
            }
        }
    }
    let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    for name in &names {
        let os_username = crate::instance_users::os_username(name);
        if !persisted
            .multi_instance
            .instances
            .iter()
            .any(|i| i.os_username == os_username)
        {
            persisted
                .multi_instance
                .instances
                .push(crate::models::Instance {
                    name: name.clone(),
                    os_username,
                    created_at: chrono::Utc::now(),
                    last_backup_at: None,
                    label: None,
                });
        }
    }
    persisted.game_path = Some(shared_root.clone());
    persisted.multi_instance.shared_game_root = Some(shared_root);
    persisted.multi_instance.enabled = true;
    crate::storage::save_state(&state.paths, &persisted).map_err(ErrorDto::from)?;
    let mi = persisted.multi_instance.clone();
    drop(persisted);
    sync_multi_instance_status(&state)?;
    Ok(mi)
}

#[tauri::command]
pub fn mi_set_instance_label(
    state: State<'_, AppState>,
    name: String,
    label: String,
) -> CommandResult<crate::models::MultiInstanceState> {
    let trimmed = label.trim();
    if trimmed.chars().count() > 32 || trimmed.chars().any(|c| c.is_control()) {
        return Err(ErrorDto {
            kind: "invalidData".into(),
            message: "label must be at most 32 characters with no control characters".into(),
        });
    }
    let label = (!trimmed.is_empty()).then(|| trimmed.to_string());
    let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    if !persisted.multi_instance.enabled {
        return Err(mi_not_enabled());
    }
    if name == crate::instance_users::BASE_INSTANCE_NAME {
        persisted.multi_instance.base_label = label;
    } else {
        let instance = persisted
            .multi_instance
            .instances
            .iter_mut()
            .find(|i| i.name == name)
            .ok_or_else(|| ErrorDto {
                kind: "invalidData".into(),
                message: format!("unknown instance {name:?}"),
            })?;
        instance.label = label;
    }
    crate::storage::save_state(&state.paths, &persisted).map_err(ErrorDto::from)?;
    let mi = persisted.multi_instance.clone();
    drop(persisted);
    sync_multi_instance_status(&state)?;
    Ok(mi)
}

#[tauri::command]
pub fn mi_set_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> CommandResult<crate::models::MultiInstanceState> {
    let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    if !persisted.multi_instance.enabled {
        return Err(mi_not_enabled());
    }
    persisted.multi_instance.enabled = enabled;
    crate::storage::save_state(&state.paths, &persisted).map_err(ErrorDto::from)?;
    let mi = persisted.multi_instance.clone();
    drop(persisted);
    sync_multi_instance_status(&state)?;
    Ok(mi)
}

#[tauri::command]
pub async fn mi_start_instance(state: State<'_, AppState>, name: String) -> CommandResult<u32> {
    let (platform, shared_root, username, mod_library, log_file) = {
        let (username, _is_base) = resolve_instance_target(&state, &name)?;
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        let platform = crate::models::current_platform();
        (
            platform,
            instance_shared_root(&persisted.multi_instance)?,
            username,
            state
                .paths
                .mods_dir
                .join(crate::mod_manager::platform_library_name(platform)),
            state.paths.logs_dir.join(format!("instance-{name}.log")),
        )
    };
    tauri::async_runtime::spawn_blocking(move || {
        crate::instance_launch::start_instance(
            platform,
            &shared_root,
            &mod_library,
            &username,
            &log_file,
        )
    })
    .await
    .map_err(|err| ErrorDto {
        kind: "operation".into(),
        message: format!("start task failed: {err}"),
    })?
    .map_err(ErrorDto::from)
}

#[tauri::command]
pub async fn mi_stop_instance(state: State<'_, AppState>, name: String) -> CommandResult<()> {
    let (platform, username) = {
        let (username, _is_base) = resolve_instance_target(&state, &name)?;
        (crate::models::current_platform(), username)
    };
    tauri::async_runtime::spawn_blocking(move || {
        crate::instance_launch::stop_instance(platform, &username)
    })
    .await
    .map_err(|err| ErrorDto {
        kind: "operation".into(),
        message: format!("stop task failed: {err}"),
    })?
    .map_err(ErrorDto::from)
}

#[tauri::command]
pub fn mi_instance_status(
    state: State<'_, AppState>,
) -> CommandResult<Vec<crate::models::InstanceStatusDto>> {
    let persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    if !persisted.multi_instance.enabled {
        return Err(mi_not_enabled());
    }
    let matcher = crate::instance_launch::process_matcher(crate::models::current_platform());
    let mi = &persisted.multi_instance;
    let mut rows = Vec::with_capacity(mi.instances.len() + 1);
    // The base account (primary user) is always the first row.
    let base_username = crate::instance_users::current_username().map_err(ErrorDto::from)?;
    let base_pid =
        crate::instance_launch::instance_pid(&base_username, matcher).map_err(ErrorDto::from)?;
    rows.push(crate::models::InstanceStatusDto {
        name: crate::instance_users::BASE_INSTANCE_NAME.into(),
        os_username: base_username,
        running: base_pid.is_some(),
        pid: base_pid,
        last_backup_at: mi.base_last_backup_at,
        label: mi.base_label.clone(),
        is_base: true,
    });
    for instance in &mi.instances {
        let pid = crate::instance_launch::instance_pid(&instance.os_username, matcher)
            .map_err(ErrorDto::from)?;
        rows.push(crate::models::InstanceStatusDto {
            name: instance.name.clone(),
            os_username: instance.os_username.clone(),
            running: pid.is_some(),
            pid,
            last_backup_at: instance.last_backup_at,
            label: instance.label.clone(),
            is_base: false,
        });
    }
    Ok(rows)
}

#[tauri::command]
pub async fn mi_backup_instance(state: State<'_, AppState>, name: String) -> CommandResult<String> {
    let (username, is_base) = resolve_instance_target(&state, &name)?;
    let paths = state.paths.clone();
    let backup_name = name.clone();
    let out_dir = tauri::async_runtime::spawn_blocking(move || {
        crate::instance_backup::backup_instance(&paths, &backup_name, &username)
    })
    .await
    .map_err(|err| ErrorDto {
        kind: "operation".into(),
        message: format!("backup task failed: {err}"),
    })?
    .map_err(ErrorDto::from)?;
    {
        let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        if is_base {
            persisted.multi_instance.base_last_backup_at = Some(chrono::Utc::now());
        } else if let Some(instance) = persisted
            .multi_instance
            .instances
            .iter_mut()
            .find(|i| i.name == name)
        {
            instance.last_backup_at = Some(chrono::Utc::now());
        }
        crate::storage::save_state(&state.paths, &persisted).map_err(ErrorDto::from)?;
    }
    sync_multi_instance_status(&state)?;
    Ok(out_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn mi_restore_instance(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> CommandResult<()> {
    let (username, _is_base) = resolve_instance_target(&state, &name)?;
    let paths = state.paths.clone();
    emit_progress(
        &app,
        ProgressEvent::message("mi_restore", "restoring", format!("restoring {name}")),
    );
    // restore_instance stops the instance itself (idempotent); do not double-stop.
    tauri::async_runtime::spawn_blocking(move || {
        crate::instance_backup::restore_instance(&paths, &name, &username)
    })
    .await
    .map_err(|err| ErrorDto {
        kind: "operation".into(),
        message: format!("restore task failed: {err}"),
    })?
    .map_err(ErrorDto::from)?;
    emit_progress(
        &app,
        ProgressEvent::message("mi_restore", "complete", "restore complete"),
    );
    Ok(())
}

/// Deprovision script for one service user (destructive; runs elevated).
#[cfg(target_os = "windows")]
fn deprovision_script(username: &str) -> String {
    format!(
        "Remove-LocalUser '{username}'\nGet-CimInstance Win32_UserProfile | Where-Object {{ $_.LocalPath -like '*{username}' }} | Remove-CimInstance\n"
    )
}

#[tauri::command]
pub async fn mi_remove_instance(
    state: State<'_, AppState>,
    name: String,
    force: bool,
) -> CommandResult<crate::models::MultiInstanceState> {
    if name == crate::instance_users::BASE_INSTANCE_NAME {
        return Err(ErrorDto {
            kind: "invalidData".into(),
            message: "the base account cannot be removed".into(),
        });
    }
    let (platform, username) = {
        let instance = managed_instance(&state, &name)?;
        (crate::models::current_platform(), instance.os_username)
    };
    remove_guard(
        crate::instance_backup::latest_backup(&state.paths, &name).is_some(),
        force,
    )
    .map_err(ErrorDto::from)?;
    // Stop (ignore "not running" — stop_instance is idempotent), then delete the
    // OS user. On macOS this opens a Terminal window for sysadminctl's secure
    // password prompt and polls for the record to vanish (provisioning.rs); on
    // Windows the UAC helper path runs the removal script. Removal from the
    // registry only happens after deprovision succeeds, so a failed remove
    // leaves a repairable state.
    let stop_username = username.clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::instance_launch::stop_instance(platform, &stop_username)
    })
    .await
    .map_err(|err| ErrorDto {
        kind: "operation".into(),
        message: format!("stop task failed: {err}"),
    })?
    .map_err(ErrorDto::from)?;
    #[cfg(target_os = "macos")]
    let deprovision = {
        let username = username.clone();
        move || crate::provisioning::deprovision_user(&username)
    };
    #[cfg(target_os = "windows")]
    let deprovision = {
        let script = deprovision_script(&username);
        move || crate::provisioning::run_elevated(&script)
    };
    tauri::async_runtime::spawn_blocking(deprovision)
        .await
        .map_err(|err| ErrorDto {
            kind: "operation".into(),
            message: format!("deprovision task failed: {err}"),
        })?
        .map_err(ErrorDto::from)?;
    let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    persisted
        .multi_instance
        .instances
        .retain(|i| i.os_username != username);
    crate::storage::save_state(&state.paths, &persisted).map_err(ErrorDto::from)?;
    let mi = persisted.multi_instance.clone();
    drop(persisted);
    sync_multi_instance_status(&state)?;
    Ok(mi)
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
    let ResolvedGame { root: game_path } = resolve_game_path(&state, platform).await?;
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
    let plan = crate::launch::build_launch_plan(platform, &game_path, &mod_library, launch_mode)
        .map_err(ErrorDto::from)?;
    state
        .diagnostics
        .info("launch", &format!("launching with mode {launch_mode:?}"))
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
    state: State<'_, AppState>,
) -> CommandResult<Option<crate::self_update::LauncherUpdateInfo>> {
    let info = crate::self_update::check_for_launcher_update(app)
        .await
        .map_err(ErrorDto::from)?;
    if let Some(info) = &info {
        state
            .diagnostics
            .info(
                "self_update",
                &format!("launcher update available: {}", info.version),
            )
            .map_err(ErrorDto::from)?;
    }
    let mut status = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher status lock is poisoned".into(),
    })?;
    status.launcher_update_available = info.is_some();
    drop(status);
    Ok(info)
}

#[tauri::command]
pub async fn install_launcher_update(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<bool> {
    let progress_app = app.clone();
    let installed = crate::self_update::install_launcher_update(app, move |downloaded, total| {
        emit_progress(
            &progress_app,
            ProgressEvent {
                operation: "launcherUpdate".into(),
                phase: "downloading".into(),
                message: "downloading launcher update".into(),
                current: Some(downloaded),
                total,
            },
        );
    })
    .await
    .map_err(ErrorDto::from)?;

    if installed {
        state
            .diagnostics
            .info("self_update", "launcher update installed; restart required")
            .map_err(ErrorDto::from)?;
        let mut status = state.status.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher status lock is poisoned".into(),
        })?;
        status.launcher_update_available = false;
    }
    Ok(installed)
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
    fn remove_refused_without_backup_or_force() {
        // remove_guard returns Err only when no backup and !force
        assert!(remove_guard(false, false).is_err());
        assert!(remove_guard(true, false).is_ok());
        assert!(remove_guard(false, true).is_ok());
        assert!(remove_guard(true, true).is_ok());
    }

    #[test]
    fn update_gate_blocks_while_instances_running() {
        assert!(update_gate(&[true]).is_err());
        assert!(update_gate(&[false, false]).is_ok());
        assert!(update_gate(&[]).is_ok());
    }

    #[test]
    fn set_game_path_locked_while_multi_instance_enabled() {
        let err = game_path_lock_check(true).expect_err("locked under MI mode");
        assert_eq!(err.kind, "gamePath");
        assert!(err.message.contains("multi-instance"));
        assert!(game_path_lock_check(false).is_ok());
    }

    #[test]
    fn home_rediscovery_suppressed_while_multi_instance_enabled() {
        // Split-brain guard: with MI on, a stale game_path must error rather
        // than re-pin to the original home-dir install (D-2).
        assert!(!home_rediscovery_allowed(true));
        assert!(home_rediscovery_allowed(false));
    }

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

    fn mock_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("mock app")
    }

    #[test]
    fn open_config_editor_creates_window_when_absent() {
        use tauri::Manager;

        let app = mock_app();
        assert!(app.get_webview_window("config-editor").is_none());

        open_or_focus_config_editor(&app).expect("create config editor window");

        assert!(app.get_webview_window("config-editor").is_some());
    }

    #[test]
    fn open_config_editor_focuses_existing_window() {
        use tauri::Manager;

        let app = mock_app();
        tauri::WebviewWindowBuilder::new(&app, "config-editor", tauri::WebviewUrl::App("/".into()))
            .build()
            .expect("seed config editor window");

        // Calling again must not error or create a second window; it focuses the
        // existing one.
        open_or_focus_config_editor(&app).expect("focus existing config editor window");

        assert!(app.get_webview_window("config-editor").is_some());
    }
}

#[tauri::command]
pub async fn open_config_editor(app: tauri::AppHandle) -> CommandResult<()> {
    open_or_focus_config_editor(&app)
}

/// Focuses the config-editor window if it already exists, otherwise creates it.
/// Generic over the runtime so it can be exercised under `tauri::test`'s mock
/// runtime (which builds windows without spawning a native webview).
fn open_or_focus_config_editor<R: tauri::Runtime>(
    manager: &impl tauri::Manager<R>,
) -> CommandResult<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    if let Some(window) = manager.get_webview_window("config-editor") {
        window.set_focus().map_err(|err| ErrorDto {
            kind: "openConfigEditor".into(),
            message: err.to_string(),
        })?;
        return Ok(());
    }

    WebviewWindowBuilder::new(manager, "config-editor", WebviewUrl::App("/".into()))
        .title("STFC Mod Config")
        .inner_size(980.0, 720.0)
        .build()
        .map_err(|err| ErrorDto {
            kind: "openConfigEditor".into(),
            message: err.to_string(),
        })?;
    Ok(())
}

/// Checks Xsolla for a game update plan from the installed version and
/// reconciles the game status snapshot. Opportunistic: when no game path is
/// known the current status is returned unchanged.
#[tauri::command]
pub async fn check_game_update(state: State<'_, AppState>) -> CommandResult<LauncherStatus> {
    let platform = crate::models::current_platform();
    let game_path = match resolve_game_path(&state, platform).await {
        Ok(resolved) => resolved.root,
        Err(_) => {
            let status = state.status.lock().map_err(|_| ErrorDto {
                kind: "state".into(),
                message: "launcher status lock is poisoned".into(),
            })?;
            return Ok(status.clone());
        }
    };
    let installed_version = crate::game_locator::installed_version(&game_path);

    {
        let mut status = state.status.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher status lock is poisoned".into(),
        })?;
        status.game.known = true;
        status.game.path = Some(game_path.to_string_lossy().to_string());
        status.game.installed_version = installed_version;
    }

    state
        .diagnostics
        .info(
            "game_update",
            &format!(
                "checking update plan from installed version {} on {:?}",
                installed_version.unwrap_or(0),
                platform
            ),
        )
        .map_err(ErrorDto::from)?;

    let client = reqwest::Client::new();
    let plan = match crate::game_updater::fetch_update_plan(
        &client,
        platform,
        installed_version.unwrap_or(0),
    )
    .await
    {
        Ok(plan) => plan,
        Err(err) => {
            let _ = state
                .diagnostics
                .error("game_update", &format!("game update check failed: {err}"));
            return Err(ErrorDto::from(err));
        }
    };

    let mut status = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher status lock is poisoned".into(),
    })?;
    status.game.latest_version = plan.as_ref().and_then(|plan| plan.target_version);
    status.game.update_available = plan.is_some();
    Ok(status.clone())
}

#[tauri::command]
pub async fn update_game(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<bool> {
    // Update gate (spec §6.2): instances share one install; refuse while any run.
    {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        if persisted.multi_instance.enabled {
            let matcher =
                crate::instance_launch::process_matcher(crate::models::current_platform());
            let mut running = Vec::new();
            for instance in &persisted.multi_instance.instances {
                running.push(
                    crate::instance_launch::instance_pid(&instance.os_username, matcher)
                        .map_err(ErrorDto::from)?
                        .is_some(),
                );
            }
            update_gate(&running).map_err(ErrorDto::from)?;
        }
    }
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

/// Checks GitHub for the latest mod release on the configured channel and
/// reconciles the status snapshot: whether the platform library is present,
/// which version is installed (from persisted state), and whether the latest
/// release differs. Installs nothing; the Update Mod button surfaces when
/// `update_available` is set.
#[tauri::command]
pub async fn check_mod_update(state: State<'_, AppState>) -> CommandResult<LauncherStatus> {
    let platform = crate::models::current_platform();
    let (channel, installed_version) = {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        (
            persisted.mod_channel,
            persisted.installed_mod_version.clone(),
        )
    };

    let library = state
        .paths
        .mods_dir
        .join(crate::mod_manager::platform_library_name(platform));
    let library_present = library.is_file();

    // Reflect on-disk reality even if the network check below fails.
    {
        let mut status = state.status.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher status lock is poisoned".into(),
        })?;
        status.mod_status.installed = library_present;
        status.mod_status.installed_version = if library_present {
            installed_version.clone()
        } else {
            None
        };
        if !library_present {
            status.mod_status.update_available = true;
        }
    }

    state
        .diagnostics
        .info(
            "mod_update",
            &format!("checking mod releases for {platform:?} on {channel:?}"),
        )
        .map_err(ErrorDto::from)?;

    let client = reqwest::Client::new();
    let selected = match crate::github_releases::fetch_releases(&client)
        .await
        .and_then(|releases| {
            crate::github_releases::select_release_asset(&releases, platform, channel)
        }) {
        Ok(selected) => selected,
        Err(err) => {
            let _ = state
                .diagnostics
                .error("mod_update", &format!("mod release check failed: {err}"));
            return Err(ErrorDto::from(err));
        }
    };

    let mut status = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher status lock is poisoned".into(),
    })?;
    status.mod_status.latest_version = Some(selected.version.clone());
    status.mod_status.update_available = !library_present
        || installed_version
            .as_deref()
            .is_none_or(|v| crate::github_releases::is_newer_version(&selected.version, v));
    Ok(status.clone())
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

/// A validated game install root.
struct ResolvedGame {
    root: PathBuf,
}

async fn resolve_game_path(
    state: &State<'_, AppState>,
    platform: crate::models::Platform,
) -> CommandResult<ResolvedGame> {
    let (persisted_path, mi_enabled) = {
        let persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        (
            persisted.game_path.clone(),
            persisted.multi_instance.enabled,
        )
    };
    if let Some(path) = persisted_path {
        // Re-validate: the persisted path may now be stale (game moved or
        // uninstalled).
        if crate::game_locator::is_valid_game_root(&path, platform) {
            return Ok(ResolvedGame { root: path });
        }
        if !home_rediscovery_allowed(mi_enabled) {
            return Err(ErrorDto {
                kind: "gamePath".into(),
                message: format!(
                    "shared game install at {} is invalid; re-run the multi-instance wizard to repair",
                    path.display()
                ),
            });
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
    Ok(ResolvedGame { root: path })
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
