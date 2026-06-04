use crate::errors::{io_context, LauncherResult};
use crate::storage::ManagedPaths;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCleanupPlan {
    pub stale_dll: Option<PathBuf>,
    pub files_to_move: Vec<LegacyFileMove>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LegacyFileMove {
    pub source: PathBuf,
    pub destination_kind: LegacyDestination,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LegacyDestination {
    Config,
    Log,
}

pub fn plan_windows_legacy_cleanup(game_root: &Path) -> LauncherResult<LegacyCleanupPlan> {
    let mut files_to_move = Vec::new();
    let config = game_root.join("community_patch_settings.toml");
    if config.exists() {
        files_to_move.push(LegacyFileMove {
            source: config,
            destination_kind: LegacyDestination::Config,
        });
    }
    let log = game_root.join("community_patch.log");
    if log.exists() {
        files_to_move.push(LegacyFileMove {
            source: log,
            destination_kind: LegacyDestination::Log,
        });
    }
    let stale_dll = game_root.join("version.dll");
    Ok(LegacyCleanupPlan {
        stale_dll: stale_dll.exists().then_some(stale_dll),
        files_to_move,
    })
}

pub fn apply_file_moves(
    plan: &LegacyCleanupPlan,
    paths: &ManagedPaths,
) -> LauncherResult<Vec<PathBuf>> {
    paths.ensure_dirs()?;
    let mut moved = Vec::new();
    for file_move in &plan.files_to_move {
        let destination = match file_move.destination_kind {
            LegacyDestination::Config => paths.config_file.clone(),
            LegacyDestination::Log => {
                let name = file_move
                    .source
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                paths.logs_dir.join(name)
            }
        };
        if destination.exists() {
            fs::remove_file(&destination)
                .map_err(|err| io_context(format!("removing {}", destination.display()), err))?;
        }
        fs::rename(&file_move.source, &destination).map_err(|err| {
            io_context(
                format!(
                    "moving {} to {}",
                    file_move.source.display(),
                    destination.display()
                ),
                err,
            )
        })?;
        moved.push(destination);
    }
    Ok(moved)
}

pub fn remove_stale_dll(plan: &LegacyCleanupPlan) -> LauncherResult<Option<PathBuf>> {
    if let Some(path) = &plan.stale_dll {
        fs::remove_file(path)
            .map_err(|err| io_context(format!("removing {}", path.display()), err))?;
        Ok(Some(path.clone()))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_legacy_windows_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let game = root.path().join("game");
        std::fs::create_dir_all(&game).expect("game");
        std::fs::write(game.join("version.dll"), b"dll").expect("dll");
        std::fs::write(game.join("community_patch_settings.toml"), b"config").expect("config");
        std::fs::write(game.join("community_patch.log"), b"log").expect("log");

        let plan = plan_windows_legacy_cleanup(&game).expect("plan");

        assert!(plan.stale_dll.is_some());
        assert_eq!(plan.files_to_move.len(), 2);
    }

    #[test]
    fn moves_legacy_files_into_managed_location() {
        let root = tempfile::tempdir().expect("tempdir");
        let game = root.path().join("game");
        let managed = crate::storage::ManagedPaths::from_root(root.path().join("managed"));
        managed.ensure_dirs().expect("managed dirs");
        std::fs::create_dir_all(&game).expect("game");
        std::fs::write(game.join("community_patch_settings.toml"), b"config").expect("config");

        let plan = plan_windows_legacy_cleanup(&game).expect("plan");
        apply_file_moves(&plan, &managed).expect("moves");

        assert!(managed.config_file.exists());
        assert!(!game.join("community_patch_settings.toml").exists());
    }
}
