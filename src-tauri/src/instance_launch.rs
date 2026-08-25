// Items here are the public interface consumed by the command layer (Task 9).
// Allow dead_code until that task lands — narrows later.
#![allow(dead_code)]

//! Per-instance launch/stop/live-status for multi-instance mode.
//!
//! macOS: the managed-mode launch plan is wrapped in
//! `sudo -Hu <user> env -u TMPDIR KEY=VAL ... <exe>` — env(1) re-injects the
//! DYLD vars past sudo's env scrubbing; the sudoers entry (provisioning.rs)
//! scopes this to run-as stfc-* service users only.
//! Windows: the managed plan runs via CreateProcessWithLogonW with the
//! service-user credential from the Credential Manager store.

use crate::errors::{LauncherError, LauncherResult};
use crate::instance_users::USER_PREFIX;
use crate::launch::{build_launch_plan, LaunchPlan};
use crate::models::{LaunchMode, Platform};
use std::path::Path;
use std::process::Command;

pub fn process_matcher(platform: Platform) -> &'static str {
    match platform {
        Platform::MacOs => "Star Trek Fleet Command",
        Platform::Windows => "prime.exe",
    }
}

pub fn build_instance_plan(
    platform: Platform,
    shared_root: &Path,
    mod_library: &Path,
    username: &str,
) -> LauncherResult<LaunchPlan> {
    let is_base = !username.starts_with(USER_PREFIX);
    if is_base && username != crate::instance_users::current_username()? {
        return Err(LauncherError::InvalidData {
            context: "building instance launch plan".into(),
            message: format!("refusing non-service username {username:?}"),
        });
    }
    let inner = build_launch_plan(platform, shared_root, mod_library, LaunchMode::Managed)?;
    // The base account IS the current user: no sudo wrapper needed.
    if is_base {
        return Ok(inner);
    }
    match platform {
        Platform::MacOs => {
            let mut args = vec![
                "-Hu".into(),
                username.into(),
                "env".into(),
                "-u".into(),
                "TMPDIR".into(),
            ];
            args.extend(inner.environment.iter().map(|(k, v)| format!("{k}={v}")));
            args.push(inner.executable);
            Ok(LaunchPlan {
                executable: "/usr/bin/sudo".into(),
                args,
                environment: Default::default(),
                working_dir: inner.working_dir,
            })
        }
        Platform::Windows => Ok(inner),
    }
}

pub fn parse_pid(output: &str) -> LauncherResult<Option<u32>> {
    match output.lines().next().map(str::trim) {
        None | Some("") => Ok(None),
        Some(line) => line
            .parse::<u32>()
            .map(Some)
            .map_err(|_| LauncherError::InvalidData {
                context: "parsing pid".into(),
                message: format!("unexpected pid output {line:?}"),
            }),
    }
}

/// Ground truth from the OS (spec FR-7.3), never cached state. matcher: exe
/// name fragment ("Star Trek Fleet Command" / "prime.exe").
pub fn instance_pid(username: &str, matcher: &str) -> LauncherResult<Option<u32>> {
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("pgrep")
            .args(["-U", username, "-f", matcher])
            .output()
            .map_err(|e| LauncherError::Io {
                context: "running pgrep".into(),
                source: e,
            })?;
        if out.status.success() {
            parse_pid(&String::from_utf8_lossy(&out.stdout))
        } else {
            Ok(None) // pgrep exit 1 = no match
        }
    }
    #[cfg(target_os = "windows")]
    {
        let out = Command::new("tasklist")
            .args([
                "/FI",
                &format!("USERNAME eq {username}"),
                "/FO",
                "CSV",
                "/NH",
            ])
            .output()
            .map_err(|e| LauncherError::Io {
                context: "running tasklist".into(),
                source: e,
            })?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let cols: Vec<&str> = line.split(',').map(|c| c.trim_matches('"')).collect();
            if cols.len() > 1 && cols[0].eq_ignore_ascii_case(matcher) {
                return parse_pid(cols[1]);
            }
        }
        Ok(None)
    }
}

pub fn start_instance(
    platform: Platform,
    shared_root: &Path,
    mod_library: &Path,
    username: &str,
    log_file: &Path,
) -> LauncherResult<u32> {
    let matcher = process_matcher(platform);
    if instance_pid(username, matcher)?.is_some() {
        return Err(LauncherError::InvalidData {
            context: "starting instance".into(),
            message: format!("instance {username} is already running"),
        });
    }
    let plan = build_instance_plan(platform, shared_root, mod_library, username)?;
    let log = std::fs::File::create(log_file).map_err(|e| LauncherError::Io {
        context: format!("creating {}", log_file.display()),
        source: e,
    })?;
    let log_err = log.try_clone().map_err(|e| LauncherError::Io {
        context: "cloning log handle".into(),
        source: e,
    })?;
    let mut cmd = Command::new(&plan.executable);
    cmd.args(&plan.args).stdout(log).stderr(log_err);
    if let Some(dir) = &plan.working_dir {
        cmd.current_dir(dir);
    }
    for (k, v) in &plan.environment {
        cmd.env(k, v);
    }
    #[cfg(target_os = "windows")]
    {
        // stdout/stderr redirection is not wired through CreateProcessWithLogonW
        // (ponytail: v1 launches inherit no console; logs land via the game's own
        // file logging). Returned pid prefers live re-enumeration; falls back to
        // the CreateProcessWithLogonW pid.
        let spawned_pid = run_as_user(username, &cmd)?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        // Enumeration errors must not mask a successful launch.
        Ok(instance_pid(username, matcher)
            .ok()
            .flatten()
            .unwrap_or(spawned_pid))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let child = cmd.spawn().map_err(|e| LauncherError::Io {
            context: format!("launching {}", plan.executable),
            source: e,
        })?;
        Ok(child.id())
    }
}

pub fn stop_instance(platform: Platform, username: &str) -> LauncherResult<()> {
    let matcher = process_matcher(platform);
    let Some(pid) = instance_pid(username, matcher)? else {
        return Ok(());
    };
    terminate_as_user(platform, username, pid, false)?;
    // ponytail: fixed 10s grace then SIGKILL / taskkill /F
    for _ in 0..100 {
        if instance_pid(username, matcher)?.is_none() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    terminate_as_user(platform, username, pid, true)?;
    if instance_pid(username, matcher)?.is_some() {
        return Err(LauncherError::Operation {
            context: "terminating instance".into(),
            message: format!("instance {username} (pid {pid}) survived force kill"),
        });
    }
    Ok(())
}

fn macos_kill_argv(username: &str, pid: u32, force: bool) -> Vec<String> {
    let mut args = vec![
        "-Hu".to_string(),
        username.to_string(),
        "env".to_string(),
        "/bin/kill".to_string(),
    ];
    if force {
        args.push("-9".to_string());
    }
    args.push(pid.to_string());
    args
}

/// The instance process belongs to the service user, so termination must
/// happen *as that user* — a primary-user `kill`/`taskkill` gets EPERM /
/// access-denied. macOS routes through the scoped sudoers entry (env(1)
/// invokes /bin/kill); Windows runs taskkill via CreateProcessWithLogonW.
fn terminate_as_user(
    platform: Platform,
    username: &str,
    pid: u32,
    force: bool,
) -> LauncherResult<()> {
    // The base account's processes belong to us — plain kill/taskkill,
    // no sudoers-scoped sudo or stored credential needed.
    if username == crate::instance_users::current_username().unwrap_or_default() {
        let status = match platform {
            Platform::MacOs => {
                let mut cmd = Command::new("/bin/kill");
                if force {
                    cmd.arg("-9");
                }
                cmd.arg(pid.to_string()).status()
            }
            Platform::Windows => {
                let mut cmd = Command::new("taskkill");
                if force {
                    cmd.arg("/F");
                }
                cmd.args(["/PID", &pid.to_string()]).status()
            }
        }
        .map_err(|e| LauncherError::Io {
            context: "terminating instance".into(),
            source: e,
        })?;
        return if status.success() {
            Ok(())
        } else {
            Err(LauncherError::Operation {
                context: "terminating instance".into(),
                message: format!("kill exited with {status}"),
            })
        };
    }
    match platform {
        Platform::MacOs => {
            let status = Command::new("/usr/bin/sudo")
                .args(macos_kill_argv(username, pid, force))
                .status()
                .map_err(|e| LauncherError::Io {
                    context: "terminating instance".into(),
                    source: e,
                })?;
            if status.success() {
                Ok(())
            } else {
                Err(LauncherError::Operation {
                    context: "terminating instance".into(),
                    message: format!("sudo kill exited with {status}"),
                })
            }
        }
        Platform::Windows => {
            let mut cmd = Command::new("taskkill");
            if force {
                cmd.arg("/F");
            }
            cmd.args(["/PID", &pid.to_string()]);
            #[cfg(target_os = "windows")]
            {
                run_as_user(username, &cmd)?;
                Ok(())
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = cmd;
                Err(LauncherError::Operation {
                    context: "terminating instance".into(),
                    message: "Windows termination path is only available on Windows".into(),
                })
            }
        }
    }
}

/// CreateProcessWithLogonW via the `windows` crate: the stored credential
/// (provisioning.rs) starts the process as the service user on the
/// interactive desktop (requirements §2.B.3). Returns the spawned pid.
///
/// The command's env overrides are forwarded via an explicit environment
/// block — lpEnvironment = NULL would inherit the *launcher's* env and drop
/// the managed-mode PATH override (mod injection would silently not load).
#[cfg(target_os = "windows")]
pub(crate) fn run_as_user(username: &str, cmd: &std::process::Command) -> LauncherResult<u32> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        CreateProcessWithLogonW, CREATE_UNICODE_ENVIRONMENT, LOGON_WITH_PROFILE,
        PROCESS_INFORMATION, STARTUPINFOW,
    };

    let password = crate::provisioning::read_windows_password(username)?;
    let exe = cmd.get_program().to_string_lossy().to_string();
    let cmdline: String = std::iter::once(format!("\"{exe}\""))
        .chain(cmd.get_args().map(|a| a.to_string_lossy().to_string()))
        .collect::<Vec<_>>()
        .join(" ");
    let env_block = build_env_block(cmd);
    let user_wide: Vec<u16> = username.encode_utf16().chain(std::iter::once(0)).collect();
    let pass_wide: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();
    let cmd_wide: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();
    let cwd_wide: Option<Vec<u16>> = cmd.get_current_dir().map(|d| {
        d.to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect()
    });
    let mut si = STARTUPINFOW::default();
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi = PROCESS_INFORMATION::default();
    unsafe {
        CreateProcessWithLogonW(
            windows::core::PCWSTR(user_wide.as_ptr()),
            windows::core::PCWSTR::null(), // local account, no domain
            windows::core::PCWSTR(pass_wide.as_ptr()),
            LOGON_WITH_PROFILE,
            windows::core::PCWSTR::null(),
            Some(windows::core::PWSTR(cmd_wide.as_ptr() as *mut u16)),
            CREATE_UNICODE_ENVIRONMENT,
            Some(env_block.as_ptr() as *const core::ffi::c_void),
            cwd_wide
                .as_ref()
                .map_or(windows::core::PCWSTR::null(), |v| {
                    windows::core::PCWSTR(v.as_ptr())
                }),
            &si,
            &mut pi,
        )
        .map_err(|e| LauncherError::Operation {
            context: "CreateProcessWithLogonW".into(),
            message: e.to_string(),
        })?;
        let _ = CloseHandle(pi.hThread);
        let _ = CloseHandle(pi.hProcess);
    }
    Ok(pi.dwProcessId)
}

/// Windows env var names are case-insensitive: fold keys to uppercase for
/// the merge so an inherited `Path` and an override `PATH` don't both land in
/// the block (override wins). Kept pure and non-cfg so it's testable
/// off-Windows; `build_env_block` uses it.
fn merge_env_case_insensitive(
    inherited: impl IntoIterator<Item = (String, String)>,
    overrides: &std::collections::BTreeMap<String, String>,
) -> Vec<(String, String)> {
    // folded key -> (override's key spelling, value); overrides replace wholesale
    let mut merged: std::collections::BTreeMap<String, (String, String)> = inherited
        .into_iter()
        .map(|(k, v)| (k.to_uppercase(), (k, v)))
        .collect();
    for (k, v) in overrides {
        merged.insert(k.to_uppercase(), (k.clone(), v.clone()));
    }
    merged.into_values().collect()
}

/// Double-NUL-terminated UTF-16 environment block: current process env with
/// the command's env overrides applied (set on Some, removed on None).
/// CREATE_UNICODE_ENVIRONMENT requires exactly this layout.
#[cfg(target_os = "windows")]
fn build_env_block(cmd: &std::process::Command) -> Vec<u16> {
    let mut overrides: std::collections::BTreeMap<String, String> = Default::default();
    let mut removals: Vec<String> = Vec::new();
    for (key, value) in cmd.get_envs() {
        let key = key.to_string_lossy().to_string();
        match value {
            Some(v) => {
                overrides.insert(key, v.to_string_lossy().to_string());
            }
            None => removals.push(key),
        }
    }
    let mut env = merge_env_case_insensitive(std::env::vars(), &overrides);
    for removed in &removals {
        env.retain(|(k, _)| !k.eq_ignore_ascii_case(removed));
    }
    let mut block: Vec<u16> = env
        .iter()
        .flat_map(|(k, v)| format!("{k}={v}\0").encode_utf16().collect::<Vec<u16>>())
        .collect();
    block.push(0);
    block
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Platform;

    #[test]
    fn base_account_plan_runs_directly_without_sudo() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::create_dir_all(game_root.join("Star Trek Fleet Command.app/Contents/MacOS"))
            .expect("dirs");
        std::fs::write(
            game_root.join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command"),
            "",
        )
        .expect("exe");
        let lib = game_root.join("libstfc-community-mod.dylib");
        std::fs::write(&lib, "").expect("lib");

        let me = crate::instance_users::current_username().expect("USER env");
        let plan = build_instance_plan(Platform::MacOs, game_root, &lib, &me).expect("plan");
        assert!(plan.executable.ends_with("Star Trek Fleet Command"));
    }

    #[test]
    fn mac_instance_plan_sudos_as_user_with_dyld_env() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::create_dir_all(game_root.join("Star Trek Fleet Command.app/Contents/MacOS"))
            .expect("dirs");
        std::fs::write(
            game_root.join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command"),
            "",
        )
        .expect("exe");
        let lib = game_root.join("libstfc-community-mod.dylib");
        std::fs::write(&lib, "").expect("lib");

        let plan =
            build_instance_plan(Platform::MacOs, game_root, &lib, "stfc-alt2").expect("plan");
        assert_eq!(plan.executable, "/usr/bin/sudo");
        let args = plan.args.join(" ");
        assert!(args.starts_with("-Hu stfc-alt2 env -u TMPDIR "));
        assert!(args.contains(&format!("DYLD_INSERT_LIBRARIES={}", lib.display())));
        assert!(args.contains("DYLD_LIBRARY_PATH="));
        assert!(
            args.ends_with("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command")
        );
    }

    #[test]
    fn instance_plan_rejects_unmanaged_prefix() {
        // Defense in depth: username must carry the stfc- prefix even before
        // the registry check in the command layer.
        let root = tempfile::tempdir().expect("tempdir");
        let result = build_instance_plan(Platform::MacOs, root.path(), root.path(), "root");
        assert!(result.is_err());
    }

    #[test]
    fn parses_pgrep_output() {
        assert_eq!(parse_pid("12345\n").expect("pid"), Some(12345));
        assert_eq!(parse_pid("").expect("none"), None);
        assert_eq!(parse_pid("123\n456\n").expect("first"), Some(123));
    }

    #[test]
    fn macos_kill_routes_through_sudo_as_service_user() {
        // Cross-user kill from the primary user would be EPERM; the argv
        // contract with the sudoers entry is `sudo -Hu <user> env /bin/kill`.
        assert_eq!(
            macos_kill_argv("stfc-alt2", 4242, false),
            vec!["-Hu", "stfc-alt2", "env", "/bin/kill", "4242"]
        );
        assert_eq!(
            macos_kill_argv("stfc-alt2", 4242, true),
            vec!["-Hu", "stfc-alt2", "env", "/bin/kill", "-9", "4242"]
        );
    }

    #[test]
    fn process_matcher_per_platform() {
        assert_eq!(process_matcher(Platform::MacOs), "Star Trek Fleet Command");
        assert_eq!(process_matcher(Platform::Windows), "prime.exe");
    }

    #[test]
    fn env_merge_folds_case_so_override_wins() {
        // Inherited `Path` + override `PATH` must yield exactly one entry,
        // spelled as the override, or the child could resolve the wrong one.
        let inherited = vec![
            ("Path".to_string(), "old".to_string()),
            ("TEMP".to_string(), "keep".to_string()),
        ];
        let mut overrides = std::collections::BTreeMap::new();
        overrides.insert("PATH".to_string(), "new".to_string());

        let merged = merge_env_case_insensitive(inherited, &overrides);

        let paths: Vec<_> = merged
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("path"))
            .collect();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], &("PATH".to_string(), "new".to_string()));
        assert!(merged.iter().any(|(k, v)| k == "TEMP" && v == "keep"));
    }
}
