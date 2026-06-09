use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    MacOs,
    Windows,
    LinuxWine,
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
        Platform::LinuxWine
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
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            game_path: None,
            mod_channel: ModChannel::Stable,
            installed_mod_version: None,
            installed_mod_checksum: None,
            launch_mode: LaunchMode::Managed,
        }
    }
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
        };

        let json = serde_json::to_value(status).expect("status serializes");
        assert_eq!(json["game"]["known"], true);
        assert_eq!(json["modStatus"]["channel"], "stable");
        assert_eq!(json["modStatus"]["launchMode"], "managed");
    }
}
