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

/// Finds prime.exe under drive_c in a WINE prefix
fn find_prime_exe_in_wine_prefix(wine_prefix: &Path) -> Option<PathBuf> {
    let drive_c = wine_prefix.join("drive_c");
    if !drive_c.is_dir() {
        return None;
    }
    fn walk_for_prime_exe(dir: &Path) -> Option<PathBuf> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = walk_for_prime_exe(&path) {
                        return Some(found);
                    }
                } else if path.file_name().map(|n| n == "prime.exe").unwrap_or(false) {
                    return Some(path);
                }
            }
        }
        None
    }
    walk_for_prime_exe(&drive_c)
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
            if !executable.is_file() {
                return Err(LauncherError::InvalidData {
                    context: "building launch plan".into(),
                    message: format!(
                        "macOS game executable was not found at {}",
                        executable.display()
                    ),
                });
            }
            if !mod_library.is_file() {
                return Err(LauncherError::InvalidData {
                    context: "building launch plan".into(),
                    message: format!(
                        "macOS mod library was not found at {}",
                        mod_library.display()
                    ),
                });
            }
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
        (Platform::LinuxWine, LaunchMode::Managed) => {
            // Find prime.exe in the WINE prefix
            let prime_exe = find_prime_exe_in_wine_prefix(game_root).ok_or_else(|| {
                LauncherError::InvalidData {
                    context: "building launch plan".into(),
                    message: "prime.exe not found in WINE prefix".into(),
                }
            })?;

            // Determine wine command (could be wine, wine64, proton, etc.)
            let wine_cmd = std::env::var("STFC_WINE_CMD").unwrap_or_else(|_| "wine".to_string());

            let mut environment = BTreeMap::new();
            // Use WINEDLLOVERRIDES to inject our version.dll
            environment.insert(
                "WINEDLLOVERRIDES".into(),
                format!("version=n,b;{}", mod_library.to_string_lossy()),
            );
            // Set WINEPREFIX
            environment.insert("WINEPREFIX".into(), game_root.to_string_lossy().to_string());

            Ok(LaunchPlan {
                executable: wine_cmd,
                args: vec![prime_exe.to_string_lossy().to_string()],
                environment,
                working_dir: prime_exe.parent().map(Path::to_path_buf),
            })
        }
        (Platform::LinuxWine, LaunchMode::WindowsProxyDll) => Err(LauncherError::InvalidData {
            context: "building launch plan".into(),
            message: "Windows proxy DLL mode is not valid on Linux/WINE".into(),
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
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::create_dir_all(game_root.join("Star Trek Fleet Command.app/Contents/MacOS"))
            .expect("mac dirs");
        std::fs::write(
            game_root.join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command"),
            "",
        )
        .expect("mac executable");
        let mod_library = game_root.join("libstfc-community-mod.dylib");
        std::fs::write(&mod_library, "").expect("mod library");

        let plan = build_launch_plan(
            crate::models::Platform::MacOs,
            game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
        )
        .expect("launch plan");

        assert_eq!(
            plan.executable,
            game_root
                .join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command")
                .to_string_lossy()
        );
        assert_eq!(
            plan.working_dir,
            Some(game_root.join("Star Trek Fleet Command.app/Contents/MacOS"))
        );
        assert_eq!(
            plan.environment
                .get("DYLD_INSERT_LIBRARIES")
                .map(String::as_str),
            Some(mod_library.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn mac_launch_plan_rejects_missing_executable() {
        let root = tempfile::tempdir().expect("tempdir");
        let mod_library = root.path().join("libstfc-community-mod.dylib");
        std::fs::write(&mod_library, "").expect("mod library");

        let result = build_launch_plan(
            crate::models::Platform::MacOs,
            root.path(),
            &mod_library,
            crate::models::LaunchMode::Managed,
        );

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
    }

    #[test]
    fn mac_launch_plan_rejects_missing_mod_library() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::create_dir_all(game_root.join("Star Trek Fleet Command.app/Contents/MacOS"))
            .expect("mac dirs");
        std::fs::write(
            game_root.join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command"),
            "",
        )
        .expect("mac executable");

        let result = build_launch_plan(
            crate::models::Platform::MacOs,
            game_root,
            &game_root.join("libstfc-community-mod.dylib"),
            crate::models::LaunchMode::Managed,
        );

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
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

    #[test]
    fn linux_wine_launch_plan_uses_wine_with_dll_override() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::create_dir_all(game_root.join("drive_c/Program Files/STFC")).expect("wine dirs");
        std::fs::write(game_root.join("drive_c/Program Files/STFC/prime.exe"), "")
            .expect("prime exe");

        let plan = build_launch_plan(
            crate::models::Platform::LinuxWine,
            game_root,
            std::path::Path::new("/mods/version.dll"),
            crate::models::LaunchMode::Managed,
        )
        .expect("launch plan");

        assert_eq!(plan.executable, "wine");
        assert_eq!(plan.args.len(), 1);
        assert!(plan.args[0].ends_with("drive_c/Program Files/STFC/prime.exe"));
        assert_eq!(
            plan.environment.get("WINEDLLOVERRIDES").map(String::as_str),
            Some("version=n,b;/mods/version.dll")
        );
        assert_eq!(
            plan.environment.get("WINEPREFIX").map(String::as_str),
            Some(game_root.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn linux_wine_rejects_windows_proxy_dll_mode() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();

        let result = build_launch_plan(
            crate::models::Platform::LinuxWine,
            game_root,
            std::path::Path::new("/mods/version.dll"),
            crate::models::LaunchMode::WindowsProxyDll,
        );

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
    }

    #[test]
    fn linux_wine_errors_when_prime_exe_not_found() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::create_dir_all(game_root.join("drive_c/Program Files/STFC")).expect("wine dirs");
        // No prime.exe

        let result = build_launch_plan(
            crate::models::Platform::LinuxWine,
            game_root,
            std::path::Path::new("/mods/version.dll"),
            crate::models::LaunchMode::Managed,
        );

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
    }
}
