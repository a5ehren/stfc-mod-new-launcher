use crate::errors::{LauncherError, LauncherResult};
use serde::Serialize;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherUpdateInfo {
    pub version: String,
    pub body: Option<String>,
}

pub async fn check_for_launcher_update(
    app: tauri::AppHandle,
) -> LauncherResult<Option<LauncherUpdateInfo>> {
    let update = app
        .updater()
        .map_err(|err| LauncherError::Operation {
            context: "creating launcher updater".into(),
            message: err.to_string(),
        })?
        .check()
        .await
        .map_err(|err| LauncherError::Operation {
            context: "checking launcher update".into(),
            message: err.to_string(),
        })?;

    Ok(update.map(|update| LauncherUpdateInfo {
        version: update.version.clone(),
        body: update.body.clone(),
    }))
}

/// Downloads and installs the pending launcher update, reporting download
/// progress via `on_chunk(downloaded_bytes, total_bytes)`. Returns `true` when
/// an update was installed; the new version takes effect on the next launch.
pub async fn install_launcher_update(
    app: tauri::AppHandle,
    on_chunk: impl Fn(u64, Option<u64>) + Send,
) -> LauncherResult<bool> {
    let update = app
        .updater()
        .map_err(|err| LauncherError::Operation {
            context: "creating launcher updater".into(),
            message: err.to_string(),
        })?
        .check()
        .await
        .map_err(|err| LauncherError::Operation {
            context: "checking launcher update".into(),
            message: err.to_string(),
        })?;

    let Some(update) = update else {
        return Ok(false);
    };

    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk_length, content_length| {
                downloaded += chunk_length as u64;
                on_chunk(downloaded, content_length);
            },
            || {},
        )
        .await
        .map_err(|err| LauncherError::Operation {
            context: "installing launcher update".into(),
            message: err.to_string(),
        })?;

    Ok(true)
}
