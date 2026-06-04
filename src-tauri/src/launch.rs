use crate::errors::{LauncherError, LauncherResult};
use crate::models::{LaunchMode, Platform};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub executable: String,
    pub args: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub working_dir: Option<PathBuf>,
}

pub fn build_launch_plan(
    platform: Platform,
    game_root: &Path,
    mod_library: &Path,
    launch_mode: LaunchMode,
) -> LauncherResult<LaunchPlan> {
    match (platform, launch_mode) {
        (Platform::MacOs, LaunchMode::Managed) => {
            let executable = game_root
                .join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command");
            let mut environment = BTreeMap::new();
            environment.insert(
                "DYLD_INSERT_LIBRARIES".into(),
                mod_library.to_string_lossy().to_string(),
            );
            environment.insert(
                "DYLD_LIBRARY_PATH".into(),
                mod_library
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .to_string_lossy()
                    .to_string(),
            );
            Ok(LaunchPlan {
                executable: executable.to_string_lossy().to_string(),
                args: Vec::new(),
                environment,
                working_dir: executable.parent().map(Path::to_path_buf),
            })
        }
        (Platform::Windows, LaunchMode::Managed) => {
            let executable = game_root.join("prime.exe");
            let mut environment = BTreeMap::new();
            environment.insert(
                "PATH".into(),
                mod_library
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .to_string_lossy()
                    .to_string(),
            );
            Ok(LaunchPlan {
                executable: executable.to_string_lossy().to_string(),
                args: Vec::new(),
                environment,
                working_dir: Some(game_root.to_path_buf()),
            })
        }
        (Platform::Windows, LaunchMode::WindowsProxyDll) => Ok(LaunchPlan {
            executable: game_root.join("prime.exe").to_string_lossy().to_string(),
            args: Vec::new(),
            environment: BTreeMap::new(),
            working_dir: Some(game_root.to_path_buf()),
        }),
        (Platform::MacOs, LaunchMode::WindowsProxyDll) => Err(LauncherError::InvalidData {
            context: "building launch plan".into(),
            message: "Windows proxy DLL mode is not valid on macOS".into(),
        }),
    }
}

pub fn run_launch_plan(plan: &LaunchPlan) -> LauncherResult<()> {
    let mut command = Command::new(&plan.executable);
    command.args(&plan.args);
    if let Some(working_dir) = &plan.working_dir {
        command.current_dir(working_dir);
    }
    for (key, value) in &plan.environment {
        command.env(key, value);
    }
    command.spawn().map_err(|err| LauncherError::Io {
        context: format!("launching {}", plan.executable),
        source: err,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_launch_plan_uses_dylib_injection() {
        let plan = build_launch_plan(
            crate::models::Platform::MacOs,
            std::path::Path::new("/game"),
            std::path::Path::new("/mods/libstfc-community-mod.dylib"),
            crate::models::LaunchMode::Managed,
        )
        .expect("launch plan");

        assert_eq!(
            plan.executable,
            "/game/Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command"
        );
        assert_eq!(
            plan.environment
                .get("DYLD_INSERT_LIBRARIES")
                .map(String::as_str),
            Some("/mods/libstfc-community-mod.dylib")
        );
    }

    #[test]
    fn windows_fallback_uses_prime_exe() {
        let plan = build_launch_plan(
            crate::models::Platform::Windows,
            std::path::Path::new("C:/Games/STFC/game"),
            std::path::Path::new("C:/Games/STFC/game/version.dll"),
            crate::models::LaunchMode::WindowsProxyDll,
        )
        .expect("launch plan");

        assert!(plan.executable.ends_with("prime.exe"));
        assert!(plan.environment.is_empty());
    }
}
