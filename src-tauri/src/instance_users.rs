// Items here are the public interface consumed by later provisioning/launch/backup tasks.
// Allow dead_code until those tasks land — narrows later.
#![allow(dead_code)]

use crate::errors::LauncherError;
use crate::errors::LauncherResult;
use crate::models::MultiInstanceState;

pub const USER_PREFIX: &str = "stfc-";

/// Reserved name for the primary (current-user) account row in instance
/// commands. Never a provisioned service user, so it can't collide with
/// generated altN names — and validation rejects it for manual names.
pub const BASE_INSTANCE_NAME: &str = "base";

/// The OS user the launcher itself runs as.
pub fn current_username() -> LauncherResult<String> {
    #[cfg(target_os = "windows")]
    let var = "USERNAME";
    #[cfg(not(target_os = "windows"))]
    let var = "USER";
    std::env::var(var).map_err(|_| LauncherError::InvalidData {
        context: "resolving current username".into(),
        message: format!("{var} env var not set"),
    })
}

pub fn os_username(name: &str) -> String {
    format!("{USER_PREFIX}{name}")
}

/// ASCII letters (any case) + digits + '-', 1..=16 chars. Short because it becomes an OS
/// username (macOS dscl and Windows both accept far more, but short+simple
/// keeps sudoers wildcards and pgrep patterns safe).
pub fn validate_instance_name(name: &str) -> LauncherResult<()> {
    let ok = !name.is_empty()
        && name.len() <= 16
        && !name.starts_with(USER_PREFIX)
        && !name.eq_ignore_ascii_case(BASE_INSTANCE_NAME)
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(LauncherError::InvalidData {
            context: "validating instance name".into(),
            message: format!("instance name {name:?} must be 1-16 chars of a-z, A-Z, 0-9, '-'"),
        })
    }
}

/// Safety rail (spec FR-8.1): only users this launcher created are ever touched.
pub fn is_managed(mi: &MultiInstanceState, username: &str) -> bool {
    username.starts_with(USER_PREFIX) && mi.instances.iter().any(|i| i.os_username == username)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_simple_names() {
        assert!(validate_instance_name("alt2").is_ok());
        assert!(validate_instance_name("armada-crew").is_ok());
        assert!(validate_instance_name("Alt2").is_ok());
    }

    #[test]
    fn rejects_unsafe_names() {
        for bad in [
            "",
            "a b",
            "../x",
            "stfc-x",
            "base",
            "BASE",
            &"a".repeat(17),
            "x_1",
        ] {
            assert!(validate_instance_name(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn managed_only_when_registered_and_prefixed() {
        let mi = crate::models::MultiInstanceState {
            instances: vec![crate::models::Instance {
                name: "alt2".into(),
                os_username: "stfc-alt2".into(),
                created_at: chrono::Utc::now(),
                last_backup_at: None,
                label: None,
            }],
            ..Default::default()
        };
        assert!(is_managed(&mi, "stfc-alt2"));
        assert!(!is_managed(&mi, "stfc-other"));
        assert!(!is_managed(&mi, "root"));
        assert!(!is_managed(&mi, "ebendler"));
    }
}
