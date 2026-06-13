use crate::errors::{LauncherError, LauncherResult};
use crate::models::{LaunchMode, Platform};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrelaunchAction {
    CopyFile { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub executable: String,
    pub args: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub working_dir: Option<PathBuf>,
    pub prelaunch_actions: Vec<PrelaunchAction>,
}

pub fn build_launch_plan(
    platform: Platform,
    game_root: &Path,
    mod_library: &Path,
    launch_mode: LaunchMode,
    prime_exe: Option<&Path>,
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
                prelaunch_actions: Vec::new(),
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
                prelaunch_actions: Vec::new(),
            })
        }
        (Platform::Windows, LaunchMode::WindowsProxyDll) => Ok(LaunchPlan {
            executable: game_root.join("prime.exe").to_string_lossy().to_string(),
            args: Vec::new(),
            environment: BTreeMap::new(),
            working_dir: Some(game_root.to_path_buf()),
            prelaunch_actions: Vec::new(),
        }),
        (Platform::LinuxWine, LaunchMode::Managed) => {
            // Reuse the executable located during path resolution when available;
            // only re-scan the WINE prefix if the caller did not supply it.
            let prime_exe = match prime_exe {
                Some(prime_exe) => prime_exe.to_path_buf(),
                None => crate::game_locator::find_prime_exe_in_wine_prefix(game_root).ok_or_else(
                    || LauncherError::InvalidData {
                        context: "building launch plan".into(),
                        message: "prime.exe not found in WINE prefix".into(),
                    },
                )?,
            };
            if !mod_library.is_file() {
                return Err(LauncherError::InvalidData {
                    context: "building launch plan".into(),
                    message: format!(
                        "WINE mod library was not found at {}",
                        mod_library.display()
                    ),
                });
            }

            let wine_cmd = std::env::var("STFC_WINE_CMD").unwrap_or_else(|_| "wine".to_string());
            let prime_dir = prime_exe
                .parent()
                .ok_or_else(|| LauncherError::InvalidData {
                    context: "building launch plan".into(),
                    message: format!(
                        "prime.exe has no parent directory at {}",
                        prime_exe.display()
                    ),
                })?;

            let mut environment = BTreeMap::new();
            environment.insert("WINEDLLOVERRIDES".into(), "version=n,b".into());
            environment.insert("WINEPREFIX".into(), game_root.to_string_lossy().to_string());

            Ok(LaunchPlan {
                executable: wine_cmd,
                args: vec![prime_exe.to_string_lossy().to_string()],
                environment,
                working_dir: prime_exe.parent().map(Path::to_path_buf),
                prelaunch_actions: vec![PrelaunchAction::CopyFile {
                    from: mod_library.to_path_buf(),
                    to: prime_dir.join("version.dll"),
                }],
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
    run_prelaunch_actions(&plan.prelaunch_actions)?;

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

fn run_prelaunch_actions(actions: &[PrelaunchAction]) -> LauncherResult<()> {
    for action in actions {
        match action {
            PrelaunchAction::CopyFile { from, to } => {
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent).map_err(|err| LauncherError::Io {
                        context: format!("creating {}", parent.display()),
                        source: err,
                    })?;
                }
                fs::copy(from, to).map_err(|err| LauncherError::Io {
                    context: format!("copying {} to {}", from.display(), to.display()),
                    source: err,
                })?;
            }
        }
    }
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
            None,
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
            None,
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
            None,
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
            None,
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
        let mod_library = root.path().join("mods/version.dll");
        std::fs::create_dir_all(mod_library.parent().expect("mod parent")).expect("mod dir");
        std::fs::write(&mod_library, "mod").expect("mod library");

        let plan = build_launch_plan(
            crate::models::Platform::LinuxWine,
            game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
            None,
        )
        .expect("launch plan");

        assert_eq!(plan.executable, "wine");
        assert_eq!(plan.args.len(), 1);
        assert!(plan.args[0].ends_with("drive_c/Program Files/STFC/prime.exe"));
        assert_eq!(
            plan.environment.get("WINEDLLOVERRIDES").map(String::as_str),
            Some("version=n,b")
        );
        assert_eq!(
            plan.environment.get("WINEPREFIX").map(String::as_str),
            Some(game_root.to_string_lossy().as_ref())
        );
        assert_eq!(plan.prelaunch_actions.len(), 1);
        assert!(matches!(
            &plan.prelaunch_actions[0],
            PrelaunchAction::CopyFile { from, to }
                if from == &mod_library && to == &game_root.join("drive_c/Program Files/STFC/version.dll")
        ));
    }

    #[test]
    fn prelaunch_actions_stage_wine_mod_library() {
        let root = tempfile::tempdir().expect("tempdir");
        let source = root.path().join("managed/version.dll");
        let destination = root.path().join("drive_c/Program Files/STFC/version.dll");
        std::fs::create_dir_all(source.parent().expect("source parent")).expect("source dir");
        std::fs::create_dir_all(destination.parent().expect("destination parent"))
            .expect("destination dir");
        std::fs::write(&source, b"mod").expect("source dll");

        run_prelaunch_actions(&[PrelaunchAction::CopyFile {
            from: source,
            to: destination.clone(),
        }])
        .expect("prelaunch actions");

        assert_eq!(std::fs::read(destination).expect("staged dll"), b"mod");
    }

    #[test]
    fn linux_wine_launch_plan_reuses_provided_prime_exe() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        // No prime.exe exists under drive_c, so a fallback walk would fail; the
        // provided path must be used verbatim instead of re-scanning the prefix.
        let prime_exe = game_root.join("drive_c/Program Files/STFC/prime.exe");
        let mod_library = root.path().join("mods/version.dll");
        std::fs::create_dir_all(mod_library.parent().expect("mod parent")).expect("mod dir");
        std::fs::write(&mod_library, "mod").expect("mod library");

        let plan = build_launch_plan(
            crate::models::Platform::LinuxWine,
            game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
            Some(&prime_exe),
        )
        .expect("launch plan");

        assert_eq!(plan.args, vec![prime_exe.to_string_lossy().to_string()]);
        assert!(matches!(
            &plan.prelaunch_actions[0],
            PrelaunchAction::CopyFile { to, .. }
                if to == &prime_exe.parent().expect("prime parent").join("version.dll")
        ));
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
            None,
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
            None,
        );

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
    }
}
