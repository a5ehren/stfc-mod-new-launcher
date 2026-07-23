use crate::errors::{LauncherError, LauncherResult};
use crate::models::{LaunchMode, Platform};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Non-Steam app id passed to raw Proton so it treats the launch as a
/// standalone (non-Steam-client) game. Uses the STFC Xsolla project id as a
/// stable, project-specific placeholder; Proton accepts arbitrary non-Steam ids.
const STFC_NONSTEAM_APPID: &str = "152033";

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

/// The Linux/WINE runner used to launch the game. Discovered from PATH at
/// launch time (see [`resolve_wine_runner`]); overridable via `STFC_WINE_CMD`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WineRunner {
    /// [umu-launcher](https://github.com/Open-Wine-Components/umu-launcher)
    /// (`umu-run`): a generic, Steam-independent Proton wrapper. Invoked as
    /// `<command> <exe>` and uses `WINEPREFIX`, exactly like plain wine, so it
    /// is preferred when present. `proton_path` optionally points umu at a
    /// specific Proton via `PROTONPATH` (discovered, not downloaded).
    Umu {
        command: String,
        proton_path: Option<PathBuf>,
    },
    /// Raw Proton (`proton`): Steam's runner. Invoked as `<command> run <exe>` and
    /// uses `STEAM_COMPAT_DATA_PATH` (the WINE prefix root) rather than
    /// `WINEPREFIX`. Steam-coupled; best-effort for non-Steam launches.
    Proton { command: String },
    /// Plain wine (`wine`): invoked as `<command> <exe>` with `WINEPREFIX`.
    Wine { command: String },
}

impl WineRunner {
    /// Short human-readable label for diagnostics logging.
    pub fn label(&self) -> &'static str {
        match self {
            WineRunner::Umu { .. } => "umu-run",
            WineRunner::Proton { .. } => "proton",
            WineRunner::Wine { .. } => "wine",
        }
    }

    /// Classifies a command path/name into a runner by its basename: `umu*` →
    /// [`Umu`](Self::Umu), `proton` → [`Proton`](Self::Proton), else
    /// [`Wine`](Self::Wine).
    fn from_command(command: String) -> Self {
        let basename = Path::new(&command)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        if basename.contains("umu") {
            Self::Umu {
                command,
                proton_path: None,
            }
        } else if basename == "proton" {
            Self::Proton { command }
        } else {
            Self::Wine { command }
        }
    }
}

/// Resolves the Linux/WINE runner to use for launching.
///
/// Priority:
/// 1. `STFC_WINE_CMD` — explicit override (classified by basename so a `proton`
///    or `umu-run` path gets the right invocation).
/// 2. PATH search for `umu-run`, then `umu-launcher`, then `proton`, then `wine`.
/// 3. Falls back to `wine` (yields a clear spawn error if nothing is installed).
///
/// For the umu runner, a Proton directory is discovered (see
/// [`discover_proton_path`]) and passed via `PROTONPATH` so an existing Proton
/// (e.g. Heroic's GE-Proton) is reused instead of triggering a UMU-Proton
/// download.
pub fn resolve_wine_runner() -> WineRunner {
    if let Some(command) = std::env::var("STFC_WINE_CMD")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        let runner = WineRunner::from_command(command);
        return match runner {
            WineRunner::Umu { command, .. } => WineRunner::Umu {
                command,
                proton_path: discover_proton_path(),
            },
            other => other,
        };
    }

    for candidate in ["umu-run", "umu-launcher", "proton", "wine"] {
        if let Some(path) = which(candidate) {
            let command = path.to_string_lossy().to_string();
            return match WineRunner::from_command(command) {
                WineRunner::Umu { command, .. } => WineRunner::Umu {
                    command,
                    proton_path: discover_proton_path(),
                },
                other => other,
            };
        }
    }

    // Nothing usable on PATH: fall back to the bare `wine` name so the spawn
    // error is self-explanatory rather than a silent no-op.
    WineRunner::Wine {
        command: "wine".into(),
    }
}

/// Searches `$PATH` for an executable named `name`, returning its full path.
fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        let is_executable = candidate.is_file() && is_executable_file(&candidate);
        if is_executable {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(_path: &Path) -> bool {
    // On non-Unix targets the LinuxWine launch arm is dead code; a plain
    // file check is sufficient for the (Windows/macOS) build to compile.
    true
}

/// Discovers a Proton directory for the umu runner to reuse via `PROTONPATH`,
/// avoiding a UMU-Proton download on first launch. A user-set `PROTONPATH` env
/// var is respected (and propagated to the child unchanged) by returning
/// `None` here; otherwise standard Proton install locations are scanned and the
/// most recently modified Proton directory is chosen.
fn discover_proton_path() -> Option<PathBuf> {
    if std::env::var_os("PROTONPATH").is_some() {
        return None;
    }
    let home = directories::BaseDirs::new()?.home_dir().to_path_buf();

    let mut candidates: Vec<PathBuf> = Vec::new();
    // Steam custom Proton tools (GE-Proton, UMU-Proton, etc.).
    if let Ok(entries) = std::fs::read_dir(home.join(".local/share/Steam/compatibilitytools.d")) {
        for entry in entries.flatten() {
            candidates.push(entry.path());
        }
    }
    // Heroic's Proton tools.
    if let Ok(entries) = std::fs::read_dir(home.join(".config/heroic/tools/proton")) {
        for entry in entries.flatten() {
            candidates.push(entry.path());
        }
    }

    candidates
        .into_iter()
        .filter(|path| path.is_dir() && is_proton_dir(path))
        .max_by_key(|path| {
            std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .map(|time| {
                    time.duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                })
                .unwrap_or(0)
        })
}

/// A directory counts as a Proton install if it contains a `proton` executable
/// (the entry point both Steam Proton and GE-Proton ship).
fn is_proton_dir(path: &Path) -> bool {
    path.join("proton").is_file()
}

pub fn build_launch_plan(
    platform: Platform,
    game_root: &Path,
    mod_library: &Path,
    launch_mode: LaunchMode,
    wine_runner: &WineRunner,
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
            // game_root is the game folder containing prime.exe. prime.exe is
            // passed in from path resolution when available; otherwise it lives
            // directly in the game folder. The WINE prefix (the ancestor holding
            // drive_c) is derived for WINEPREFIX.
            let prime_exe = match prime_exe {
                Some(prime_exe) => prime_exe.to_path_buf(),
                None => game_root.join("prime.exe"),
            };
            if !prime_exe.is_file() {
                return Err(LauncherError::InvalidData {
                    context: "building launch plan".into(),
                    message: format!("prime.exe not found at {}", prime_exe.display()),
                });
            }
            if !mod_library.is_file() {
                return Err(LauncherError::InvalidData {
                    context: "building launch plan".into(),
                    message: format!(
                        "WINE mod library was not found at {}",
                        mod_library.display()
                    ),
                });
            }

            let wine_prefix =
                crate::game_locator::find_wine_prefix(game_root).ok_or_else(|| {
                    LauncherError::InvalidData {
                        context: "building launch plan".into(),
                        message: format!(
                            "could not derive WINE prefix from {} (no drive_c ancestor found)",
                            game_root.display()
                        ),
                    }
                })?;
            let prime_dir = prime_exe
                .parent()
                .ok_or_else(|| LauncherError::InvalidData {
                    context: "building launch plan".into(),
                    message: format!(
                        "prime.exe has no parent directory at {}",
                        prime_exe.display()
                    ),
                })?;

            // All runner types inject the mod library as version.dll next to
            // prime.exe and force it to load native-then-builtin.
            let mut environment = BTreeMap::new();
            environment.insert("WINEDLLOVERRIDES".into(), "version=n,b".into());

            let (executable, args) = match wine_runner {
                WineRunner::Umu {
                    command,
                    proton_path,
                } => {
                    // umu-run behaves like wine (WINEPREFIX + `<exe>`), but wraps
                    // a Proton. Point it at a discovered Proton so it reuses an
                    // existing install instead of downloading UMU-Proton.
                    if let Some(proton) = proton_path {
                        environment
                            .insert("PROTONPATH".into(), proton.to_string_lossy().to_string());
                    }
                    environment.insert(
                        "WINEPREFIX".into(),
                        wine_prefix.to_string_lossy().to_string(),
                    );
                    (
                        command.clone(),
                        vec![prime_exe.to_string_lossy().to_string()],
                    )
                }
                WineRunner::Proton { command } => {
                    // Raw Proton: invoked as `proton run <exe>` and keyed off
                    // STEAM_COMPAT_DATA_PATH (the prefix root) rather than
                    // WINEPREFIX. Non-Steam app id so Proton treats this as a
                    // standalone launch.
                    environment.insert(
                        "STEAM_COMPAT_DATA_PATH".into(),
                        wine_prefix.to_string_lossy().to_string(),
                    );
                    environment.insert("SteamAppId".into(), STFC_NONSTEAM_APPID.into());
                    environment.insert("SteamGameId".into(), STFC_NONSTEAM_APPID.into());
                    (
                        command.clone(),
                        vec!["run".into(), prime_exe.to_string_lossy().to_string()],
                    )
                }
                WineRunner::Wine { command } => {
                    environment.insert(
                        "WINEPREFIX".into(),
                        wine_prefix.to_string_lossy().to_string(),
                    );
                    (
                        command.clone(),
                        vec![prime_exe.to_string_lossy().to_string()],
                    )
                }
            };

            Ok(LaunchPlan {
                executable,
                args,
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

    /// Default runner for launch-plan tests: plain `wine` with `WINEPREFIX`, so
    /// the existing assertions stay deterministic regardless of what is on PATH.
    fn wine_runner() -> WineRunner {
        WineRunner::Wine {
            command: "wine".into(),
        }
    }

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
            &wine_runner(),
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
            &wine_runner(),
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
            &wine_runner(),
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
            &wine_runner(),
            None,
        )
        .expect("launch plan");

        assert!(plan.executable.ends_with("prime.exe"));
        assert!(plan.environment.is_empty());
    }

    #[test]
    fn linux_wine_launch_plan_uses_wine_with_dll_override() {
        let root = tempfile::tempdir().expect("tempdir");
        let prefix = root.path().join("heroic-prefix");
        let game_root = prefix.join("drive_c/Games/STFC");
        std::fs::create_dir_all(&game_root).expect("game dirs");
        std::fs::write(game_root.join("prime.exe"), "").expect("prime exe");
        let mod_library = root.path().join("mods/version.dll");
        std::fs::create_dir_all(mod_library.parent().expect("mod parent")).expect("mod dir");
        std::fs::write(&mod_library, "mod").expect("mod library");

        let plan = build_launch_plan(
            crate::models::Platform::LinuxWine,
            &game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
            &wine_runner(),
            None,
        )
        .expect("launch plan");

        assert_eq!(plan.executable, "wine");
        assert_eq!(plan.args.len(), 1);
        assert!(plan.args[0].ends_with("drive_c/Games/STFC/prime.exe"));
        assert_eq!(
            plan.environment.get("WINEDLLOVERRIDES").map(String::as_str),
            Some("version=n,b")
        );
        assert_eq!(
            plan.environment.get("WINEPREFIX").map(String::as_str),
            Some(prefix.to_string_lossy().as_ref())
        );
        assert_eq!(plan.prelaunch_actions.len(), 1);
        assert!(matches!(
            &plan.prelaunch_actions[0],
            PrelaunchAction::CopyFile { from, to }
                if from == &mod_library && to == &game_root.join("version.dll")
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
        let prefix = root.path().join("heroic-prefix");
        let game_root = prefix.join("drive_c/Games/STFC");
        // game_root deliberately has no prime.exe of its own; the caller supplies
        // a prime.exe elsewhere under drive_c and it must be used verbatim rather
        // than falling back to game_root/prime.exe.
        let prime_exe = prefix.join("drive_c/Program Files/STFC/prime.exe");
        std::fs::create_dir_all(prime_exe.parent().expect("prime parent")).expect("prime dir");
        std::fs::write(&prime_exe, "").expect("prime exe");
        let mod_library = root.path().join("mods/version.dll");
        std::fs::create_dir_all(mod_library.parent().expect("mod parent")).expect("mod dir");
        std::fs::write(&mod_library, "mod").expect("mod library");

        let plan = build_launch_plan(
            crate::models::Platform::LinuxWine,
            &game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
            &wine_runner(),
            Some(&prime_exe),
        )
        .expect("launch plan");

        assert_eq!(plan.args, vec![prime_exe.to_string_lossy().to_string()]);
        assert_eq!(
            plan.environment.get("WINEPREFIX").map(String::as_str),
            Some(prefix.to_string_lossy().as_ref())
        );
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
            &wine_runner(),
            None,
        );

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
    }

    #[test]
    fn linux_wine_errors_when_prime_exe_not_found() {
        let root = tempfile::tempdir().expect("tempdir");
        let prefix = root.path().join("heroic-prefix");
        let game_root = prefix.join("drive_c/Games/STFC");
        std::fs::create_dir_all(&game_root).expect("game dirs");
        // No prime.exe

        let result = build_launch_plan(
            crate::models::Platform::LinuxWine,
            &game_root,
            std::path::Path::new("/mods/version.dll"),
            crate::models::LaunchMode::Managed,
            &wine_runner(),
            None,
        );

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
    }

    #[test]
    fn linux_wine_errors_when_wine_prefix_not_derivable() {
        let root = tempfile::tempdir().expect("tempdir");
        // A game folder with no drive_c ancestor: prime.exe and the mod library
        // exist, but the WINE prefix cannot be derived.
        let game_root = root.path().join("plain/game");
        std::fs::create_dir_all(&game_root).expect("game dirs");
        std::fs::write(game_root.join("prime.exe"), "").expect("prime exe");
        let mod_library = root.path().join("mods/version.dll");
        std::fs::create_dir_all(mod_library.parent().expect("mod parent")).expect("mod dir");
        std::fs::write(&mod_library, "mod").expect("mod library");

        let result = build_launch_plan(
            crate::models::Platform::LinuxWine,
            &game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
            &wine_runner(),
            None,
        );

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
    }

    #[test]
    fn wine_runner_from_command_classifies_by_basename() {
        assert_eq!(
            WineRunner::from_command("umu-run".to_string()),
            WineRunner::Umu {
                command: "umu-run".into(),
                proton_path: None,
            }
        );
        assert_eq!(
            WineRunner::from_command("/usr/bin/umu-launcher".to_string()),
            WineRunner::Umu {
                command: "/usr/bin/umu-launcher".into(),
                proton_path: None,
            }
        );
        assert_eq!(
            WineRunner::from_command("/opt/GE-Proton10-34/proton".to_string()),
            WineRunner::Proton {
                command: "/opt/GE-Proton10-34/proton".into(),
            }
        );
        assert_eq!(
            WineRunner::from_command("wine".to_string()),
            WineRunner::Wine {
                command: "wine".into(),
            }
        );
        assert_eq!(
            WineRunner::from_command("/usr/bin/wine64".to_string()),
            WineRunner::Wine {
                command: "/usr/bin/wine64".into(),
            }
        );
    }

    #[test]
    fn is_proton_dir_detects_proton_executable() {
        let root = tempfile::tempdir().expect("tempdir");
        assert!(!is_proton_dir(root.path()));

        std::fs::write(root.path().join("proton"), "#!/bin/sh\n").expect("proton");
        assert!(is_proton_dir(root.path()));
    }

    #[cfg(unix)]
    #[test]
    fn is_executable_file_checks_exec_bit() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().expect("tempdir");
        let file = root.path().join("bin");
        std::fs::write(&file, "").expect("file");

        assert!(!is_executable_file(&file));

        let mut perms = std::fs::metadata(&file).expect("metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&file, perms).expect("chmod");

        assert!(is_executable_file(&file));
    }

    #[test]
    fn linux_wine_launch_plan_uses_umu_runner() {
        let root = tempfile::tempdir().expect("tempdir");
        let prefix = root.path().join("heroic-prefix");
        let game_root = prefix.join("drive_c/Games/STFC");
        std::fs::create_dir_all(&game_root).expect("game dirs");
        std::fs::write(game_root.join("prime.exe"), "").expect("prime exe");
        let mod_library = root.path().join("mods/version.dll");
        std::fs::create_dir_all(mod_library.parent().expect("mod parent")).expect("mod dir");
        std::fs::write(&mod_library, "mod").expect("mod library");
        let proton_dir = root.path().join("GE-Proton10-34");
        std::fs::create_dir_all(&proton_dir).expect("proton dir");
        std::fs::write(proton_dir.join("proton"), "").expect("proton");

        let plan = build_launch_plan(
            crate::models::Platform::LinuxWine,
            &game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
            &WineRunner::Umu {
                command: "umu-run".into(),
                proton_path: Some(proton_dir.clone()),
            },
            None,
        )
        .expect("launch plan");

        assert_eq!(plan.executable, "umu-run");
        assert_eq!(plan.args.len(), 1);
        assert!(plan.args[0].ends_with("drive_c/Games/STFC/prime.exe"));
        assert_eq!(
            plan.environment.get("WINEPREFIX").map(String::as_str),
            Some(prefix.to_string_lossy().as_ref())
        );
        assert_eq!(
            plan.environment.get("PROTONPATH").map(String::as_str),
            Some(proton_dir.to_string_lossy().as_ref())
        );
        assert_eq!(
            plan.environment.get("WINEDLLOVERRIDES").map(String::as_str),
            Some("version=n,b")
        );
        assert!(!plan.environment.contains_key("STEAM_COMPAT_DATA_PATH"));
        assert_eq!(plan.prelaunch_actions.len(), 1);
        assert!(matches!(
            &plan.prelaunch_actions[0],
            PrelaunchAction::CopyFile { from, to }
                if from == &mod_library && to == &game_root.join("version.dll")
        ));
    }

    #[test]
    fn linux_wine_launch_plan_umu_without_proton_path_omits_protonpath() {
        let root = tempfile::tempdir().expect("tempdir");
        let prefix = root.path().join("heroic-prefix");
        let game_root = prefix.join("drive_c/Games/STFC");
        std::fs::create_dir_all(&game_root).expect("game dirs");
        std::fs::write(game_root.join("prime.exe"), "").expect("prime exe");
        let mod_library = root.path().join("mods/version.dll");
        std::fs::create_dir_all(mod_library.parent().expect("mod parent")).expect("mod dir");
        std::fs::write(&mod_library, "mod").expect("mod library");

        let plan = build_launch_plan(
            crate::models::Platform::LinuxWine,
            &game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
            &WineRunner::Umu {
                command: "umu-run".into(),
                proton_path: None,
            },
            None,
        )
        .expect("launch plan");

        assert!(!plan.environment.contains_key("PROTONPATH"));
        assert_eq!(
            plan.environment.get("WINEPREFIX").map(String::as_str),
            Some(prefix.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn linux_wine_launch_plan_uses_proton_runner() {
        let root = tempfile::tempdir().expect("tempdir");
        let prefix = root.path().join("heroic-prefix");
        let game_root = prefix.join("drive_c/Games/STFC");
        std::fs::create_dir_all(&game_root).expect("game dirs");
        std::fs::write(game_root.join("prime.exe"), "").expect("prime exe");
        let mod_library = root.path().join("mods/version.dll");
        std::fs::create_dir_all(mod_library.parent().expect("mod parent")).expect("mod dir");
        std::fs::write(&mod_library, "mod").expect("mod library");

        let plan = build_launch_plan(
            crate::models::Platform::LinuxWine,
            &game_root,
            &mod_library,
            crate::models::LaunchMode::Managed,
            &WineRunner::Proton {
                command: "/opt/proton".into(),
            },
            None,
        )
        .expect("launch plan");

        assert_eq!(plan.executable, "/opt/proton");
        assert_eq!(
            plan.args,
            vec![
                "run".to_string(),
                game_root.join("prime.exe").to_string_lossy().to_string()
            ]
        );
        assert_eq!(
            plan.environment
                .get("STEAM_COMPAT_DATA_PATH")
                .map(String::as_str),
            Some(prefix.to_string_lossy().as_ref())
        );
        assert_eq!(
            plan.environment.get("SteamAppId").map(String::as_str),
            Some(STFC_NONSTEAM_APPID)
        );
        assert_eq!(
            plan.environment.get("SteamGameId").map(String::as_str),
            Some(STFC_NONSTEAM_APPID)
        );
        assert_eq!(
            plan.environment.get("WINEDLLOVERRIDES").map(String::as_str),
            Some("version=n,b")
        );
        assert!(!plan.environment.contains_key("WINEPREFIX"));
    }
}
