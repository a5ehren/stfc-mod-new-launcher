use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    MacOs,
    Windows,
}

pub fn current_platform() -> Platform {
    #[cfg(target_os = "macos")]
    {
        Platform::MacOs
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        compile_error!("unsupported target OS: only macOS and Windows are supported");
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModChannel {
    Stable,
    Prerelease,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LaunchMode {
    Managed,
    WindowsProxyDll,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub known: bool,
    pub path: Option<String>,
    pub installed_version: Option<u32>,
    pub latest_version: Option<u32>,
    pub update_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModStatus {
    pub installed: bool,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub channel: ModChannel,
    pub update_available: bool,
    pub launch_mode: LaunchMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    pub game: GameStatus,
    pub mod_status: ModStatus,
    pub launcher_update_available: bool,
    pub multi_instance: MultiInstanceState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WizardPlanDto {
    pub needs_relocation: bool,
    pub game_source: Option<PathBuf>,
    pub shared_root: PathBuf,
    pub existing_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatusDto {
    pub name: String,
    pub os_username: String,
    pub running: bool,
    pub pid: Option<u32>,
    pub last_backup_at: Option<chrono::DateTime<chrono::Utc>>,
    pub label: Option<String>,
    pub is_base: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct PersistedState {
    pub game_path: Option<PathBuf>,
    pub mod_channel: ModChannel,
    pub installed_mod_version: Option<String>,
    pub installed_mod_checksum: Option<String>,
    pub launch_mode: LaunchMode,
    pub multi_instance: MultiInstanceState,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            game_path: None,
            mod_channel: ModChannel::Stable,
            installed_mod_version: None,
            installed_mod_checksum: None,
            launch_mode: LaunchMode::Managed,
            multi_instance: MultiInstanceState::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MultiInstanceState {
    pub enabled: bool,
    pub shared_game_root: Option<PathBuf>,
    pub instances: Vec<Instance>,
    pub base_label: Option<String>,
    pub base_last_backup_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub name: String,
    pub os_username: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_backup_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub label: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_launcher_status() {
        let status = LauncherStatus {
            game: GameStatus {
                known: true,
                path: Some("/tmp/game".into()),
                installed_version: Some(168),
                latest_version: Some(169),
                update_available: true,
            },
            mod_status: ModStatus {
                installed: false,
                installed_version: None,
                latest_version: Some("v1.2.3".into()),
                channel: ModChannel::Stable,
                update_available: true,
                launch_mode: LaunchMode::Managed,
            },
            launcher_update_available: false,
            multi_instance: MultiInstanceState::default(),
        };

        let json = serde_json::to_value(status).expect("status serializes");
        assert_eq!(json["game"]["known"], true);
        assert_eq!(json["modStatus"]["channel"], "stable");
        assert_eq!(json["modStatus"]["launchMode"], "managed");
        assert_eq!(json["multiInstance"]["enabled"], false);
    }

    #[test]
    fn persisted_state_defaults_multi_instance_when_missing() {
        let json = serde_json::json!({
            "gamePath": null,
            "modChannel": "stable",
            "installedModVersion": null,
            "installedModChecksum": null,
            "launchMode": "managed"
        });
        let state: PersistedState =
            serde_json::from_value(json).expect("loads without multiInstance");
        assert!(!state.multi_instance.enabled);
        assert!(state.multi_instance.instances.is_empty());
    }

    #[test]
    fn multi_instance_state_roundtrips_camel_case() {
        let mi = MultiInstanceState {
            enabled: true,
            shared_game_root: Some(PathBuf::from("/Users/Shared/STFC/game")),
            instances: vec![Instance {
                name: "alt2".into(),
                os_username: "stfc-alt2".into(),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-08-24T00:00:00Z")
                    .expect("date")
                    .to_utc(),
                last_backup_at: None,
                label: None,
            }],
            ..Default::default()
        };
        let json = serde_json::to_value(&mi).expect("serialize");
        assert_eq!(json["sharedGameRoot"], "/Users/Shared/STFC/game");
        assert_eq!(json["instances"][0]["osUsername"], "stfc-alt2");
        assert!(json["instances"][0]["lastBackupAt"].is_null());
        let back: MultiInstanceState = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, mi);
    }
}
