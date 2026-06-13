use crate::diagnostics::DiagnosticsService;
use crate::errors::{io_context, LauncherError, LauncherResult};
use crate::models::{GameStatus, LauncherStatus, ModStatus, PersistedState};
use crate::storage::{load_state, ManagedPaths};
use std::fs;
use std::io::ErrorKind;
use std::sync::Mutex;

pub struct AppState {
    pub paths: ManagedPaths,
    pub persisted: Mutex<crate::models::PersistedState>,
    pub diagnostics: DiagnosticsService,
    pub status: Mutex<LauncherStatus>,
}

impl AppState {
    pub fn new() -> LauncherResult<Self> {
        let paths = ManagedPaths::discover()?;
        Self::new_with_paths(paths)
    }

    fn new_with_paths(paths: ManagedPaths) -> LauncherResult<Self> {
        paths.ensure_dirs()?;
        let diagnostics = DiagnosticsService::new(paths.logs_dir.clone());
        let persisted = load_state_or_recover(&paths, &diagnostics)?;

        Ok(Self {
            paths,
            diagnostics,
            status: Mutex::new(LauncherStatus {
                game: GameStatus {
                    known: false,
                    path: None,
                    installed_version: None,
                    update_available: false,
                },
                mod_status: ModStatus {
                    installed: false,
                    installed_version: None,
                    latest_version: None,
                    channel: persisted.mod_channel,
                    update_available: false,
                    launch_mode: persisted.launch_mode,
                },
                launcher_update_available: false,
            }),
            persisted: Mutex::new(persisted),
        })
    }
}

fn load_state_or_recover(
    paths: &ManagedPaths,
    diagnostics: &DiagnosticsService,
) -> LauncherResult<PersistedState> {
    match load_state(paths) {
        Ok(persisted) => Ok(persisted),
        Err(err @ LauncherError::InvalidData { .. }) => {
            let _ = diagnostics.error(
                "state",
                &format!("failed to load persisted state; using defaults: {err}"),
            );
            match quarantine_state_file(paths) {
                Ok(Some(quarantine_file)) => {
                    let _ = diagnostics.warn(
                        "state",
                        &format!(
                            "quarantined invalid persisted state at {}",
                            quarantine_file.display()
                        ),
                    );
                }
                Ok(None) => {
                    let _ = diagnostics.warn("state", "no persisted state file to quarantine");
                }
                Err(quarantine_err) => {
                    let _ = diagnostics.error(
                        "state",
                        &format!("failed to quarantine invalid persisted state: {quarantine_err}"),
                    );
                }
            }
            Ok(PersistedState::default())
        }
        Err(err) => {
            let _ = diagnostics.error(
                "state",
                &format!("failed to load persisted state; startup cannot continue: {err}"),
            );
            Err(err)
        }
    }
}

fn quarantine_state_file(paths: &ManagedPaths) -> LauncherResult<Option<std::path::PathBuf>> {
    let quarantine_file = paths.state_file.with_extension("json.invalid");
    match fs::remove_file(&quarantine_file) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => {
            return Err(io_context(
                format!("removing {}", quarantine_file.display()),
                err,
            ));
        }
    }
    match fs::rename(&paths.state_file, &quarantine_file) {
        Ok(()) => Ok(Some(quarantine_file)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(io_context(
            format!(
                "renaming {} to {}",
                paths.state_file.display(),
                quarantine_file.display()
            ),
            err,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::LauncherError;
    use crate::models::{LaunchMode, ModChannel, PersistedState};
    use crate::storage::save_state;

    #[test]
    fn initial_status_keeps_detection_fields_unresolved() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        let persisted = PersistedState {
            game_path: Some(root.path().join("game")),
            mod_channel: ModChannel::Prerelease,
            installed_mod_version: Some("v1.2.3".into()),
            installed_mod_checksum: Some("abc123".into()),
            launch_mode: LaunchMode::WindowsProxyDll,
        };
        save_state(&paths, &persisted).expect("save state");

        let app_state = AppState::new_with_paths(paths).expect("app state");
        let status = app_state.status.lock().expect("status");

        assert!(!status.game.known);
        assert_eq!(status.game.path, None);
        assert_eq!(status.game.installed_version, None);
        assert!(!status.game.update_available);
        assert!(!status.mod_status.installed);
        assert_eq!(status.mod_status.installed_version, None);
        assert_eq!(status.mod_status.latest_version, None);
        assert!(!status.mod_status.update_available);
        assert_eq!(status.mod_status.channel, ModChannel::Prerelease);
        assert_eq!(status.mod_status.launch_mode, LaunchMode::WindowsProxyDll);
    }

    #[test]
    fn invalid_persisted_state_is_logged_and_reset() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        paths.ensure_dirs().expect("dirs");
        std::fs::write(&paths.state_file, "{ invalid json").expect("write invalid state");
        let quarantine_file = paths.state_file.with_extension("json.invalid");

        let app_state = AppState::new_with_paths(paths.clone()).expect("app state");
        let persisted = app_state.persisted.lock().expect("persisted");
        let log_text = std::fs::read_to_string(app_state.diagnostics.log_file()).expect("log");

        assert_eq!(*persisted, PersistedState::default());
        assert!(quarantine_file.exists());
        assert!(!paths.state_file.exists());
        assert!(log_text.contains("\"level\":\"error\""));
        assert!(log_text.contains("\"category\":\"state\""));
        assert!(log_text.contains("failed to load persisted state"));
        assert!(log_text.contains("quarantined invalid persisted state"));
    }

    #[test]
    fn io_load_error_is_logged_and_returned() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        paths.ensure_dirs().expect("dirs");
        std::fs::create_dir(&paths.state_file).expect("state dir");
        let quarantine_file = paths.state_file.with_extension("json.invalid");

        let result = AppState::new_with_paths(paths.clone());
        let log_text = std::fs::read_to_string(paths.logs_dir.join("launcher.log.jsonl"))
            .expect("diagnostics log");

        assert!(matches!(result, Err(LauncherError::Io { .. })));
        assert!(paths.state_file.is_dir());
        assert!(!quarantine_file.exists());
        assert!(log_text.contains("\"level\":\"error\""));
        assert!(log_text.contains("\"category\":\"state\""));
        assert!(log_text.contains("failed to load persisted state"));
    }
}
