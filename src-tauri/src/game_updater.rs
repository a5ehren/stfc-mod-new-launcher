use crate::diagnostics::DiagnosticsService;
use crate::errors::{io_context, LauncherError, LauncherResult};
use crate::events::ProgressEvent;
use crate::models::Platform;
use crate::xsolla::{normalize_relative_patch_path, XsollaAction, XsollaPlan};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GameUpdateContext {
    pub game_root: PathBuf,
    pub xsolla_temp_root: PathBuf,
    pub staging_root: PathBuf,
}

struct UpdateWorkspaceCleanup {
    paths: Vec<PathBuf>,
    active: bool,
}

impl UpdateWorkspaceCleanup {
    fn new(context: &GameUpdateContext) -> Self {
        Self {
            paths: vec![
                context.xsolla_temp_root.clone(),
                context.staging_root.clone(),
            ],
            active: true,
        }
    }

    fn cleanup(&mut self) {
        if !self.active {
            return;
        }
        for path in &self.paths {
            if path.exists() {
                let _ = fs::remove_dir_all(path);
            }
        }
        self.active = false;
    }
}

impl Drop for UpdateWorkspaceCleanup {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PatchRule {
    relative_path: String,
    rule: String,
}

fn substitute_paths(value: &str, context: &GameUpdateContext) -> String {
    value
        .replace("$game_path", &context.game_root.to_string_lossy())
        .replace("$temp_path", &context.xsolla_temp_root.to_string_lossy())
}

pub fn xsolla_update_url(installed_version: u32, platform: Platform) -> String {
    let platform = match platform {
        Platform::MacOs => "mac_os",
        Platform::Windows => "windows",
    };
    format!(
        "https://gus.xsolla.com/updates?version={installed_version}&project_id=152033&region=&platform={platform}"
    )
}

pub fn extract_7z_archive(archive: &Path, destination: &Path) -> LauncherResult<()> {
    fs::create_dir_all(destination)
        .map_err(|err| io_context(format!("creating {}", destination.display()), err))?;
    sevenz_rust2::decompress_file(archive, destination).map_err(|err| LauncherError::Operation {
        context: "extracting Xsolla 7z archive".into(),
        message: err.to_string(),
    })
}

pub fn finalize_update(
    staging_root: &Path,
    game_root: &Path,
    pending_deletes: &[PathBuf],
    pending_version: Option<u32>,
) -> LauncherResult<()> {
    copy_directory_contents(staging_root, game_root)?;
    apply_deferred_deletes(staging_root, game_root, pending_deletes)?;
    if let Some(version) = pending_version {
        write_installed_game_version(game_root, version)?;
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, target: &Path) -> LauncherResult<()> {
    fs::create_dir_all(target)
        .map_err(|err| io_context(format!("creating {}", target.display()), err))?;
    for entry in fs::read_dir(source)
        .map_err(|err| io_context(format!("reading {}", source.display()), err))?
    {
        let entry = entry
            .map_err(|err| io_context(format!("reading entry in {}", source.display()), err))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_contents(&source_path, &target_path)?;
        } else {
            // Staged artifacts are created with default 0644 (rsync output file,
            // 7z extraction) and fs::copy propagates source permissions — capture
            // and restore the overwritten file's mode or the patched game binary
            // loses its execute bit (EACCES on next launch).
            let existing_permissions = if target_path.exists() {
                let permissions = fs::metadata(&target_path)
                    .map_err(|err| io_context(format!("reading {}", target_path.display()), err))?
                    .permissions();
                fs::remove_file(&target_path).map_err(|err| {
                    io_context(format!("removing {}", target_path.display()), err)
                })?;
                Some(permissions)
            } else {
                None
            };
            fs::copy(&source_path, &target_path).map_err(|err| {
                io_context(
                    format!(
                        "copying {} to {}",
                        source_path.display(),
                        target_path.display()
                    ),
                    err,
                )
            })?;
            if let Some(permissions) = existing_permissions {
                fs::set_permissions(&target_path, permissions).map_err(|err| {
                    io_context(
                        format!("restoring permissions on {}", target_path.display()),
                        err,
                    )
                })?;
            }
        }
    }
    Ok(())
}

fn apply_deferred_deletes(
    staging_root: &Path,
    game_root: &Path,
    pending_deletes: &[PathBuf],
) -> LauncherResult<()> {
    for relative in pending_deletes {
        if staging_root.join(relative).exists() {
            continue;
        }
        let target = game_root.join(relative);
        if target.exists() {
            fs::remove_file(&target)
                .map_err(|err| io_context(format!("deleting {}", target.display()), err))?;
        }
    }
    Ok(())
}

fn write_installed_game_version(game_root: &Path, version: u32) -> LauncherResult<()> {
    fs::write(game_root.join(".version"), format!("&game={version}")).map_err(|err| {
        io_context(
            format!("writing {}", game_root.join(".version").display()),
            err,
        )
    })
}

pub async fn run_update_plan(
    client: &reqwest::Client,
    plan: &XsollaPlan,
    context: &GameUpdateContext,
    diagnostics: &DiagnosticsService,
    mut progress: impl FnMut(ProgressEvent) + Send,
) -> LauncherResult<()> {
    progress(ProgressEvent::message(
        "gameUpdate",
        "prepare",
        "preparing game update workspace",
    ));
    if context.xsolla_temp_root.exists() {
        fs::remove_dir_all(&context.xsolla_temp_root).map_err(|err| {
            io_context(
                format!("removing {}", context.xsolla_temp_root.display()),
                err,
            )
        })?;
    }
    if context.staging_root.exists() {
        fs::remove_dir_all(&context.staging_root).map_err(|err| {
            io_context(format!("removing {}", context.staging_root.display()), err)
        })?;
    }
    fs::create_dir_all(&context.xsolla_temp_root).map_err(|err| {
        io_context(
            format!("creating {}", context.xsolla_temp_root.display()),
            err,
        )
    })?;
    fs::create_dir_all(&context.staging_root)
        .map_err(|err| io_context(format!("creating {}", context.staging_root.display()), err))?;
    let mut workspace_cleanup = UpdateWorkspaceCleanup::new(context);

    let mut pending_deletes = Vec::new();
    let mut pending_version = None;
    let total_actions = plan.actions.len() as u64;
    let _ = diagnostics.info(
        "game_update",
        &format!(
            "running update plan to version {:?}: {} action(s); game_root={} temp={} staging={}",
            plan.target_version,
            total_actions,
            context.game_root.display(),
            context.xsolla_temp_root.display(),
            context.staging_root.display()
        ),
    );

    for (index, action) in plan.actions.iter().enumerate() {
        progress(ProgressEvent::counted(
            "gameUpdate",
            "action",
            format!("processing Xsolla action {}", index + 1),
            (index + 1) as u64,
            total_actions,
        ));
        match action {
            XsollaAction::Download { url, to, .. } => {
                progress(ProgressEvent::message(
                    "gameUpdate",
                    "download",
                    "downloading game update files".to_string(),
                ));
                let target = PathBuf::from(substitute_paths(to, context));
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|err| io_context(format!("creating {}", parent.display()), err))?;
                }
                let mut response = client
                    .get(url)
                    .send()
                    .await
                    .map_err(|source| LauncherError::Network {
                        context: format!("downloading Xsolla payload {url}"),
                        source,
                    })?
                    .error_for_status()
                    .map_err(|source| LauncherError::Network {
                        context: format!("checking Xsolla payload response {url}"),
                        source,
                    })?;
                let total = response.content_length();
                let mut bytes = Vec::new();
                let mut downloaded: u64 = 0;
                let mut last_pct = 0u64;
                while let Some(chunk) =
                    response
                        .chunk()
                        .await
                        .map_err(|source| LauncherError::Network {
                            context: format!("reading Xsolla payload {url}"),
                            source,
                        })?
                {
                    downloaded += chunk.len() as u64;
                    bytes.extend_from_slice(&chunk);
                    if let Some(total) = total.filter(|&t| t > 0) {
                        let pct = (downloaded * 100 / total).min(100);
                        if pct != last_pct {
                            last_pct = pct;
                            progress(ProgressEvent::message(
                                "gameUpdate",
                                "download",
                                format!("downloading game update files ({pct}%)"),
                            ));
                        }
                    }
                }
                let byte_len = bytes.len();
                fs::write(&target, bytes)
                    .map_err(|err| io_context(format!("writing {}", target.display()), err))?;
                let _ = diagnostics.info(
                    "game_update",
                    &format!(
                        "downloaded {url} -> {} ({byte_len} bytes)",
                        target.display()
                    ),
                );
            }
            XsollaAction::Extract { file, to } => {
                progress(ProgressEvent::message(
                    "gameUpdate",
                    "extract",
                    "extracting game update files".to_string(),
                ));
                let archive = PathBuf::from(substitute_paths(file, context));
                let destination = PathBuf::from(substitute_paths(to, context));
                let _ = diagnostics.info(
                    "game_update",
                    &format!(
                        "extracting {} -> {}",
                        archive.display(),
                        destination.display()
                    ),
                );
                extract_7z_archive(&archive, &destination)?;
            }
            XsollaAction::Patch { patch, .. } => {
                progress(ProgressEvent::message(
                    "gameUpdate",
                    "patch",
                    "applying game update patches".to_string(),
                ));
                let patch_root = PathBuf::from(substitute_paths(patch, context));
                let rules_path = patch_root.join("patchRules.json");
                let rules_text = fs::read_to_string(&rules_path)
                    .map_err(|err| io_context(format!("reading {}", rules_path.display()), err))?;
                let rules: Vec<PatchRule> = serde_json::from_str(&rules_text).map_err(|err| {
                    LauncherError::InvalidData {
                        context: format!("parsing {}", rules_path.display()),
                        message: err.to_string(),
                    }
                })?;
                let _ = diagnostics.info(
                    "game_update",
                    &format!(
                        "applying {} patch rule(s) from {}",
                        rules.len(),
                        patch_root.display()
                    ),
                );
                for rule in rules {
                    let relative = normalize_relative_patch_path(&rule.relative_path)?;
                    if relative.contains("_CodeSignature") {
                        continue;
                    }
                    let staged_target = context.staging_root.join(&relative);
                    let source_path = context.game_root.join(&relative);
                    let patch_path = patch_root.join(&relative);
                    match rule.rule.as_str() {
                        "patch" => {
                            let basis = if staged_target.exists() {
                                staged_target.clone()
                            } else {
                                source_path
                            };
                            let _ = diagnostics.info(
                                "game_update",
                                &format!(
                                    "rule patch {relative}: basis={} patch_bytes={}",
                                    basis.display(),
                                    fs::metadata(&patch_path).map(|m| m.len()).unwrap_or(0)
                                ),
                            );
                            let output = staged_target.with_extension("patching");
                            if let Some(parent) = staged_target.parent() {
                                fs::create_dir_all(parent).map_err(|err| {
                                    io_context(format!("creating {}", parent.display()), err)
                                })?;
                            }
                            if let Err(err) =
                                crate::rsync_patch::apply_rsync_patch(&basis, &patch_path, &output)
                            {
                                let _ = diagnostics.error(
                                    "game_update",
                                    &format!("rsync patch failed for {relative}: {err}"),
                                );
                                return Err(err);
                            }
                            fs::rename(&output, &staged_target).map_err(|err| {
                                io_context(
                                    format!(
                                        "renaming {} to {}",
                                        output.display(),
                                        staged_target.display()
                                    ),
                                    err,
                                )
                            })?;
                        }
                        "create" => {
                            if let Some(parent) = staged_target.parent() {
                                fs::create_dir_all(parent).map_err(|err| {
                                    io_context(format!("creating {}", parent.display()), err)
                                })?;
                            }
                            if !staged_target.exists() {
                                fs::write(&staged_target, []).map_err(|err| {
                                    io_context(format!("creating {}", staged_target.display()), err)
                                })?;
                            }
                        }
                        "copy" => {
                            if let Some(parent) = staged_target.parent() {
                                fs::create_dir_all(parent).map_err(|err| {
                                    io_context(format!("creating {}", parent.display()), err)
                                })?;
                            }
                            fs::copy(&patch_path, &staged_target).map_err(|err| {
                                io_context(
                                    format!(
                                        "copying {} to {}",
                                        patch_path.display(),
                                        staged_target.display()
                                    ),
                                    err,
                                )
                            })?;
                        }
                        "delete" => pending_deletes.push(PathBuf::from(relative)),
                        other => {
                            return Err(LauncherError::InvalidData {
                                context: "applying Xsolla patch rule".into(),
                                message: format!("unknown patch rule {other}"),
                            });
                        }
                    }
                }
            }
            XsollaAction::Wait => {
                progress(ProgressEvent::message(
                    "gameUpdate",
                    "wait",
                    "waiting for the update plan to continue",
                ));
            }
            XsollaAction::Version { version } => {
                pending_version = Some(*version);
                progress(ProgressEvent::message(
                    "gameUpdate",
                    "version",
                    format!("deferring installed version write to {version}"),
                ));
            }
        }
    }

    progress(ProgressEvent::message(
        "gameUpdate",
        "finalizing",
        "copying staged files into the game directory",
    ));
    let _ = diagnostics.info(
        "game_update",
        &format!(
            "finalizing into {}: {} deferred delete(s), pending_version={:?}",
            context.game_root.display(),
            pending_deletes.len(),
            pending_version
        ),
    );
    finalize_update(
        &context.staging_root,
        &context.game_root,
        &pending_deletes,
        pending_version,
    )?;

    progress(ProgressEvent::message(
        "gameUpdate",
        "cleanup",
        "removing temporary update files",
    ));
    workspace_cleanup.cleanup();

    progress(ProgressEvent::message(
        "gameUpdate",
        "complete",
        "game update completed",
    ));
    Ok(())
}

pub async fn fetch_update_plan(
    client: &reqwest::Client,
    platform: Platform,
    installed_version: u32,
) -> LauncherResult<Option<XsollaPlan>> {
    let url = xsolla_update_url(installed_version, platform);
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|source| LauncherError::Network {
            context: format!("fetching Xsolla update plan from {url}"),
            source,
        })?
        .error_for_status()
        .map_err(|source| LauncherError::Network {
            context: format!("checking Xsolla update plan response from {url}"),
            source,
        })?;
    let xml = response
        .text()
        .await
        .map_err(|source| LauncherError::Network {
            context: format!("reading Xsolla update plan from {url}"),
            source,
        })?;
    let plan = crate::xsolla::parse_update_plan(&xml)?;
    Ok(actionable_plan(plan, installed_version))
}

/// A plan is actionable only when it carries a target version newer than the
/// installed one. Xsolla signals "already up to date" with a bare
/// `version="-1"` marker, which yields no target version at all.
fn actionable_plan(plan: XsollaPlan, installed_version: u32) -> Option<XsollaPlan> {
    match plan.target_version {
        Some(target_version) if target_version > installed_version => Some(plan),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_version_after_staged_copy() {
        let root = tempfile::tempdir().expect("tempdir");
        let game = root.path().join("game");
        let staging = root.path().join("staging");
        std::fs::create_dir_all(&game).expect("game");
        std::fs::create_dir_all(&staging).expect("staging");
        std::fs::write(staging.join("prime.exe"), b"patched").expect("staged");

        finalize_update(&staging, &game, &[], Some(169)).expect("finalize");

        assert_eq!(
            std::fs::read(game.join("prime.exe")).expect("patched"),
            b"patched"
        );
        assert_eq!(
            std::fs::read_to_string(game.join(".version")).expect("version"),
            "&game=169"
        );
    }

    #[cfg(unix)]
    #[test]
    fn finalize_preserves_overwritten_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().expect("tempdir");
        let game = root.path().join("game");
        let staging = root.path().join("staging");
        std::fs::create_dir_all(&game).expect("game");
        std::fs::create_dir_all(&staging).expect("staging");
        let binary = game.join("Star Trek Fleet Command");
        std::fs::write(&binary, b"old").expect("binary");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        // Staged artifacts land 0644 (rsync File::create / 7z extraction).
        std::fs::write(staging.join("Star Trek Fleet Command"), b"new").expect("staged");

        finalize_update(&staging, &game, &[], None).expect("finalize");

        assert_eq!(std::fs::read(&binary).expect("content"), b"new");
        assert_eq!(
            std::fs::metadata(&binary)
                .expect("meta")
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }

    #[test]
    fn xsolla_update_url_uses_platform_and_version() {
        assert_eq!(
            xsolla_update_url(168, Platform::MacOs),
            "https://gus.xsolla.com/updates?version=168&project_id=152033&region=&platform=mac_os"
        );
        assert_eq!(
            xsolla_update_url(168, Platform::Windows),
            "https://gus.xsolla.com/updates?version=168&project_id=152033&region=&platform=windows"
        );
    }

    /// Hits the live Xsolla update service; run explicitly with
    /// `cargo test -- --ignored live_xsolla`.
    #[test]
    #[ignore = "requires network access"]
    fn live_xsolla_reports_current_version_as_up_to_date() {
        let client = reqwest::Client::new();
        let plan = tauri::async_runtime::block_on(fetch_update_plan(&client, Platform::MacOs, 189))
            .expect("live fetch");
        assert!(plan.is_none(), "version 189 should be up to date");

        let plan = tauri::async_runtime::block_on(fetch_update_plan(&client, Platform::MacOs, 185))
            .expect("live fetch");
        assert!(
            plan.and_then(|plan| plan.target_version)
                .is_some_and(|target| target > 185),
            "version 185 should have a patch chain to a newer version"
        );
    }

    #[test]
    fn actionable_plan_requires_newer_target_version() {
        let plan = |target_version: Option<u32>| XsollaPlan {
            target_version,
            actions: vec![],
        };

        // A newer target is actionable.
        assert!(actionable_plan(plan(Some(190)), 189).is_some());
        // Same-version and older targets are not.
        assert!(actionable_plan(plan(Some(189)), 189).is_none());
        assert!(actionable_plan(plan(Some(188)), 189).is_none());
        // No target version (the version="-1" up-to-date marker) is not.
        assert!(actionable_plan(plan(None), 189).is_none());
    }

    #[test]
    fn run_update_plan_cleans_workspace_after_action_error() {
        let root = tempfile::tempdir().expect("tempdir");
        let game = root.path().join("game");
        let xsolla_temp = root.path().join("xsolla-temp");
        let staging = root.path().join("staging");
        std::fs::create_dir_all(&game).expect("game dir");
        let plan = XsollaPlan {
            target_version: Some(169),
            actions: vec![XsollaAction::Extract {
                file: "$temp_path/missing.7z".into(),
                to: "$game_path".into(),
            }],
        };
        let context = GameUpdateContext {
            game_root: game,
            xsolla_temp_root: xsolla_temp.clone(),
            staging_root: staging.clone(),
        };
        let client = reqwest::Client::new();

        let diagnostics = crate::diagnostics::DiagnosticsService::new(root.path().join("logs"));
        let result = tauri::async_runtime::block_on(run_update_plan(
            &client,
            &plan,
            &context,
            &diagnostics,
            |_| {},
        ));

        assert!(result.is_err());
        assert!(!xsolla_temp.exists(), "xsolla temp root should be removed");
        assert!(!staging.exists(), "staging root should be removed");
    }

    #[test]
    fn run_update_plan_logs_plan_boundaries() {
        let root = tempfile::tempdir().expect("tempdir");
        let game = root.path().join("game");
        std::fs::create_dir_all(&game).expect("game dir");
        let diagnostics = crate::diagnostics::DiagnosticsService::new(root.path().join("logs"));
        let plan = XsollaPlan {
            target_version: Some(170),
            actions: vec![XsollaAction::Version { version: 170 }],
        };
        let context = GameUpdateContext {
            game_root: game,
            xsolla_temp_root: root.path().join("xsolla-temp"),
            staging_root: root.path().join("staging"),
        };
        let client = reqwest::Client::new();
        tauri::async_runtime::block_on(run_update_plan(
            &client,
            &plan,
            &context,
            &diagnostics,
            |_| {},
        ))
        .expect("plan");

        let log = std::fs::read_to_string(diagnostics.log_file()).expect("log");
        assert!(log.contains("running update plan to version Some(170)"));
        assert!(log.contains("finalizing into"));
    }
}
