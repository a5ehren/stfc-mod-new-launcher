//! Per-instance game-account backup & restore (FR-6). Backups contain live
//! account credentials — they are owner-only on disk (0700/0600).
#![allow(dead_code)]

use crate::errors::{LauncherError, LauncherResult};
use crate::storage::ManagedPaths;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn backup_dir(paths: &ManagedPaths, name: &str) -> PathBuf {
    paths.root.join("backups").join(name)
}

pub fn latest_backup(paths: &ManagedPaths, name: &str) -> Option<PathBuf> {
    let dir = backup_dir(paths, name);
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .max() // ISO-8601 dir names sort lexicographically
}

/// The sudo run-as prefix is only needed for cross-user access; backing up
/// the base account (ourselves) runs the tools directly.
fn sudo_prefix(username: &str) -> Vec<String> {
    if username == crate::instance_users::current_username().unwrap_or_default() {
        vec!["env".into()]
    } else {
        vec!["sudo".into(), "-Hu".into(), username.into(), "env".into()]
    }
}

/// Command argv lists; each writes to STDOUT so the launcher (primary user)
/// owns the resulting backup files directly — no cross-user file handoff.
/// `defaults export … -` and `tar -cf -` both stream to stdout. Cross-user
/// runs go through the scoped sudoers prefix; the base account runs direct.
pub fn macos_backup_commands(username: &str, home: &Path) -> Vec<Vec<String>> {
    let prefix = || sudo_prefix(username);
    let mut defaults = prefix();
    defaults.extend([
        "/usr/bin/defaults".into(),
        "export".into(),
        "com.scopely.startrek".into(),
        "-".into(),
    ]);
    let mut tar = prefix();
    tar.extend([
        "/usr/bin/tar".into(),
        "-cf".into(),
        "-".into(),
        "-C".into(),
        home.join("Library/Application Support/com.scopely.startrek")
            .to_string_lossy()
            .to_string(),
        ".".into(),
    ]);
    vec![defaults, tar]
}

/// Restore argv lists; stdin carries the archive/plist contents. Cross-user
/// runs go through the scoped sudoers entry (env(1) runs the real tool,
/// including /bin/sh for the staged plist import); the base account runs
/// the tools directly as the current user.
pub fn macos_restore_commands(username: &str, home: &Path) -> Vec<Vec<String>> {
    let prefix = || sudo_prefix(username);
    // Freshly re-provisioned instances have no session dir yet — tar -C fails
    // on a missing target, which is exactly the lost-account recovery path.
    let mut mkdir = prefix();
    mkdir.extend([
        "/bin/mkdir".into(),
        "-p".into(),
        home.join("Library/Application Support/com.scopely.startrek")
            .to_string_lossy()
            .to_string(),
    ]);
    let mut tar = prefix();
    tar.extend([
        "/usr/bin/tar".into(),
        "-xf".into(),
        "-".into(),
        "-C".into(),
        home.join("Library/Application Support/com.scopely.startrek")
            .to_string_lossy()
            .to_string(),
    ]);
    let mut import = prefix();
    import.extend([
        "/bin/sh".into(),
        "-c".into(),
        "cat > \"$HOME/.stfc-restore.plist\" && /usr/bin/defaults import com.scopely.startrek \"$HOME/.stfc-restore.plist\" && rm -f \"$HOME/.stfc-restore.plist\"".into(),
    ]);
    vec![mkdir, tar, import]
}

/// macOS: `dscl . -read /Users/<u> NFSHomeDirectory` (world-readable, no sudo).
#[cfg(target_os = "macos")]
pub fn service_user_home(username: &str) -> LauncherResult<PathBuf> {
    let out = Command::new("dscl")
        .args([
            ".",
            "-read",
            &format!("/Users/{username}"),
            "NFSHomeDirectory",
        ])
        .output()
        .map_err(|e| LauncherError::Io {
            context: "reading home dir".into(),
            source: e,
        })?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .strip_prefix("NFSHomeDirectory: ")
        .map(PathBuf::from)
        .ok_or_else(|| LauncherError::InvalidData {
            context: "reading home dir".into(),
            message: format!("unexpected dscl output: {text}"),
        })
}

/// Windows: local user profiles live at `C:\Users\<name>`.
#[cfg(target_os = "windows")]
pub fn service_user_home(username: &str) -> LauncherResult<PathBuf> {
    Ok(PathBuf::from(format!(r"C:\Users\{username}")))
}

/// Stops the instance first (FR-6.2), then restores from the latest backup.
fn stop_before_restore(username: &str) -> LauncherResult<()> {
    crate::instance_launch::stop_instance(crate::models::current_platform(), username)
}

/// Returns the timestamped backup directory. Backups hold account
/// credentials: dir 0700, files 0600 (spec FR-6.3).
#[cfg(target_os = "macos")]
pub fn backup_instance(
    paths: &ManagedPaths,
    name: &str,
    username: &str,
) -> LauncherResult<PathBuf> {
    let home = service_user_home(username)?;
    let stamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
    let out_dir = backup_dir(paths, name).join(&stamp);
    create_private_dir(&out_dir)?;
    let files = ["account.plist", "sessions.tar"];
    for (argv, file) in macos_backup_commands(username, &home)
        .into_iter()
        .zip(files)
    {
        let out = std::fs::File::create(out_dir.join(file)).map_err(|e| LauncherError::Io {
            context: format!("creating {file}"),
            source: e,
        })?;
        let status = Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(Stdio::from(out))
            .status()
            .map_err(|e| LauncherError::Io {
                context: format!("running {}", argv[0]),
                source: e,
            })?;
        if !status.success() {
            return Err(LauncherError::InvalidData {
                context: "backup".into(),
                message: format!("{} exited {status}", argv.join(" ")),
            });
        }
        set_owner_only(&out_dir.join(file))?;
    }
    Ok(out_dir)
}

/// Creates a directory and immediately locks it to 0700, before any
/// credential file lands in it (FR-6.3). Errors are propagated — backup
/// permissions must never silently degrade.
#[cfg(unix)]
fn create_private_dir(dir: &Path) -> LauncherResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).map_err(|e| LauncherError::Io {
        context: "creating backup dir".into(),
        source: e,
    })?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| {
        LauncherError::Io {
            context: format!("locking {}", dir.display()),
            source: e,
        }
    })?;
    Ok(())
}

/// Locks a single credential file to 0600 (FR-6.3). Propagates errors.
#[cfg(unix)]
fn set_owner_only(file: &Path) -> LauncherResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600)).map_err(|e| {
        LauncherError::Io {
            context: format!("locking {}", file.display()),
            source: e,
        }
    })
}

/// Restore: stops the instance, then puts the account record back via
/// `defaults import` from a file staged into the user's home (never direct
/// plist edits — cfprefsd); session files extract in place. All as the
/// service user, so ownership is correct.
#[cfg(target_os = "macos")]
pub fn restore_instance(paths: &ManagedPaths, name: &str, username: &str) -> LauncherResult<()> {
    let backup = latest_backup(paths, name).ok_or_else(|| LauncherError::InvalidData {
        context: "restore".into(),
        message: format!("no backup found for instance {name}"),
    })?;
    stop_before_restore(username)?;
    let home = service_user_home(username)?;
    // mkdir carries no backup file; tar/import read sessions.tar/account.plist.
    let inputs = [None, Some("sessions.tar"), Some("account.plist")];
    for (argv, file) in macos_restore_commands(username, &home)
        .into_iter()
        .zip(inputs)
    {
        let mut command = Command::new(&argv[0]);
        command.args(&argv[1..]);
        if let Some(file) = file {
            let input = std::fs::File::open(backup.join(file)).map_err(|e| LauncherError::Io {
                context: format!("opening {file}"),
                source: e,
            })?;
            command.stdin(Stdio::from(input));
        }
        let status = command.status().map_err(|e| LauncherError::Io {
            context: format!("running {}", argv[0]),
            source: e,
        })?;
        if !status.success() {
            return Err(LauncherError::InvalidData {
                context: "restore".into(),
                message: format!("{} exited {status}", argv.join(" ")),
            });
        }
    }
    Ok(())
}

/// Windows v1: backup requires the instance RUNNING — the service user's
/// registry hive is only loaded while it has a live process (spec FR-6.1,
/// plan Task 8). `reg.exe export` and the session-file copy run as the
/// service user via the stored credential, writing to a Public staging dir;
/// the launcher then moves results into the backup dir and ACLs it
/// owner-only. run_as_user gives us a pid, not an exit code, so success is
/// detected by polling for the expected output (ponytail ceiling: a tool
/// that hangs is reported as failure after the timeout, not diagnosed).
#[cfg(target_os = "windows")]
pub fn backup_instance(
    paths: &ManagedPaths,
    name: &str,
    username: &str,
) -> LauncherResult<PathBuf> {
    use crate::instance_launch::{instance_pid, process_matcher, run_as_user};
    use crate::models::Platform;
    if instance_pid(username, process_matcher(Platform::Windows))?.is_none() {
        return Err(LauncherError::InvalidData {
            context: "backup".into(),
            message: "Windows backup requires the instance to be running (registry hive loaded)"
                .into(),
        });
    }
    let stamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
    let out_dir = backup_dir(paths, name).join(&stamp);
    std::fs::create_dir_all(&out_dir).map_err(|e| LauncherError::Io {
        context: "creating backup dir".into(),
        source: e,
    })?;
    if username == crate::instance_users::current_username().unwrap_or_default() {
        // Base account: our own hive is always loaded and our files are
        // readable — export/copy straight into the backup dir, no staging
        // dir or stored credential. robocopy exit codes 0-7 are success.
        let reg = Command::new("reg.exe")
            .arg("export")
            .arg(format!(
                r"HKCU\{}",
                crate::windows_targets::PLAYERPREFS_REG_KEY
            ))
            .arg(out_dir.join("account.reg"))
            .arg("/y")
            .status()
            .map_err(|e| LauncherError::Io {
                context: "exporting account key".into(),
                source: e,
            })?;
        if !reg.success() {
            return Err(LauncherError::Operation {
                context: "backup".into(),
                message: format!("reg export exited with {reg}"),
            });
        }
        let home = service_user_home(username)?;
        let copy = Command::new("robocopy")
            .arg(home.join(crate::windows_targets::SESSION_DIR_RELATIVE))
            .arg(out_dir.join("sessions"))
            .arg("/E")
            .status()
            .map_err(|e| LauncherError::Io {
                context: "copying session files".into(),
                source: e,
            })?;
        if copy.code().unwrap_or(8) >= 8 {
            return Err(LauncherError::Operation {
                context: "backup".into(),
                message: format!("robocopy exited with {copy}"),
            });
        }
        restrict_to_current_user(&out_dir)?;
        return Ok(out_dir);
    }
    let staging = Path::new(r"C:\Users\Public\stfc-mi-staging").join(name);
    let staged_reg = staging.join("account.reg");
    let staged_sessions = staging.join("sessions");
    let _ = std::fs::remove_file(&staged_reg);
    let _ = std::fs::remove_dir_all(&staged_sessions);
    std::fs::create_dir_all(&staging).map_err(|e| LauncherError::Io {
        context: "creating staging dir".into(),
        source: e,
    })?;

    let mut reg = Command::new("reg.exe");
    reg.arg("export")
        .arg(format!(
            r"HKCU\{}",
            crate::windows_targets::PLAYERPREFS_REG_KEY
        ))
        .arg(&staged_reg)
        .arg("/y");
    run_as_user(username, &reg)?;
    wait_for_path(&staged_reg)?;

    let home = service_user_home(username)?;
    let mut robocopy = Command::new("robocopy");
    robocopy
        .arg(home.join(crate::windows_targets::SESSION_DIR_RELATIVE))
        .arg(&staged_sessions)
        .arg("/E");
    run_as_user(username, &robocopy)?;
    wait_for_path(&staged_sessions)?;

    std::fs::rename(&staged_reg, out_dir.join("account.reg")).map_err(|e| LauncherError::Io {
        context: "moving account.reg into backup".into(),
        source: e,
    })?;
    std::fs::rename(&staged_sessions, out_dir.join("sessions")).map_err(|e| LauncherError::Io {
        context: "moving sessions into backup".into(),
        source: e,
    })?;
    restrict_to_current_user(&out_dir)?;
    Ok(out_dir)
}

/// Windows v1: restore runs with the instance STOPPED, so the hive must be
/// loaded explicitly — that needs elevation (rare path, UAC prompt is
/// acceptable per plan). The account .reg is rewritten from HKEY_CURRENT_USER
/// to a temporary HKEY_USERS\stfc_restore mount of the service user's hive.
/// Session files are robocopied by the elevated process without /COPYALL so
/// they inherit the profile ACLs (service user keeps access).
#[cfg(target_os = "windows")]
pub fn restore_instance(paths: &ManagedPaths, name: &str, username: &str) -> LauncherResult<()> {
    let backup = latest_backup(paths, name).ok_or_else(|| LauncherError::InvalidData {
        context: "restore".into(),
        message: format!("no backup found for instance {name}"),
    })?;
    stop_before_restore(username)?;
    if username == crate::instance_users::current_username().unwrap_or_default() {
        // Base account: own hive is loaded and HKCU paths match the backup
        // as-is — direct import + copy, no elevation.
        let reg = Command::new("reg.exe")
            .arg("import")
            .arg(backup.join("account.reg"))
            .status()
            .map_err(|e| LauncherError::Io {
                context: "importing account key".into(),
                source: e,
            })?;
        if !reg.success() {
            return Err(LauncherError::Operation {
                context: "restore".into(),
                message: format!("reg import exited with {reg}"),
            });
        }
        let home = service_user_home(username)?;
        let copy = Command::new("robocopy")
            .arg(backup.join("sessions"))
            .arg(home.join(crate::windows_targets::SESSION_DIR_RELATIVE))
            .arg("/E")
            .status()
            .map_err(|e| LauncherError::Io {
                context: "restoring session files".into(),
                source: e,
            })?;
        if copy.code().unwrap_or(8) >= 8 {
            return Err(LauncherError::Operation {
                context: "restore".into(),
                message: format!("robocopy exited with {copy}"),
            });
        }
        return Ok(());
    }
    let reg_text =
        std::fs::read_to_string(backup.join("account.reg")).map_err(|e| LauncherError::Io {
            context: "reading account.reg".into(),
            source: e,
        })?;
    let rewritten = reg_text.replace("HKEY_CURRENT_USER", "HKEY_USERS\\stfc_restore");
    let rewritten_path = backup.join("account.restore.reg");
    std::fs::write(&rewritten_path, rewritten).map_err(|e| LauncherError::Io {
        context: "writing rewritten account.reg".into(),
        source: e,
    })?;
    let home = service_user_home(username)?;
    let script = format!(
        "$hive = 'HKU\\stfc_restore'\n\
         $loadedBefore = Test-Path 'Registry::HKEY_USERS\\stfc_restore'\n\
         if (-not $loadedBefore) {{ reg load $hive '{}' | Out-Null }}\n\
         reg import '{}'\n\
         if (-not $loadedBefore) {{ [gc]::Collect(); reg unload $hive | Out-Null }}\n\
         robocopy '{}' '{}' /E | Out-Null\n",
        home.join("NTUSER.DAT").display(),
        rewritten_path.display(),
        backup.join("sessions").display(),
        home.join(crate::windows_targets::SESSION_DIR_RELATIVE)
            .display(),
    );
    crate::provisioning::run_elevated(&script)
}

/// Owner-only ACL on a backup dir (spec FR-6.3): strip inheritance, grant the
/// current user full control only.
#[cfg(target_os = "windows")]
fn restrict_to_current_user(dir: &Path) -> LauncherResult<()> {
    let user = std::env::var("USERNAME").map_err(|_| LauncherError::InvalidData {
        context: "restricting backup ACL".into(),
        message: "USERNAME env var not set".into(),
    })?;
    let status = Command::new("icacls")
        .arg(dir)
        .args(["/inheritance:r", "/grant:r", &format!("{user}:(OI)(CI)F")])
        .status()
        .map_err(|e| LauncherError::Io {
            context: "running icacls".into(),
            source: e,
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(LauncherError::Operation {
            context: "restricting backup ACL".into(),
            message: format!("icacls exited with {status}"),
        })
    }
}

/// run_as_user returns a pid, not an exit status; detect success by the
/// expected output appearing. 30s ceiling.
#[cfg(target_os = "windows")]
fn wait_for_path(path: &Path) -> LauncherResult<()> {
    for _ in 0..300 {
        if path.exists() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err(LauncherError::Operation {
        context: "backup".into(),
        message: format!("timed out waiting for {}", path.display()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::ManagedPaths;
    use std::path::Path;

    #[test]
    fn backup_layout_is_per_instance_dated() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        let dir = backup_dir(&paths, "alt2");
        assert!(dir.ends_with("backups/alt2") || dir.ends_with(r"backups\alt2"));
    }

    #[test]
    fn latest_backup_picks_newest() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        let dir = backup_dir(&paths, "alt2");
        std::fs::create_dir_all(dir.join("2026-08-01T00-00-00")).expect("b1");
        std::fs::create_dir_all(dir.join("2026-08-24T00-00-00")).expect("b2");
        let latest = latest_backup(&paths, "alt2").expect("some");
        assert!(latest.ends_with("2026-08-24T00-00-00"));
        assert!(latest_backup(&paths, "nobody").is_none());
    }

    #[test]
    fn macos_backup_commands_capture_credentials_via_stdout() {
        let cmds = macos_backup_commands("stfc-alt2", Path::new("/Users/stfc-alt2"));
        let joined: Vec<String> = cmds.iter().map(|c| c.join(" ")).collect();
        assert!(joined
            .iter()
            .any(|c| c.contains("defaults export com.scopely.startrek -")));
        assert!(joined
            .iter()
            .any(|c| c.contains("tar -cf -") && c.contains("com.scopely.startrek")));
        assert!(joined
            .iter()
            .all(|c| c.starts_with("sudo -Hu stfc-alt2 env ")));
    }

    #[test]
    fn macos_restore_commands_run_as_service_user_via_env() {
        let cmds = macos_restore_commands("stfc-alt2", Path::new("/Users/stfc-alt2"));
        let joined: Vec<String> = cmds.iter().map(|c| c.join(" ")).collect();
        assert!(joined
            .iter()
            .all(|c| c.starts_with("sudo -Hu stfc-alt2 env ")));
        assert!(joined.iter().any(|c| c.contains("tar -xf -")));
        assert!(joined
            .iter()
            .any(|c| c.contains("defaults import com.scopely.startrek")));
    }

    #[test]
    fn macos_restore_mkdirs_session_dir_first() {
        let cmds = macos_restore_commands("stfc-alt2", Path::new("/Users/stfc-alt2"));
        // A freshly re-provisioned instance has no session dir; the mkdir must
        // run before tar -C or the lost-account recovery path fails.
        let first = cmds[0].join(" ");
        assert!(
            first.contains("/bin/mkdir -p"),
            "first restore command must mkdir: {first}"
        );
        assert!(
            first.contains("/Users/stfc-alt2/Library/Application Support/com.scopely.startrek"),
            "mkdir targets the session dir: {first}"
        );
        assert!(cmds[1].join(" ").contains("tar -xf -"));
    }

    #[test]
    fn macos_restore_stages_plist_under_home_not_tmpdir() {
        let cmds = macos_restore_commands("stfc-alt2", Path::new("/Users/stfc-alt2"));
        let joined: Vec<String> = cmds.iter().map(|c| c.join(" ")).collect();
        let import = joined
            .iter()
            .find(|c| c.contains("defaults import"))
            .expect("import command present");
        // $TMPDIR is env_keep'd by sudo but is mode 0700 owned by the invoking
        // user — the service user cannot write there. sudo -H sets HOME to the
        // service user's own writable home.
        assert!(import.contains("$HOME/.stfc-restore.plist"));
        assert!(!import.contains("$TMPDIR"));
    }

    #[cfg(unix)]
    #[test]
    fn backup_dir_is_created_owner_only() {
        use std::os::unix::fs::MetadataExt;
        let root = tempfile::tempdir().expect("tempdir");
        let dir = root.path().join("backups/alt2/stamp");
        create_private_dir(&dir).expect("create_private_dir");
        assert_eq!(dir.metadata().expect("metadata").mode() & 0o777, 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn credential_file_is_locked_owner_only() {
        use std::os::unix::fs::MetadataExt;
        let root = tempfile::tempdir().expect("tempdir");
        let file = root.path().join("account.plist");
        std::fs::write(&file, "x").expect("write");
        set_owner_only(&file).expect("set_owner_only");
        assert_eq!(file.metadata().expect("metadata").mode() & 0o777, 0o600);
    }
}
