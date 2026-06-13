use crate::errors::{io_context, LauncherResult};
use crate::models::PersistedState;
use directories::ProjectDirs;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;

#[derive(Debug, Clone)]
pub struct ManagedPaths {
    pub root: PathBuf,
    pub mods_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub config_file: PathBuf,
    pub state_file: PathBuf,
}

impl ManagedPaths {
    pub fn discover() -> LauncherResult<Self> {
        let project_dirs = ProjectDirs::from("com", "stfcmod", "launcher").ok_or_else(|| {
            crate::errors::LauncherError::Operation {
                context: "resolving app data directory".into(),
                message: "platform did not provide an app data directory".into(),
            }
        })?;
        Ok(Self::from_root(project_dirs.data_local_dir().to_path_buf()))
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self {
            mods_dir: root.join("mods"),
            staging_dir: root.join("staging"),
            logs_dir: root.join("logs"),
            config_file: root.join("community_patch_settings.toml"),
            state_file: root.join("state.json"),
            root,
        }
    }

    pub fn ensure_dirs(&self) -> LauncherResult<()> {
        for path in [
            &self.root,
            &self.mods_dir,
            &self.staging_dir,
            &self.logs_dir,
        ] {
            fs::create_dir_all(path)
                .map_err(|err| io_context(format!("creating {}", path.display()), err))?;
        }
        Ok(())
    }
}

pub fn load_state(paths: &ManagedPaths) -> LauncherResult<PersistedState> {
    let text = match fs::read_to_string(&paths.state_file) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(PersistedState::default()),
        Err(err) => {
            return Err(io_context(
                format!("reading {}", paths.state_file.display()),
                err,
            ));
        }
    };
    serde_json::from_str(&text).map_err(|err| crate::errors::LauncherError::InvalidData {
        context: format!("parsing {}", paths.state_file.display()),
        message: err.to_string(),
    })
}

pub fn save_state(paths: &ManagedPaths, state: &PersistedState) -> LauncherResult<()> {
    paths.ensure_dirs()?;
    let temp_file = paths.state_file.with_extension("json.tmp");
    match fs::remove_file(&temp_file) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => {
            return Err(io_context(
                format!("removing stale {}", temp_file.display()),
                err,
            ));
        }
    }
    let state_dir =
        paths
            .state_file
            .parent()
            .ok_or_else(|| crate::errors::LauncherError::Operation {
                context: format!(
                    "resolving parent directory for {}",
                    paths.state_file.display()
                ),
                message: "state file path has no parent directory".into(),
            })?;
    let text = serde_json::to_string_pretty(state).map_err(|err| {
        crate::errors::LauncherError::InvalidData {
            context: "serializing launcher state".into(),
            message: err.to_string(),
        }
    })?;
    let mut file = NamedTempFile::new_in(state_dir).map_err(|err| {
        io_context(
            format!("creating temporary file in {}", state_dir.display()),
            err,
        )
    })?;
    file.write_all(text.as_bytes())
        .map_err(|err| io_context(format!("writing {}", file.path().display()), err))?;
    file.flush()
        .map_err(|err| io_context(format!("flushing {}", file.path().display()), err))?;
    file.as_file()
        .sync_all()
        .map_err(|err| io_context(format!("syncing {}", file.path().display()), err))?;
    file.into_temp_path()
        .persist(&paths.state_file)
        .map_err(|err| {
            let tempfile::PathPersistError { error, path } = err;
            io_context(
                format!(
                    "persisting {} to {}",
                    path.display(),
                    paths.state_file.display()
                ),
                error,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_paths_are_under_app_root() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());

        assert_eq!(
            paths.config_file,
            root.path().join("community_patch_settings.toml")
        );
        assert_eq!(paths.mods_dir, root.path().join("mods"));
        assert_eq!(paths.staging_dir, root.path().join("staging"));
        assert_eq!(paths.logs_dir, root.path().join("logs"));
        assert_eq!(paths.state_file, root.path().join("state.json"));
    }

    #[test]
    fn state_round_trips() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        let state = crate::models::PersistedState {
            installed_mod_version: Some("v1.0.0".into()),
            ..Default::default()
        };

        save_state(&paths, &state).expect("save state");
        let loaded = load_state(&paths).expect("load state");

        assert_eq!(loaded.installed_mod_version.as_deref(), Some("v1.0.0"));
    }

    #[test]
    fn missing_state_returns_default() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());

        let loaded = load_state(&paths).expect("load state");

        assert_eq!(loaded, crate::models::PersistedState::default());
    }

    #[test]
    fn save_state_replaces_stale_temp_file() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        paths.ensure_dirs().expect("dirs");
        let temp_file = paths.state_file.with_extension("json.tmp");
        std::fs::write(&temp_file, "stale").expect("write stale temp");
        let state = crate::models::PersistedState {
            installed_mod_version: Some("v2.0.0".into()),
            ..Default::default()
        };

        save_state(&paths, &state).expect("save state");
        let loaded = load_state(&paths).expect("load state");

        assert!(!temp_file.exists());
        assert_eq!(loaded.installed_mod_version.as_deref(), Some("v2.0.0"));
    }

    #[test]
    fn save_state_replaces_existing_state_file() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        let first = crate::models::PersistedState {
            installed_mod_version: Some("v1.0.0".into()),
            ..Default::default()
        };
        let second = crate::models::PersistedState {
            installed_mod_version: Some("v2.0.0".into()),
            ..Default::default()
        };

        save_state(&paths, &first).expect("save first state");
        save_state(&paths, &second).expect("save second state");
        let loaded = load_state(&paths).expect("load state");

        assert_eq!(loaded.installed_mod_version.as_deref(), Some("v2.0.0"));
    }

    #[test]
    fn partial_state_uses_defaults_for_missing_fields() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        paths.ensure_dirs().expect("dirs");
        std::fs::write(&paths.state_file, r#"{ "modChannel": "prerelease" }"#)
            .expect("write partial state");

        let loaded = load_state(&paths).expect("load state");

        assert_eq!(loaded.game_path, None);
        assert_eq!(loaded.mod_channel, crate::models::ModChannel::Prerelease);
        assert_eq!(loaded.installed_mod_version, None);
        assert_eq!(loaded.installed_mod_checksum, None);
        assert_eq!(loaded.launch_mode, crate::models::LaunchMode::Managed);
    }
}
