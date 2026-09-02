//! Macos/Win provisioning scripts + sudoers content for the multi-instance
//! mode. Run as `std::process::Command` argv (pure builder fns); a different
//! code path executes the result with admin privileges (mac: `osascript ...
//! with administrator privileges`; win: `Start-Process -Verb RunAs`).
//!
//! `dead_code` allowed intentionally: callers (Task 5 elevation wrappers,
//! Task 9 `mi_provision` command) land in subsequent tasks.
#![allow(dead_code)]

use crate::errors::{LauncherError, LauncherResult};
use crate::instance_users::{os_username, USER_PREFIX};
use crate::models::Platform;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Machine-wide location the game's single shared install lives at once
/// provisioning relocates it (spec D-2, §5). Constant per platform.
pub fn default_shared_root(platform: Platform) -> PathBuf {
    match platform {
        Platform::MacOs => "/Users/Shared/STFC/game".into(),
        Platform::Windows => r"C:\Games\STFC\game".into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelocationStatus {
    /// Game already at `shared_root` — relocation is a no-op.
    AlreadyThere,
    /// Game installs lives at `from` and needs to move to `shared_root`.
    NeedsMove { from: PathBuf },
    /// No game install is known — wizard step is skipped.
    NoGame,
}

/// Pure decision: given the launcher's known `game_path` and the desired
/// shared root, what does the relocation step actually need to do? Used by
/// Task 9's `mi_wizard_plan` to decide whether to dispatch the relocate step.
pub fn relocation_status(game_path: Option<&Path>, shared_root: &Path) -> RelocationStatus {
    match game_path {
        None => RelocationStatus::NoGame,
        Some(p) if p == shared_root => RelocationStatus::AlreadyThere,
        Some(p) => RelocationStatus::NeedsMove {
            from: p.to_path_buf(),
        },
    }
}

/// Recursive regular-file count — cheap post-ditto verification that the
/// shared-root payload matches the source before the launcher re-pins
/// `game_path` (spec D-2: the shared root becomes THE install).
pub fn file_count(root: &Path) -> LauncherResult<u64> {
    let mut count = 0u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| LauncherError::Io {
            context: format!("reading {}", dir.display()),
            source: e,
        })? {
            let entry = entry.map_err(|e| LauncherError::Io {
                context: format!("reading {}", dir.display()),
                source: e,
            })?;
            let file_type = entry.file_type().map_err(|e| LauncherError::Io {
                context: format!("reading {}", entry.path().display()),
                source: e,
            })?;
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Day-to-day sudo scope (spec §5.1): run ANY command but only AS stfc-*
/// service users — the run-as restriction is the security boundary; service
/// users own nothing outside their homes. Never `NOPASSWD: ALL` without a
/// run-as list.
pub fn sudoers_content(primary_user: &str) -> String {
    // ponytail: command set is `/usr/bin/env` only so the entire shell
    // (incl. defaults/tar/sh as covered by env) runs as a service user.
    // Add new binaries here only after reviewing the run-as boundary.
    format!("{primary_user} ALL=({USER_PREFIX}*) NOPASSWD: /usr/bin/env\n")
}

/// Idempotent provisioning: only the bare record creation is guarded by a
/// `dscl -read` short-circuit — every attribute set, password reset, and
/// createhomedir runs unconditionally so a user left half-created by an
/// interrupted run is repaired on the next one. If the game already lives at
/// shared_root, `ditto` is fine (it copies into the existing directory merge
/// — acceptable because we move the source away after the wizard run). All
/// inputs are shell-quoted.
///
/// Called with one password prompt via osascript (Task 5 wrapper).
pub fn macos_provision_script(
    primary_user: &str,
    game_source: &Path,
    shared_root: &Path,
    names: &[String],
) -> String {
    let mut s = String::from("#!/bin/bash\nset -e\n");

    // --- users ---
    // UIDs are allocated above the current on-disk max at run time; a fixed
    // base collides in practice (502 is a common second-login-account uid).
    // set_user_attr delete+create keeps re-runs idempotent: dscl -create
    // errors when the value already exists.
    s.push_str("next_uid=$(( $(dscl . -list /Users UniqueID | awk '{print $2}' | sort -n | tail -1) + 1 ))\n");
    s.push_str("set_user_attr() {\n  dscl . -delete \"/Users/$1\" \"$2\" 2>/dev/null || true\n  dscl . -create \"/Users/$1\" \"$2\" \"$3\"\n}\n");
    for name in names {
        let user = os_username(name);
        for line in [
            format!("if ! dscl . -read /Users/{user} >/dev/null 2>&1; then"),
            format!("  dscl . -create /Users/{user}"),
            "fi".to_string(),
            format!("uid=$(dscl . -read /Users/{user} UniqueID 2>/dev/null | awk '{{print $2}}')"),
            "if [ -z \"$uid\" ]; then uid=$next_uid; next_uid=$((next_uid + 1)); fi".to_string(),
            format!("set_user_attr {user} UserShell /usr/bin/false"),
            format!("set_user_attr {user} RealName 'STFC {name}'"),
            format!("set_user_attr {user} UniqueID \"$uid\""),
            format!("set_user_attr {user} PrimaryGroupID 20"),
            format!("set_user_attr {user} NFSHomeDirectory /Users/{user}"),
            format!("set_user_attr {user} IsHidden 1"),
            format!("dscl . -passwd /Users/{user} \"$(openssl rand -base64 24)\""),
            format!("createhomedir -c -u {user}"),
        ] {
            s.push_str(&line);
            s.push('\n');
        }
    }

    // --- shared install --- (skip the move entirely if the source IS the shared root)
    let src = posix_quote(game_source);
    let dst = posix_quote(shared_root);
    s.push_str(&format!("mkdir -p '{dst}'\n"));
    if game_source != shared_root {
        s.push_str(&format!(
            "if [ -d '{src}' ]; then ditto '{src}' '{dst}'; fi\n"
        ));
    }
    s.push_str(&format!(
        "chmod -R a+rX '{dst}'\n\
         xattr -dr com.apple.quarantine '{dst}' 2>/dev/null || true\n"
    ));

    // --- sudoers (validate a temp file, then install atomically) ---
    // Under `set -e`, a failed visudo must not leave a live malformed sudoers.
    // mktemp + trap: a PID-predictable /tmp path is a root symlink-follow
    // overwrite vector; the trap removes the temp file on any failure.
    s.push_str(&format!(
        "tmp=$(mktemp /tmp/stfc-mi.sudoers.XXXXXX)\n\
         trap 'rm -f \"$tmp\"' EXIT\n\
         cat > \"$tmp\" <<'EOF'\n{}EOF\n\
         visudo -cf \"$tmp\"\n\
         install -o root -g wheel -m 0440 \"$tmp\" /etc/sudoers.d/stfc-mi\n\
         rm -f \"$tmp\"\n\
         trap - EXIT\n",
        sudoers_content(primary_user)
    ));
    s
}

/// Naive single-quote-escape — paths are launcher-internal, never user input.
fn posix_quote(p: &Path) -> String {
    p.to_string_lossy().replace('\'', "'\\''")
}

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// ponytail: std-only base64 (no new dep for one encode call).
pub fn encode_script_for_osascript(script: &str) -> String {
    let mut out = String::new();
    for chunk in script.as_bytes().chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    // test-only decoder: inverse of encode_script_for_osascript
    let val = |c: u8| -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("bad base64 char {}", c as char)),
        }
    };
    let bytes: Vec<u8> = s.bytes().filter(|b| *b != b'=').collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let mut n: u32 = 0;
        for (i, c) in chunk.iter().enumerate() {
            n |= val(*c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

/// One password prompt for the whole provisioning run (spec D-5).
/// AppleScript string stays single-line: script travels base64-encoded.
#[cfg(target_os = "macos")]
pub fn run_elevated(script: &str) -> LauncherResult<()> {
    let encoded = encode_script_for_osascript(script);
    let applescript = format!(
        "do shell script \"echo {encoded} | /usr/bin/base64 -D | /bin/bash\" with administrator privileges"
    );
    let output = Command::new("osascript")
        .args(["-e", &applescript])
        .output()
        .map_err(|e| LauncherError::Io {
            context: "running osascript".into(),
            source: e,
        })?;
    if output.status.success() {
        Ok(())
    } else {
        // osascript reports a cancel as "User canceled. (-128)" and a shell
        // failure as "execution error: <stderr>"; surface whichever we got.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        Err(LauncherError::InvalidData {
            context: "elevated provisioning".into(),
            message: if detail.is_empty() {
                format!("osascript exited with {}", output.status)
            } else {
                format!("elevated provisioning failed: {detail}")
            },
        })
    }
}

/// Contents of the Terminal-run deletion script (pure, for tests).
#[cfg(target_os = "macos")]
fn macos_deprovision_script(username: &str, current: &str, self_path: &Path) -> String {
    format!(
        "sudo sysadminctl -deleteUser {username} -adminUser {current} -adminPassword -\nrm -f \"{self_path}\"\nexit\n",
        self_path = self_path.display()
    )
}

#[cfg(target_os = "macos")]
fn user_record_exists(username: &str) -> bool {
    Command::new("dscl")
        .args([".", "-read", &format!("/Users/{username}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(true) // on lookup error, assume present
}

/// Deletes a service user on macOS. Directory Services refuses record
/// deletion from a root osascript shell (eDSPermissionError -14120) and TCC
/// blocks root from removing the home dir; sysadminctl is Apple's entitled
/// tool but needs a real TTY for its secure `-adminPassword -` prompt. So we
/// hand the command a visible Terminal window (one password entry, typed
/// straight into the OS prompt — never through the launcher) and poll for
/// the record to disappear. Already-deleted records return Ok immediately so
/// retries after a partial failure complete cleanly.
#[cfg(target_os = "macos")]
pub fn deprovision_user(username: &str) -> LauncherResult<()> {
    if !user_record_exists(username) {
        return Ok(());
    }
    let current = crate::instance_users::current_username()?;
    let script_path = std::env::temp_dir().join(format!("stfc-remove-{username}.sh"));
    std::fs::write(
        &script_path,
        macos_deprovision_script(username, &current, &script_path),
    )
    .map_err(|e| LauncherError::Io {
        context: "writing deprovision script".into(),
        source: e,
    })?;
    let status = Command::new("osascript")
        .args([
            "-e",
            &format!(
                "tell application \"Terminal\" to do script \"bash '{}'\"",
                script_path.display().to_string().replace('\'', "'\\''")
            ),
            "-e",
            "tell application \"Terminal\" to activate",
        ])
        .status()
        .map_err(|e| LauncherError::Io {
            context: "opening Terminal for user deletion".into(),
            source: e,
        })?;
    if !status.success() {
        return Err(LauncherError::Operation {
            context: "deprovisioning".into(),
            message: format!("could not open Terminal (osascript exited {status})"),
        });
    }
    // ponytail: fixed 2-minute ceiling — if the user closes the Terminal
    // window without entering their password, we time out and they retry.
    for _ in 0..240 {
        if !user_record_exists(username) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    Err(LauncherError::Operation {
        context: "deprovisioning".into(),
        message: format!(
            "user {username} still present after 2 minutes — was the Terminal password prompt completed?"
        ),
    })
}

#[cfg(target_os = "windows")]
pub fn run_elevated(script: &str) -> LauncherResult<()> {
    // Write the PS1 to the launcher staging dir, then self-elevate via UAC.
    // The elevated process writes a sentinel .done/.fail file we poll for.
    let dir = std::env::temp_dir().join("stfc-mi-provision");
    std::fs::create_dir_all(&dir).map_err(|e| LauncherError::Io {
        context: "staging dir".into(),
        source: e,
    })?;
    let ps1 = dir.join("provision.ps1");
    let done = dir.join("provision.done");
    let fail = dir.join("provision.fail");
    let _ = std::fs::remove_file(&done);
    let _ = std::fs::remove_file(&fail);
    let wrapped = format!(
        "try {{\n{script}\nNew-Item '{}' -Force | Out-Null\n}} catch {{\nNew-Item '{}' -Force | Out-Null\n}}\nRemove-Item -LiteralPath $PSCommandPath -Force -ErrorAction SilentlyContinue\n",
        done.display(),
        fail.display()
    );
    std::fs::write(&ps1, wrapped).map_err(|e| LauncherError::Io {
        context: "writing provision.ps1".into(),
        source: e,
    })?;
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Start-Process",
            "powershell",
            "-Verb",
            "RunAs",
            "-ArgumentList",
            &format!(
                "'-NoProfile','-ExecutionPolicy','Bypass','-File','{}'",
                ps1.display()
            ),
        ])
        .status()
        .map_err(|e| LauncherError::Io {
            context: "UAC elevation".into(),
            source: e,
        })?;
    if !status.success() {
        let _ = std::fs::remove_file(&ps1);
        return Err(LauncherError::InvalidData {
            context: "UAC elevation".into(),
            message: "elevation declined or failed to start".into(),
        });
    }
    // ponytail: poll sentinel up to 10 min; UAC prompt waits on the human.
    // Compute the result first so ps1/sentinel cleanup runs on every exit path
    // (the ps1 contains generated passwords — never leave it on disk, NFR-1).
    let mut result = Err(LauncherError::InvalidData {
        context: "elevated provisioning".into(),
        message: "timed out waiting for elevated script".into(),
    });
    for _ in 0..600 {
        if done.exists() {
            result = Ok(());
            break;
        }
        if fail.exists() {
            result = Err(LauncherError::InvalidData {
                context: "elevated provisioning".into(),
                message: "provision script failed (see staging dir)".into(),
            });
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    let _ = std::fs::remove_file(&ps1);
    let _ = std::fs::remove_file(&done);
    let _ = std::fs::remove_file(&fail);
    result
}

/// Alphanumeric-only so the password can safely embed in the elevated PS1 string.
pub fn generate_password() -> LauncherResult<String> {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).map_err(|e| LauncherError::InvalidData {
        context: "generating instance password".into(),
        message: e.to_string(),
    })?;
    let mut chars: Vec<char> = hex::encode(buf).chars().collect();
    // Windows' default local-account complexity policy requires 3 of 4
    // character classes; lowercase hex has only 2 (lower + digit). Uppercase
    // the second half, then force one char per class so the guarantee is
    // deterministic, not probabilistic.
    for c in chars.iter_mut().skip(32) {
        c.make_ascii_uppercase();
    }
    if !chars[..32].iter().any(|c| c.is_ascii_lowercase()) {
        chars[0] = 'a';
    }
    if !chars[32..].iter().any(|c| c.is_ascii_uppercase()) {
        chars[32] = 'A';
    }
    if !chars.iter().any(|c| c.is_ascii_digit()) {
        chars[1] = '0';
    }
    Ok(chars.into_iter().collect())
}

#[cfg(target_os = "windows")]
fn windows_entry(username: &str) -> keyring_core::Result<keyring_core::Entry> {
	static INIT: std::sync::Once = std::sync::Once::new();
	INIT.call_once(|| {
		let store = windows_native_keyring_store::Store::new()
			.expect("windows credential store init");
		keyring_core::set_default_store(store);
	});
	keyring_core::Entry::new("stfc-mi", username)
}

#[cfg(target_os = "windows")]
pub fn store_windows_password(username: &str, password: &str) -> LauncherResult<()> {
	windows_entry(username)
		.and_then(|e| e.set_password(password))
		.map_err(|e| LauncherError::InvalidData {
			context: "storing instance credential".into(),
			message: e.to_string(),
		})
}

#[cfg(target_os = "windows")]
pub fn read_windows_password(username: &str) -> LauncherResult<String> {
	windows_entry(username)
		.and_then(|e| e.get_password())
		.map_err(|e| LauncherError::InvalidData {
			context: "reading instance credential".into(),
			message: e.to_string(),
		})
}

/// One-time elevation (T-# spec §5). Passwords are generated by the caller
/// and stored in Credential Manager by Task 5 — they appear here only inside
/// the elevated process, never on disk. Script is COPY-only so re-runs are
/// safe and idempotent.
pub fn windows_provision_script(
    game_source: &Path,
    shared_root: &Path,
    names: &[String],
    passwords: &[String],
) -> String {
    assert_eq!(
        names.len(),
        passwords.len(),
        "names and passwords must pair up"
    );
    let mut s = String::from("$ErrorActionPreference = 'Stop'\n");

    for (name, password) in names.iter().zip(passwords) {
        let user = os_username(name);
        let pw_escaped = password.replace('\'', "''");
        s.push_str(&format!(
            "if (-not (Get-LocalUser -Name '{user}' -ErrorAction SilentlyContinue)) {{\n\
             \x20 $p = ConvertTo-SecureString '{pw_escaped}' -AsPlainText -Force\n\
             \x20 New-LocalUser '{user}' -Password $p -FullName 'STFC {name}' -PasswordNeverExpires | Out-Null\n\
             \x20 New-Item -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon\\SpecialAccounts\\UserList' -Force | Out-Null\n\
             \x20 New-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon\\SpecialAccounts\\UserList' -Name '{user}' -Value 0 -PropertyType DWord -Force | Out-Null\n\
             }}\n"
        ));
    }

    // Escape single quotes for PS single-quoted strings, same as passwords.
    let src = game_source.to_string_lossy().replace('\'', "''");
    let dst = shared_root.to_string_lossy().replace('\'', "''");
    s.push_str(&format!(
        "if ((Test-Path '{src}') -and ('{src}' -ne '{dst}')) {{\n\
         \x20 robocopy '{src}' '{dst}' /MIR /COPY:DAT | Out-Null\n\
         \x20 if ($LASTEXITCODE -ge 8) {{ throw \"robocopy failed: $LASTEXITCODE\" }}\n\
         }}\n\
         icacls '{dst}' /grant 'BUILTIN\\Users:(OI)(CI)RX' /T | Out-Null\n\
         if ($LASTEXITCODE -ne 0) {{ throw \"icacls failed: $LASTEXITCODE\" }}\n"
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn sudoers_is_runas_scoped_not_blanket() {
        let s = sudoers_content("ebendler");
        assert!(
            s.contains("ebendler ALL=(stfc-*) NOPASSWD: /usr/bin/env"),
            "expected run-as-scoped NOPASSWD rule in {s:?}"
        );
        assert!(!s.contains("NOPASSWD: ALL"), "must not include bare ALL");
    }

    #[test]
    fn mac_script_covers_users_game_and_sudoers() {
        let names = vec!["alt2".to_string()];
        let s = macos_provision_script(
            "ebendler",
            Path::new("/Users/ebendler/Library/Application Support/Star Trek Fleet Command"),
            Path::new("/Users/Shared/STFC/game"),
            &names,
        );
        // user creation (requirements doc §2.A.3 recipe)
        assert!(s.contains("dscl . -create /Users/stfc-alt2"));
        assert!(s.contains("IsHidden 1"));
        assert!(s.contains("createhomedir -c -u stfc-alt2"));
        // idempotent: skips existing users
        assert!(s.contains("dscl . -read /Users/stfc-alt2"));
        // single shared install move + perms + quarantine strip
        assert!(s.contains("ditto"));
        assert!(s.contains("/Users/Shared/STFC/game"));
        assert!(s.contains("chmod -R a+rX"));
        assert!(s.contains("xattr -dr com.apple.quarantine"));
        // sudoers written atomically and validated
        assert!(s.contains("/etc/sudoers.d/stfc-mi"));
        assert!(s.contains("visudo -cf"));
        assert!(s.contains("ebendler ALL=(stfc-*) NOPASSWD: /usr/bin/env"));
    }

    #[test]
    fn windows_script_hides_users_and_acls_game() {
        let names = vec!["alt2".to_string()];
        let pw = vec!["pw-alt2".to_string()];
        let s = windows_provision_script(
            Path::new(r"C:\Users\e\AppData\Star Trek Fleet Command"),
            Path::new(r"C:\Games\STFC\game"),
            &names,
            &pw,
        );
        assert!(s.contains("New-LocalUser 'stfc-alt2'"));
        assert!(s.contains("SpecialAccounts\\UserList"));
        assert!(s.contains("robocopy"));
        assert!(s.contains("icacls"));
        assert!(s.contains("C:\\Games\\STFC\\game"));
        // ensure the elevated script reads cleanly as PowerShell (sentences joined with '\n')
        assert!(s.contains("ConvertTo-SecureString 'pw-alt2' -AsPlainText -Force"));
    }

    #[test]
    fn mac_shared_root_is_quoted_and_same_path_skips_ditto() {
        let names = vec!["alt2".to_string()];
        // single quote in the shared root must be safely escaped
        let s = macos_provision_script(
            "ebendler",
            Path::new("/old/game"),
            Path::new("/Users/Shared/STFC's game"),
            &names,
        );
        assert!(
            s.contains("/Users/Shared/STFC'\\''s game"),
            "shared_root must be posix-quoted: {s}"
        );

        // identical src/dst: no ditto line at all
        let same = macos_provision_script(
            "ebendler",
            Path::new("/Users/Shared/STFC/game"),
            Path::new("/Users/Shared/STFC/game"),
            &names,
        );
        assert!(
            !same.contains("ditto"),
            "same src/dst must skip ditto: {same}"
        );
    }

    #[test]
    fn mac_sudoers_validates_temp_before_install() {
        let s = macos_provision_script(
            "ebendler",
            Path::new("/old/game"),
            Path::new("/Users/Shared/STFC/game"),
            &["alt2".to_string()],
        );
        let visudo = s.find("visudo -cf").expect("visudo present");
        let install = s
            .find("install -o root -g wheel -m 0440")
            .expect("install present");
        assert!(
            visudo < install,
            "temp file must be validated before install"
        );
        // mktemp: a PID-predictable /tmp path is a root symlink-follow vector
        assert!(s.contains("tmp=$(mktemp /tmp/stfc-mi.sudoers.XXXXXX)"));
        assert!(!s.contains("stfc-mi.sudoers.$$"));
        // trap cleans the temp file up on failure; cleared after success
        let trap_set = s.find("trap 'rm -f \"$tmp\"' EXIT").expect("trap set");
        let trap_clear = s.find("trap - EXIT").expect("trap cleared");
        assert!(trap_set < visudo && visudo < trap_clear);
        // never written live before validation
        assert!(!s.contains("cat > /etc/sudoers.d"));
        // heredoc terminator must be a bare EOF line
        assert!(
            s.contains("\nEOF\n"),
            "heredoc terminator must be bare EOF: {s}"
        );
    }

    #[test]
    fn elevated_mac_wraps_script_as_base64_one_liner() {
        // run_elevated (macOS) must not embed raw newlines/quotes in the AppleScript
        // string; verify the encoding helper instead of executing osascript.
        let encoded = encode_script_for_osascript("echo \"hi\"\nset -e\n");
        let decoded =
            String::from_utf8(base64_decode(&encoded).expect("valid base64")).expect("utf8");
        assert_eq!(decoded, "echo \"hi\"\nset -e\n");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_deprovision_uses_sysadminctl_interactive_password() {
        let s = macos_deprovision_script(
            "stfc-alt2",
            "ebendler",
            Path::new("/tmp/stfc-remove-stfc-alt2.sh"),
        );
        assert!(s.contains(
            "sudo sysadminctl -deleteUser stfc-alt2 -adminUser ebendler -adminPassword -"
        ));
        assert!(s.contains("rm -f \"/tmp/stfc-remove-stfc-alt2.sh\""));
    }

    #[test]
    fn generated_passwords_are_long_and_alphanumeric() {
        let p = generate_password().expect("entropy");
        assert!(p.len() >= 24);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn file_count_counts_recursively() {
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(root.path().join("a/b")).expect("dirs");
        std::fs::write(root.path().join("a/one"), "1").expect("file");
        std::fs::write(root.path().join("a/b/two"), "2").expect("file");
        std::fs::write(root.path().join("three"), "3").expect("file");

        assert_eq!(file_count(root.path()).expect("count"), 3);
    }

    #[test]
    fn file_count_errors_on_missing_dir() {
        let root = tempfile::tempdir().expect("tempdir");
        assert!(file_count(&root.path().join("nope")).is_err());
    }

    #[test]
    fn generated_passwords_meet_windows_complexity_classes() {
        // Windows default local-account policy requires 3 of 4 classes;
        // lowercase hex alone (lower + digit) fails New-LocalUser.
        for _ in 0..20 {
            let p = generate_password().expect("entropy");
            assert!(p.chars().any(|c| c.is_ascii_lowercase()), "lower: {p}");
            assert!(p.chars().any(|c| c.is_ascii_uppercase()), "upper: {p}");
            assert!(p.chars().any(|c| c.is_ascii_digit()), "digit: {p}");
        }
    }

    #[test]
    fn windows_paths_escape_single_quotes_and_check_exit_codes() {
        let s = windows_provision_script(
            Path::new(r"C:\Users\e'photos\game"),
            Path::new(r"C:\Games\STFC\game"),
            &["alt2".to_string()],
            &["pw".to_string()],
        );
        assert!(
            s.contains("C:\\Users\\e''photos\\game"),
            "path quote must double: {s}"
        );
        assert!(s.contains("if ($LASTEXITCODE -ge 8) { throw \"robocopy failed"));
        assert!(s.contains("if ($LASTEXITCODE -ne 0) { throw \"icacls failed"));
    }

    #[test]
    fn relocation_status_cases() {
        use std::path::PathBuf;
        use RelocationStatus::*;
        let root = Path::new("/Users/Shared/STFC/game");
        assert_eq!(relocation_status(Some(root), root), AlreadyThere);
        assert_eq!(relocation_status(None, root), NoGame);
        assert_eq!(
            relocation_status(Some(Path::new("/Users/e/game")), root),
            NeedsMove {
                from: PathBuf::from("/Users/e/game"),
            }
        );
    }

    #[test]
    fn default_shared_root_matches_platform() {
        assert_eq!(
            default_shared_root(crate::models::Platform::MacOs),
            std::path::PathBuf::from("/Users/Shared/STFC/game")
        );
        assert_eq!(
            default_shared_root(crate::models::Platform::Windows),
            std::path::PathBuf::from(r"C:\Games\STFC\game")
        );
    }
}
