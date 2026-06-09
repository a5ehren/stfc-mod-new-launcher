use crate::errors::{io_context, LauncherError, LauncherResult};
use crate::models::Platform;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherSettings {
    pub game_path: Option<String>,
    pub temp_path: Option<String>,
}

pub struct GameLocator {
    platform: Platform,
}

impl GameLocator {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn validate_manual_root(&self, path: PathBuf) -> LauncherResult<PathBuf> {
        if is_valid_game_root(&path, self.platform) {
            return Ok(path);
        }

        Err(LauncherError::InvalidData {
            context: "validating selected game folder".into(),
            message: format!("{} is not a valid STFC game folder", path.display()),
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn from_launcher_settings_file(
        &self,
        settings_file: &Path,
    ) -> LauncherResult<Option<PathBuf>> {
        match settings_file.try_exists() {
            Ok(true) => {}
            Ok(false) => return Ok(None),
            Err(err) => {
                return Err(io_context(
                    format!("checking {}", settings_file.display()),
                    err,
                ));
            }
        }

        let text = fs::read_to_string(settings_file)
            .map_err(|err| io_context(format!("reading {}", settings_file.display()), err))?;
        let parsed = parse_launcher_settings_for_platform(&text, self.platform)?;
        Ok(parsed
            .game_path
            .map(PathBuf::from)
            .filter(|path| is_valid_game_root(path, self.platform)))
    }
}

/// Parses launcher settings using macOS normalization for legacy callers and tests.
#[cfg_attr(not(test), allow(dead_code))]
pub fn parse_launcher_settings(text: &str) -> LauncherResult<LauncherSettings> {
    parse_launcher_settings_for_platform(text, Platform::MacOs)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn parse_launcher_settings_for_platform(
    text: &str,
    platform: Platform,
) -> LauncherResult<LauncherSettings> {
    let mut game_path = None;
    let mut temp_path = None;
    let mut in_general = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_general = &line[1..line.len() - 1] == "General";
            continue;
        }

        if !in_general {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "152033..GAME_PATH" => game_path = Some(normalize_xsolla_path(value, platform)),
            "152033..GAME_TEMP_PATH" => temp_path = Some(normalize_xsolla_path(value, platform)),
            _ => {}
        }
    }

    Ok(LauncherSettings {
        game_path,
        temp_path,
    })
}

#[cfg_attr(not(test), allow(dead_code))]
fn normalize_xsolla_path(value: &str, platform: Platform) -> String {
    let value = value.trim();
    match platform {
        Platform::MacOs => {
            if let Some(rest) = value.strip_prefix("//Users/") {
                return format!("/Users/{rest}");
            }
        }
        Platform::Windows => {}
        Platform::LinuxWine => {}
    }
    value.to_string()
}

pub fn is_valid_game_root(path: &Path, platform: Platform) -> bool {
    if !path.is_dir() {
        return false;
    }

    match platform {
        Platform::MacOs => path
            .join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command")
            .is_file(),
        Platform::Windows => path.join("prime.exe").is_file(),
        Platform::LinuxWine => {
            // For WINE, the game root is a WINE prefix containing drive_c/.../prime.exe
            // Common locations:
            // - Steam: <prefix>/drive_c/Program Files (x86)/Steam/steamapps/common/Star Trek Fleet Command/prime.exe
            // - Lutris: <prefix>/drive_c/Games/STFC/prime.exe or similar
            // - Bottles: <prefix>/drive_c/Program Files/STFC/prime.exe
            // We check for prime.exe anywhere under drive_c
            has_prime_exe_under_drive_c(path)
        }
    }
}

fn has_prime_exe_under_drive_c(path: &Path) -> bool {
    let drive_c = path.join("drive_c");
    if !drive_c.is_dir() {
        return false;
    }
    // Walk drive_c to find prime.exe
    fn walk_for_prime_exe(dir: &Path) -> bool {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if walk_for_prime_exe(&path) {
                        return true;
                    }
                } else if path.file_name().map(|n| n == "prime.exe").unwrap_or(false) {
                    return true;
                }
            }
        }
        false
    }
    walk_for_prime_exe(&drive_c)
}

pub fn installed_version(game_root: &Path) -> Option<u32> {
    let text = fs::read_to_string(game_root.join(".version")).ok()?;
    let (_, version) = text.split_once('=')?;
    version.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xsolla_ini_game_path() {
        let text = "[General]\n152033..GAME_PATH=//Users/test/STFC/default/game\n152033..GAME_TEMP_PATH=//Users/test/STFC/tmp\n";
        let parsed = parse_launcher_settings(text).expect("parse ini");

        assert_eq!(
            parsed.game_path.as_deref(),
            Some("/Users/test/STFC/default/game")
        );
        assert_eq!(parsed.temp_path.as_deref(), Some("/Users/test/STFC/tmp"));
    }

    #[test]
    fn macos_parser_normalizes_xsolla_users_path() {
        let text = "[General]\n152033..GAME_PATH=//Users/test/STFC/default/game\n";
        let parsed = parse_launcher_settings_for_platform(text, crate::models::Platform::MacOs)
            .expect("parse ini");

        assert_eq!(
            parsed.game_path.as_deref(),
            Some("/Users/test/STFC/default/game")
        );
    }

    #[test]
    fn windows_parser_preserves_unc_paths() {
        let server_path = "[General]\n152033..GAME_PATH=//server/share/STFC\n";
        let users_server_path = "[General]\n152033..GAME_PATH=//Users/share/STFC\n";

        let parsed_server =
            parse_launcher_settings_for_platform(server_path, crate::models::Platform::Windows)
                .expect("parse server path");
        let parsed_users = parse_launcher_settings_for_platform(
            users_server_path,
            crate::models::Platform::Windows,
        )
        .expect("parse users server path");

        assert_eq!(
            parsed_server.game_path.as_deref(),
            Some("//server/share/STFC")
        );
        assert_eq!(
            parsed_users.game_path.as_deref(),
            Some("//Users/share/STFC")
        );
    }

    #[test]
    fn preserves_unc_like_network_path() {
        let text = "[General]\n152033..GAME_PATH=//server/share/STFC\n";
        let parsed = parse_launcher_settings(text).expect("parse ini");

        assert_eq!(parsed.game_path.as_deref(), Some("//server/share/STFC"));
    }

    #[test]
    fn ignores_launcher_keys_outside_general_section() {
        let text = "152033..GAME_PATH=//Users/outside/STFC\n[Other]\n152033..GAME_TEMP_PATH=//Users/other/tmp\n[General]\n152033..GAME_PATH=//Users/inside/STFC\n";
        let parsed = parse_launcher_settings(text).expect("parse ini");

        assert_eq!(parsed.game_path.as_deref(), Some("/Users/inside/STFC"));
        assert_eq!(parsed.temp_path, None);
    }

    #[test]
    fn validates_game_root_by_platform_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::write(game_root.join(".version"), "&game=168").expect("version");
        std::fs::create_dir_all(game_root.join("Star Trek Fleet Command.app/Contents/MacOS"))
            .expect("mac dirs");
        std::fs::write(
            game_root.join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command"),
            "",
        )
        .expect("mac executable");

        assert!(is_valid_game_root(
            game_root,
            crate::models::Platform::MacOs
        ));
    }

    #[test]
    fn validates_windows_game_root_by_prime_exe_file() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::write(game_root.join("prime.exe"), "").expect("prime exe");

        assert!(is_valid_game_root(
            game_root,
            crate::models::Platform::Windows
        ));
    }

    #[test]
    fn rejects_windows_game_root_when_prime_exe_is_directory() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::create_dir(game_root.join("prime.exe")).expect("prime exe dir");

        assert!(!is_valid_game_root(
            game_root,
            crate::models::Platform::Windows
        ));
    }

    #[test]
    fn parses_installed_version_from_version_file() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::write(root.path().join(".version"), "&game=168\n").expect("version");

        assert_eq!(installed_version(root.path()), Some(168));
    }

    #[test]
    fn validate_manual_root_errors_for_invalid_path() {
        let root = tempfile::tempdir().expect("tempdir");
        let locator = GameLocator::new(crate::models::Platform::MacOs);
        let result = locator.validate_manual_root(root.path().join("missing"));

        assert!(matches!(result, Err(LauncherError::InvalidData { .. })));
    }

    #[test]
    fn missing_launcher_settings_file_returns_none() {
        let root = tempfile::tempdir().expect("tempdir");
        let locator = GameLocator::new(crate::models::Platform::MacOs);
        let result = locator
            .from_launcher_settings_file(&root.path().join("launcher_settings.ini"))
            .expect("missing settings file");

        assert_eq!(result, None);
    }

    #[test]
    fn launcher_settings_file_returns_valid_game_path() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path().join("game");
        std::fs::create_dir_all(game_root.join("Star Trek Fleet Command.app/Contents/MacOS"))
            .expect("mac dirs");
        std::fs::write(
            game_root.join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command"),
            "",
        )
        .expect("mac executable");
        let settings_file = root.path().join("launcher_settings.ini");
        std::fs::write(
            &settings_file,
            format!(
                "[General]\n152033..GAME_PATH={}\n",
                game_root.to_string_lossy()
            ),
        )
        .expect("settings");

        let locator = GameLocator::new(crate::models::Platform::MacOs);
        let result = locator
            .from_launcher_settings_file(&settings_file)
            .expect("launcher settings");

        assert_eq!(result.as_deref(), Some(game_root.as_path()));
    }

    #[test]
    fn launcher_settings_file_filters_invalid_game_path() {
        let root = tempfile::tempdir().expect("tempdir");
        let invalid_game_root = root.path().join("invalid-game");
        std::fs::create_dir(&invalid_game_root).expect("invalid game dir");
        let settings_file = root.path().join("launcher_settings.ini");
        std::fs::write(
            &settings_file,
            format!(
                "[General]\n152033..GAME_PATH={}\n",
                invalid_game_root.to_string_lossy()
            ),
        )
        .expect("settings");

        let locator = GameLocator::new(crate::models::Platform::MacOs);
        let result = locator
            .from_launcher_settings_file(&settings_file)
            .expect("launcher settings");

        assert_eq!(result, None);
    }

    #[test]
    fn launcher_settings_file_read_error_uses_io_error() {
        let root = tempfile::tempdir().expect("tempdir");
        let settings_file = root.path().join("launcher_settings.ini");
        std::fs::create_dir(&settings_file).expect("settings dir");
        let locator = GameLocator::new(crate::models::Platform::MacOs);
        let result = locator.from_launcher_settings_file(&settings_file);

        assert!(matches!(result, Err(LauncherError::Io { .. })));
    }

    #[cfg(unix)]
    #[test]
    fn launcher_settings_file_metadata_error_uses_io_error() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let settings_file = PathBuf::from(OsString::from_vec(b"bad\0path".to_vec()));
        let locator = GameLocator::new(crate::models::Platform::MacOs);
        let result = locator.from_launcher_settings_file(&settings_file);

        assert!(matches!(
            result,
            Err(LauncherError::Io { context, .. }) if context.starts_with("checking ")
        ));
    }

    #[test]
    fn validates_linux_wine_game_root_with_prime_exe_under_drive_c() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::write(game_root.join(".version"), "&game=168").expect("version");
        std::fs::create_dir_all(game_root.join("drive_c/Program Files (x86)/Steam/steamapps/common/Star Trek Fleet Command"))
            .expect("wine dirs");
        std::fs::write(
            game_root.join("drive_c/Program Files (x86)/Steam/steamapps/common/Star Trek Fleet Command/prime.exe"),
            "",
        )
        .expect("prime exe");

        assert!(is_valid_game_root(
            game_root,
            crate::models::Platform::LinuxWine
        ));
    }

    #[test]
    fn validates_linux_wine_game_root_with_prime_exe_in_lutris_layout() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::write(game_root.join(".version"), "&game=168").expect("version");
        std::fs::create_dir_all(game_root.join("drive_c/Games/STFC"))
            .expect("lutris dirs");
        std::fs::write(
            game_root.join("drive_c/Games/STFC/prime.exe"),
            "",
        )
        .expect("prime exe");

        assert!(is_valid_game_root(
            game_root,
            crate::models::Platform::LinuxWine
        ));
    }

    #[test]
    fn rejects_linux_wine_game_root_without_drive_c() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::write(game_root.join(".version"), "&game=168").expect("version");
        std::fs::create_dir_all(game_root.join("Program Files/STFC")).expect("no drive_c");
        std::fs::write(
            game_root.join("Program Files/STFC/prime.exe"),
            "",
        )
        .expect("prime exe");

        assert!(!is_valid_game_root(
            game_root,
            crate::models::Platform::LinuxWine
        ));
    }

    #[test]
    fn rejects_linux_wine_game_root_without_prime_exe() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::write(game_root.join(".version"), "&game=168").expect("version");
        std::fs::create_dir_all(game_root.join("drive_c/Program Files/STFC")).expect("wine dirs");

        assert!(!is_valid_game_root(
            game_root,
            crate::models::Platform::LinuxWine
        ));
    }
}
