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
        Platform::LinuxWine => "windows", // WINE uses Windows binaries
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
            if target_path.exists() {
                fs::remove_file(&target_path).map_err(|err| {
                    io_context(format!("removing {}", target_path.display()), err)
                })?;
            }
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

    let mut pending_deletes = Vec::new();
    let mut pending_version = None;
    let total_actions = plan.actions.len() as u64;

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
                    format!("downloading update payload from {url}"),
                ));
                let target = PathBuf::from(substitute_paths(to, context));
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|err| io_context(format!("creating {}", parent.display()), err))?;
                }
                let bytes = client
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
                    })?
                    .bytes()
                    .await
                    .map_err(|source| LauncherError::Network {
                        context: format!("reading Xsolla payload {url}"),
                        source,
                    })?;
                fs::write(&target, bytes)
                    .map_err(|err| io_context(format!("writing {}", target.display()), err))?;
            }
            XsollaAction::Extract { file, to } => {
                progress(ProgressEvent::message(
                    "gameUpdate",
                    "extract",
                    format!("extracting {file}"),
                ));
                let archive = PathBuf::from(substitute_paths(file, context));
                let destination = PathBuf::from(substitute_paths(to, context));
                extract_7z_archive(&archive, &destination)?;
            }
            XsollaAction::Patch { patch, .. } => {
                progress(ProgressEvent::message(
                    "gameUpdate",
                    "patch",
                    format!("applying patches from {patch}"),
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
                            let output = staged_target.with_extension("patching");
                            if let Some(parent) = staged_target.parent() {
                                fs::create_dir_all(parent).map_err(|err| {
                                    io_context(format!("creating {}", parent.display()), err)
                                })?;
                            }
                            crate::rsync_patch::apply_rsync_patch(&basis, &patch_path, &output)?;
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
    if context.xsolla_temp_root.exists() {
        let _ = fs::remove_dir_all(&context.xsolla_temp_root);
    }
    if context.staging_root.exists() {
        let _ = fs::remove_dir_all(&context.staging_root);
    }

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
    let xml = response.text().await.map_err(|source| LauncherError::Network {
        context: format!("reading Xsolla update plan from {url}"),
        source,
    })?;
    let plan = crate::xsolla::parse_update_plan(&xml)?;
    if let Some(target_version) = plan.target_version {
        if target_version <= installed_version {
            return Ok(None);
        }
    }
    Ok(Some(plan))
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
        assert_eq!(
            xsolla_update_url(168, Platform::LinuxWine),
            "https://gus.xsolla.com/updates?version=168&project_id=152033&region=&platform=windows"
        );
    }
}
