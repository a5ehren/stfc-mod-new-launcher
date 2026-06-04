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
