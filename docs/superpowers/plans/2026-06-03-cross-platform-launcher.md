# Cross-Platform STFC Mod Launcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build the approved Tauri 2 macOS/Windows launcher runtime, with LCARS UI, managed mod updates, config editing, game updates, migration, launch injection, diagnostics, and self-updates.

**Architecture:** Vue renders the LCARS main window and separate config editor window. Rust owns filesystem, network, update, migration, launch, logging, and Tauri command boundaries. Services are small Rust modules behind typed commands, with frontend wrappers and focused Vue views.

**Tech Stack:** Tauri 2, Rust 2021, Vue 3, TypeScript, Vite, Vitest, `reqwest`, `quick-xml`, `sha2`, `tar`, `zstd`, `sevenz-rust2`, `librsync`, `tempfile`, `thiserror`, Tauri updater/process/dialog/opener plugins.

---

## Scope Check

The approved design covers several subsystems, but they are tightly coupled around one launcher runtime and shared state. This plan keeps them in one implementation sequence and commits after each coherent vertical slice.

CI/release automation, installer design, notarization workflow, and full STFC game-update rollback stay out of this plan.

## Current Status

Implemented and verified in the repository:

- Tooling, Tauri plugins, and Vitest harness
- Shared Rust models, errors, events, and app state
- Managed storage and diagnostics
- Game locator and manual path validation
- GitHub release selection and mod install pipeline
- Migration and Windows fallback cleanup
- Config TOML model and raw config service
- Frontend command wrappers and type mirrors
- LCARS main window and config editor window
- Xsolla parser, game update finalization, and `update_game`
- Mod update flow, checksum verification, extraction, and atomic install
- Frontend progress-event subscription and status updates
- Launch service
- Launcher self-update check service

Current status:

- The launcher implementation plan is complete and recorded in commit d9cb5af

## File Structure

Rust backend files:

- `src-tauri/src/lib.rs` wires Tauri plugins, state, commands, and setup checks.
- `src-tauri/src/app_state.rs` owns shared app state and service construction.
- `src-tauri/src/errors.rs` defines serializable launcher errors.
- `src-tauri/src/models.rs` defines DTOs shared by commands and frontend.
- `src-tauri/src/events.rs` defines status/progress event payloads.
- `src-tauri/src/storage.rs` resolves managed paths and persisted state.
- `src-tauri/src/diagnostics.rs` writes structured log files and opens log location.
- `src-tauri/src/game_locator.rs` detects and validates STFC paths.
- `src-tauri/src/github_releases.rs` fetches and selects GitHub release assets.
- `src-tauri/src/mod_manager.rs` downloads, verifies, extracts, and installs mod libraries.
- `src-tauri/src/migration.rs` moves legacy config/logs and handles stale Windows DLL decisions.
- `src-tauri/src/config_service.rs` reads/writes active config paths.
- `src-tauri/src/xsolla.rs` parses Xsolla update XML into typed actions.
- `src-tauri/src/game_updater.rs` applies Xsolla game updates and preserves finalization ordering.
- `src-tauri/src/rsync_patch.rs` wraps librsync patch application.
- `src-tauri/src/launch.rs` builds and runs platform-specific launch flows.
- `src-tauri/src/self_update.rs` wraps Tauri updater checks and installs.
- `src-tauri/src/commands.rs` exposes Tauri commands.

Frontend files:

- `src/App.vue` selects the current window view by Tauri window label.
- `src/main.ts` mounts Vue and global styles.
- `src/types/launcher.ts` mirrors Rust command DTOs.
- `src/lib/commands.ts` wraps `invoke` and event listening.
- `src/lib/config/defaults.ts` ports `../modconfig` defaults.
- `src/lib/config/types.ts` ports config TypeScript interfaces.
- `src/lib/config/definitions.ts` ports field metadata exports.
- `src/lib/config/toml.ts` ports TOML parse/generate behavior.
- `src/components/lcars/LcarsShell.vue` provides frame/chrome.
- `src/components/lcars/LcarsButton.vue` provides LCARS action buttons.
- `src/components/lcars/DataCascade.vue` provides header decoration.
- `src/components/StatusStrip.vue` shows progress and warnings.
- `src/views/MainLauncher.vue` renders main launcher controls.
- `src/views/ConfigEditor.vue` renders separate config editor window.
- `src/components/config/ConfigSection.vue` renders a section of fields.
- `src/components/config/ConfigField.vue` renders a single typed field.
- `src/components/config/SyncTargetsEditor.vue` renders dynamic sync targets.

Test files:

- `src-tauri/src/*` unit tests colocated in each Rust module.
- `src/lib/config/*.test.ts` for TypeScript config parity.
- `src/views/*.test.ts` and `src/components/**/*.test.ts` for UI behavior.
- `src-tauri/tests/fixtures/` for release JSON, INI, XML, and patch fixture data.

## Task 1: Tooling, Plugins, And Test Harness

**Files:**
- Modify: `package.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Create: `vitest.config.ts`
- Create: `src/test/setup.ts`

- [x] **Step 1: Add frontend test and Tauri plugin dependencies**

Run:

```bash
pnpm add @tauri-apps/plugin-dialog@^2 @tauri-apps/plugin-process@^2 @tauri-apps/plugin-updater@^2
pnpm add -D vitest @vue/test-utils jsdom
```

Expected: `package.json` and `pnpm-lock.yaml` include the new packages.

- [x] **Step 2: Add Rust dependencies**

Run:

```bash
cd src-tauri
cargo add tauri-plugin-dialog@2 tauri-plugin-process@2 tauri-plugin-updater@2
cargo add thiserror reqwest --features reqwest/json,reqwest/rustls-tls
cargo add sha2 hex tar zstd tempfile quick-xml serde_with chrono directories sevenz-rust2@0.21.0
cargo add librsync@0.2.5
```

Expected: `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` include the new crates.

- [x] **Step 3: Create Vitest config**

Write `vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    globals: true,
  },
  resolve: {
    alias: {
      "@": new URL("./src", import.meta.url).pathname,
    },
  },
});
```

- [x] **Step 4: Create test setup**

Write `src/test/setup.ts`:

```ts
import { vi } from "vitest";

Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {},
  configurable: true,
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => vi.fn()),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn(() => ({ label: "main" })),
  WebviewWindow: vi.fn(),
}));
```

- [x] **Step 5: Add package scripts**

Modify `package.json` scripts:

```json
{
  "dev": "vite",
  "build": "vue-tsc --noEmit && vite build",
  "preview": "vite preview",
  "test": "vitest run",
  "test:watch": "vitest",
  "tauri": "tauri"
}
```

- [x] **Step 6: Configure Tauri plugins and windows**

Modify `src-tauri/src/lib.rs` to initialize plugins:

```rust
mod app_state;
mod commands;
mod diagnostics;
mod errors;
mod events;
mod models;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_launcher_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Modify `src-tauri/tauri.conf.json` window section:

```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "STFC Community Mod",
        "width": 760,
        "height": 520,
        "resizable": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "createUpdaterArtifacts": true
  }
}
```

Keep the existing icon list inside `bundle`.

- [x] **Step 7: Configure default capabilities**

Modify `src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default launcher capability",
  "windows": ["main", "config-editor"],
  "permissions": [
    "core:default",
    "core:webview:allow-create-webview-window",
    "opener:default",
    "dialog:default",
    "process:default",
    "updater:default"
  ]
}
```

- [x] **Step 8: Verify baseline**

Run:

```bash
pnpm test
pnpm build
cd src-tauri && cargo test
```

Expected: Vitest finds no tests or passes setup, frontend build passes, Rust tests pass.

- [x] **Step 9: Commit**

```bash
git add package.json pnpm-lock.yaml vitest.config.ts src/test/setup.ts src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "chore: add launcher tooling and plugins"
```

## Task 2: Shared Rust Models, Errors, Events, And App State

**Files:**
- Create: `src-tauri/src/errors.rs`
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/events.rs`
- Create: `src-tauri/src/app_state.rs`
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write model serialization tests**

Create the test module inside `src-tauri/src/models.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_launcher_status() {
        let status = LauncherStatus {
            game: GameStatus {
                known: true,
                path: Some("/tmp/game".into()),
                installed_version: Some(168),
                update_available: true,
            },
            mod_status: ModStatus {
                installed: false,
                installed_version: None,
                latest_version: Some("v1.2.3".into()),
                channel: ModChannel::Stable,
                update_available: true,
                launch_mode: LaunchMode::Managed,
            },
            launcher_update_available: false,
        };

        let json = serde_json::to_value(status).expect("status serializes");
        assert_eq!(json["game"]["known"], true);
        assert_eq!(json["modStatus"]["channel"], "stable");
        assert_eq!(json["modStatus"]["launchMode"], "managed");
    }
}
```

Run:

```bash
cd src-tauri && cargo test models::tests::serializes_launcher_status
```

Expected: FAIL because the types do not exist.

- [x] **Step 2: Implement shared models**

Write `src-tauri/src/models.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    MacOs,
    Windows,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModChannel {
    Stable,
    Prerelease,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LaunchMode {
    Managed,
    WindowsProxyDll,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub known: bool,
    pub path: Option<String>,
    pub installed_version: Option<u32>,
    pub update_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModStatus {
    pub installed: bool,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub channel: ModChannel,
    pub update_available: bool,
    pub launch_mode: LaunchMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    pub game: GameStatus,
    pub mod_status: ModStatus,
    pub launcher_update_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    pub game_path: Option<PathBuf>,
    pub mod_channel: ModChannel,
    pub installed_mod_version: Option<String>,
    pub installed_mod_checksum: Option<String>,
    pub launch_mode: LaunchMode,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            game_path: None,
            mod_channel: ModChannel::Stable,
            installed_mod_version: None,
            installed_mod_checksum: None,
            launch_mode: LaunchMode::Managed,
        }
    }
}
```

- [x] **Step 3: Implement serializable error type**

Write `src-tauri/src/errors.rs`:

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("I/O error while {context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("network error while {context}: {source}")]
    Network {
        context: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("invalid data while {context}: {message}")]
    InvalidData { context: String, message: String },
    #[error("operation failed while {context}: {message}")]
    Operation { context: String, message: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDto {
    pub kind: String,
    pub message: String,
}

impl From<LauncherError> for ErrorDto {
    fn from(error: LauncherError) -> Self {
        let kind = match &error {
            LauncherError::Io { .. } => "io",
            LauncherError::Network { .. } => "network",
            LauncherError::InvalidData { .. } => "invalidData",
            LauncherError::Operation { .. } => "operation",
        };
        Self {
            kind: kind.to_string(),
            message: error.to_string(),
        }
    }
}

pub type LauncherResult<T> = Result<T, LauncherError>;

pub fn io_context(context: impl Into<String>, source: std::io::Error) -> LauncherError {
    LauncherError::Io {
        context: context.into(),
        source,
    }
}
```

- [x] **Step 4: Implement event payloads**

Write `src-tauri/src/events.rs`:

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub operation: String,
    pub phase: String,
    pub message: String,
    pub current: Option<u64>,
    pub total: Option<u64>,
}

impl ProgressEvent {
    pub fn message(operation: &str, phase: &str, message: impl Into<String>) -> Self {
        Self {
            operation: operation.to_string(),
            phase: phase.to_string(),
            message: message.into(),
            current: None,
            total: None,
        }
    }

    pub fn counted(
        operation: &str,
        phase: &str,
        message: impl Into<String>,
        current: u64,
        total: u64,
    ) -> Self {
        Self {
            operation: operation.to_string(),
            phase: phase.to_string(),
            message: message.into(),
            current: Some(current),
            total: Some(total),
        }
    }
}
```

- [x] **Step 5: Implement app state and status command**

Write `src-tauri/src/app_state.rs`:

```rust
use crate::models::{GameStatus, LaunchMode, LauncherStatus, ModChannel, ModStatus};
use std::sync::Mutex;

pub struct AppState {
    pub status: Mutex<LauncherStatus>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(LauncherStatus {
                game: GameStatus {
                    known: false,
                    path: None,
                    installed_version: None,
                    update_available: false,
                },
                mod_status: ModStatus {
                    installed: false,
                    installed_version: None,
                    latest_version: None,
                    channel: ModChannel::Stable,
                    update_available: false,
                    launch_mode: LaunchMode::Managed,
                },
                launcher_update_available: false,
            }),
        }
    }
}
```

Write `src-tauri/src/commands.rs`:

```rust
use crate::app_state::AppState;
use crate::errors::ErrorDto;
use crate::models::LauncherStatus;
use tauri::State;

pub type CommandResult<T> = Result<T, ErrorDto>;

#[tauri::command]
pub fn get_launcher_status(state: State<'_, AppState>) -> CommandResult<LauncherStatus> {
    let guard = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    Ok(guard.clone())
}
```

Modify `src-tauri/src/lib.rs` to manage state:

```rust
.manage(app_state::AppState::new())
```

Place it before `.invoke_handler(...)`.

- [x] **Step 6: Verify**

Run:

```bash
cd src-tauri && cargo test
pnpm build
```

Expected: Rust tests pass and frontend build still passes.

- [x] **Step 7: Commit**

```bash
git add src-tauri/src/errors.rs src-tauri/src/models.rs src-tauri/src/events.rs src-tauri/src/app_state.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add launcher service contracts"
```

## Task 3: Managed Storage And Diagnostics

**Files:**
- Create: `src-tauri/src/storage.rs`
- Create: `src-tauri/src/diagnostics.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write storage tests**

Add to `src-tauri/src/storage.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_paths_are_under_app_root() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());

        assert_eq!(paths.config_file, root.path().join("community_patch_settings.toml"));
        assert_eq!(paths.mods_dir, root.path().join("mods"));
        assert_eq!(paths.logs_dir, root.path().join("logs"));
        assert_eq!(paths.state_file, root.path().join("state.json"));
    }

    #[test]
    fn state_round_trips() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = ManagedPaths::from_root(root.path().to_path_buf());
        let mut state = crate::models::PersistedState::default();
        state.installed_mod_version = Some("v1.0.0".into());

        save_state(&paths, &state).expect("save state");
        let loaded = load_state(&paths).expect("load state");

        assert_eq!(loaded.installed_mod_version.as_deref(), Some("v1.0.0"));
    }
}
```

Run:

```bash
cd src-tauri && cargo test storage::tests
```

Expected: FAIL because storage code does not exist.

- [x] **Step 2: Implement managed paths and state persistence**

Write `src-tauri/src/storage.rs`:

```rust
use crate::errors::{io_context, LauncherResult};
use crate::models::PersistedState;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ManagedPaths {
    pub root: PathBuf,
    pub mods_dir: PathBuf,
    pub staging_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub config_file: PathBuf,
    pub state_file: PathBuf,
}

impl ManagedPaths {
    pub fn discover() -> LauncherResult<Self> {
        let project_dirs = ProjectDirs::from("com", "stfcmod", "launcher").ok_or_else(|| {
            crate::errors::LauncherError::Operation {
                context: "resolving app data directory".into(),
                message: "platform did not provide an app data directory".into(),
            }
        })?;
        Ok(Self::from_root(project_dirs.data_local_dir().to_path_buf()))
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self {
            mods_dir: root.join("mods"),
            staging_dir: root.join("staging"),
            logs_dir: root.join("logs"),
            config_file: root.join("community_patch_settings.toml"),
            state_file: root.join("state.json"),
            root,
        }
    }

    pub fn ensure_dirs(&self) -> LauncherResult<()> {
        for path in [&self.root, &self.mods_dir, &self.staging_dir, &self.logs_dir] {
            fs::create_dir_all(path).map_err(|err| io_context(format!("creating {}", path.display()), err))?;
        }
        Ok(())
    }
}

pub fn load_state(paths: &ManagedPaths) -> LauncherResult<PersistedState> {
    if !paths.state_file.exists() {
        return Ok(PersistedState::default());
    }
    let text = fs::read_to_string(&paths.state_file)
        .map_err(|err| io_context(format!("reading {}", paths.state_file.display()), err))?;
    serde_json::from_str(&text).map_err(|err| crate::errors::LauncherError::InvalidData {
        context: format!("parsing {}", paths.state_file.display()),
        message: err.to_string(),
    })
}

pub fn save_state(paths: &ManagedPaths, state: &PersistedState) -> LauncherResult<()> {
    paths.ensure_dirs()?;
    let text = serde_json::to_string_pretty(state).map_err(|err| {
        crate::errors::LauncherError::InvalidData {
            context: "serializing launcher state".into(),
            message: err.to_string(),
        }
    })?;
    fs::write(&paths.state_file, text)
        .map_err(|err| io_context(format!("writing {}", paths.state_file.display()), err))
}
```

- [x] **Step 3: Write diagnostics tests**

Add to `src-tauri/src/diagnostics.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_log_line() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = crate::storage::ManagedPaths::from_root(root.path().to_path_buf());
        paths.ensure_dirs().expect("dirs");
        let diagnostics = DiagnosticsService::new(paths.logs_dir.clone());

        diagnostics.info("startup", "launcher started").expect("write log");
        let log_text = std::fs::read_to_string(diagnostics.log_file()).expect("read log");

        assert!(log_text.contains("\"category\":\"startup\""));
        assert!(log_text.contains("\"message\":\"launcher started\""));
    }
}
```

Run:

```bash
cd src-tauri && cargo test diagnostics::tests
```

Expected: FAIL because diagnostics code does not exist.

- [x] **Step 4: Implement diagnostics service**

Write `src-tauri/src/diagnostics.rs`:

```rust
use crate::errors::{io_context, LauncherResult};
use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogEntry<'a> {
    timestamp: String,
    level: &'a str,
    category: &'a str,
    message: &'a str,
}

#[derive(Debug, Clone)]
pub struct DiagnosticsService {
    logs_dir: PathBuf,
    log_file: PathBuf,
}

impl DiagnosticsService {
    pub fn new(logs_dir: PathBuf) -> Self {
        Self {
            log_file: logs_dir.join("launcher.log.jsonl"),
            logs_dir,
        }
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    pub fn log_file(&self) -> &Path {
        &self.log_file
    }

    pub fn info(&self, category: &str, message: &str) -> LauncherResult<()> {
        self.write("info", category, message)
    }

    pub fn warn(&self, category: &str, message: &str) -> LauncherResult<()> {
        self.write("warn", category, message)
    }

    pub fn error(&self, category: &str, message: &str) -> LauncherResult<()> {
        self.write("error", category, message)
    }

    fn write(&self, level: &str, category: &str, message: &str) -> LauncherResult<()> {
        fs::create_dir_all(&self.logs_dir)
            .map_err(|err| io_context(format!("creating {}", self.logs_dir.display()), err))?;
        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level,
            category,
            message,
        };
        let line = serde_json::to_string(&entry).map_err(|err| {
            crate::errors::LauncherError::InvalidData {
                context: "serializing log entry".into(),
                message: err.to_string(),
            }
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .map_err(|err| io_context(format!("opening {}", self.log_file.display()), err))?;
        writeln!(file, "{line}")
            .map_err(|err| io_context(format!("writing {}", self.log_file.display()), err))
    }
}
```

- [x] **Step 5: Wire state and open logs command**

Extend `AppState`:

```rust
use crate::diagnostics::DiagnosticsService;
use crate::storage::{load_state, ManagedPaths};

pub struct AppState {
    pub paths: ManagedPaths,
    pub persisted: Mutex<crate::models::PersistedState>,
    pub diagnostics: DiagnosticsService,
    pub status: Mutex<LauncherStatus>,
}
```

Update `AppState::new()` to return `LauncherResult<Self>` and initialize `ManagedPaths::discover()`, `paths.ensure_dirs()`, `load_state(&paths)`, and `DiagnosticsService::new(paths.logs_dir.clone())`.

Add command in `commands.rs`:

```rust
#[tauri::command]
pub async fn open_logs(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    let path = state.diagnostics.logs_dir().to_path_buf();
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|err| ErrorDto {
            kind: "openLogs".into(),
            message: err.to_string(),
        })?;
    Ok(())
}
```

Register `open_logs` in `tauri::generate_handler!`.

- [x] **Step 6: Verify**

Run:

```bash
cd src-tauri && cargo test storage::tests diagnostics::tests
pnpm build
```

Expected: Storage and diagnostics tests pass, frontend build passes.

- [x] **Step 7: Commit**

```bash
git add src-tauri/src/storage.rs src-tauri/src/diagnostics.rs src-tauri/src/app_state.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add managed storage and diagnostics"
```

## Task 4: Game Locator

**Files:**
- Create: `src-tauri/src/game_locator.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write locator tests**

Create `src-tauri/src/game_locator.rs` with tests:

```rust
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
        assert_eq!(
            parsed.temp_path.as_deref(),
            Some("/Users/test/STFC/tmp")
        );
    }

    #[test]
    fn validates_game_root_by_platform_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let game_root = root.path();
        std::fs::write(game_root.join(".version"), "&game=168").expect("version");
        std::fs::create_dir_all(game_root.join("Star Trek Fleet Command.app/Contents/MacOS")).expect("mac dirs");
        std::fs::write(
            game_root.join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command"),
            "",
        )
        .expect("mac executable");

        assert!(is_valid_game_root(game_root, crate::models::Platform::MacOs));
    }
}
```

Run:

```bash
cd src-tauri && cargo test game_locator::tests
```

Expected: FAIL because the locator does not exist.

- [x] **Step 2: Implement game locator parser and validation**

Write `src-tauri/src/game_locator.rs`:

```rust
use crate::errors::{io_context, LauncherError, LauncherResult};
use crate::models::Platform;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherSettings {
    pub game_path: Option<String>,
    pub temp_path: Option<String>,
}

pub fn parse_launcher_settings(text: &str) -> LauncherResult<LauncherSettings> {
    let mut in_general = false;
    let mut game_path = None;
    let mut temp_path = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_general = line == "[General]";
            continue;
        }
        if !in_general {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let normalized = normalize_xsolla_path(value.trim());
        match key.trim() {
            "152033..GAME_PATH" => game_path = Some(normalized),
            "152033..GAME_TEMP_PATH" => temp_path = Some(normalized),
            _ => {}
        }
    }

    Ok(LauncherSettings { game_path, temp_path })
}

fn normalize_xsolla_path(value: &str) -> String {
    if value.starts_with("//") {
        value.trim_start_matches('/').to_string().insert_prefix("/")
    } else {
        value.to_string()
    }
}

trait PrefixInsert {
    fn insert_prefix(self, prefix: &str) -> String;
}

impl PrefixInsert for String {
    fn insert_prefix(self, prefix: &str) -> String {
        format!("{prefix}{self}")
    }
}

pub fn is_valid_game_root(path: &Path, platform: Platform) -> bool {
    if !path.is_dir() {
        return false;
    }
    match platform {
        Platform::MacOs => path
            .join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command")
            .exists(),
        Platform::Windows => path.join("prime.exe").exists(),
    }
}

pub fn installed_version(game_root: &Path) -> Option<u32> {
    let text = fs::read_to_string(game_root.join(".version")).ok()?;
    let (_, value) = text.split_once('=')?;
    value.trim().parse().ok()
}

#[derive(Debug, Clone)]
pub struct GameLocator {
    platform: Platform,
}

impl GameLocator {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn validate_manual_root(&self, path: PathBuf) -> LauncherResult<PathBuf> {
        if is_valid_game_root(&path, self.platform) {
            Ok(path)
        } else {
            Err(LauncherError::InvalidData {
                context: "validating selected game folder".into(),
                message: format!("{} is not a valid STFC game folder", path.display()),
            })
        }
    }

    pub fn from_launcher_settings_file(&self, settings_file: &Path) -> LauncherResult<Option<PathBuf>> {
        if !settings_file.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(settings_file)
            .map_err(|err| io_context(format!("reading {}", settings_file.display()), err))?;
        let parsed = parse_launcher_settings(&text)?;
        Ok(parsed.game_path.map(PathBuf::from).filter(|path| is_valid_game_root(path, self.platform)))
    }
}
```

- [x] **Step 3: Add platform detection**

Add to `models.rs`:

```rust
pub fn current_platform() -> Platform {
    #[cfg(target_os = "macos")]
    {
        Platform::MacOs
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
}
```

- [x] **Step 4: Add locate command shell**

Add command in `commands.rs`:

```rust
#[tauri::command]
pub fn validate_game_path(path: String) -> CommandResult<crate::models::GameStatus> {
    let locator = crate::game_locator::GameLocator::new(crate::models::current_platform());
    let validated = locator.validate_manual_root(std::path::PathBuf::from(path)).map_err(ErrorDto::from)?;
    Ok(crate::models::GameStatus {
        known: true,
        installed_version: crate::game_locator::installed_version(&validated),
        update_available: false,
        path: Some(validated.to_string_lossy().to_string()),
    })
}
```

Register it in `generate_handler!`.

- [x] **Step 5: Wire module**

Add to `lib.rs`:

```rust
mod game_locator;
```

- [x] **Step 6: Verify**

Run:

```bash
cd src-tauri && cargo test game_locator::tests
pnpm build
```

Expected: locator tests pass, frontend build passes.

- [x] **Step 7: Commit**

```bash
git add src-tauri/src/game_locator.rs src-tauri/src/models.rs src-tauri/src/app_state.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add game locator"
```

## Task 5: GitHub Release Selection For Mod Updates

**Files:**
- Create: `src-tauri/src/github_releases.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/models.rs`

- [x] **Step 1: Add release fixture**

Create `src-tauri/tests/fixtures/github_releases.json`:

```json
[
  {
    "tag_name": "v1.2.0",
    "prerelease": false,
    "assets": [
      { "name": "stfc-community-mod-windows-x64.tar.zst", "browser_download_url": "https://example.test/win.tar.zst" },
      { "name": "stfc-community-mod-windows-x64.tar.zst.sha256", "browser_download_url": "https://example.test/win.sha256" },
      { "name": "stfc-community-mod-macos-universal.tar.zst", "browser_download_url": "https://example.test/mac.tar.zst" },
      { "name": "stfc-community-mod-macos-universal.tar.zst.sha256", "browser_download_url": "https://example.test/mac.sha256" }
    ]
  },
  {
    "tag_name": "v1.3.0-beta.1",
    "prerelease": true,
    "assets": [
      { "name": "stfc-community-mod-windows-x64.tar.zst", "browser_download_url": "https://example.test/win-beta.tar.zst" },
      { "name": "stfc-community-mod-windows-x64.tar.zst.sha256", "browser_download_url": "https://example.test/win-beta.sha256" }
    ]
  }
]
```

- [x] **Step 2: Write release selection tests**

Create `src-tauri/src/github_releases.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_channel_skips_prereleases() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(include_str!("../tests/fixtures/github_releases.json")).expect("fixture");

        let selected = select_release_asset(&releases, crate::models::Platform::Windows, crate::models::ModChannel::Stable)
            .expect("stable asset");

        assert_eq!(selected.version, "v1.2.0");
        assert_eq!(selected.archive_url, "https://example.test/win.tar.zst");
    }

    #[test]
    fn prerelease_channel_can_select_prerelease() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(include_str!("../tests/fixtures/github_releases.json")).expect("fixture");

        let selected = select_release_asset(&releases, crate::models::Platform::Windows, crate::models::ModChannel::Prerelease)
            .expect("prerelease asset");

        assert_eq!(selected.version, "v1.3.0-beta.1");
        assert_eq!(selected.checksum_url, "https://example.test/win-beta.sha256");
    }
}
```

Run:

```bash
cd src-tauri && cargo test github_releases::tests
```

Expected: FAIL because release types do not exist.

- [x] **Step 3: Implement release models and selection**

Write `src-tauri/src/github_releases.rs`:

```rust
use crate::errors::{LauncherError, LauncherResult};
use crate::models::{ModChannel, Platform};
use serde::Deserialize;

const STFC_MOD_RELEASES_URL: &str = "https://api.github.com/repos/netniV/stfc-mod/releases";

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub prerelease: bool,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedModAsset {
    pub version: String,
    pub archive_name: String,
    pub archive_url: String,
    pub checksum_url: String,
}

fn expected_archive_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => "stfc-community-mod-windows-x64.tar.zst",
        Platform::MacOs => "stfc-community-mod-macos-universal.tar.zst",
    }
}

pub fn select_release_asset(
    releases: &[GitHubRelease],
    platform: Platform,
    channel: ModChannel,
) -> LauncherResult<SelectedModAsset> {
    let archive_name = expected_archive_name(platform);
    let checksum_name = format!("{archive_name}.sha256");

    for release in releases {
        if release.prerelease && channel == ModChannel::Stable {
            continue;
        }
        let archive = release.assets.iter().find(|asset| asset.name == archive_name);
        let checksum = release.assets.iter().find(|asset| asset.name == checksum_name);
        if let (Some(archive), Some(checksum)) = (archive, checksum) {
            return Ok(SelectedModAsset {
                version: release.tag_name.clone(),
                archive_name: archive.name.clone(),
                archive_url: archive.browser_download_url.clone(),
                checksum_url: checksum.browser_download_url.clone(),
            });
        }
    }

    Err(LauncherError::InvalidData {
        context: "selecting mod release asset".into(),
        message: format!("no {archive_name} asset with checksum found for {channel:?}"),
    })
}

pub async fn fetch_releases(client: &reqwest::Client) -> LauncherResult<Vec<GitHubRelease>> {
    let response = client
        .get(STFC_MOD_RELEASES_URL)
        .header(reqwest::header::USER_AGENT, "stfc-mod-launcher")
        .send()
        .await
        .map_err(|source| crate::errors::LauncherError::Network {
            context: "fetching STFC mod releases".into(),
            source,
        })?
        .error_for_status()
        .map_err(|source| crate::errors::LauncherError::Network {
            context: "checking STFC mod releases response".into(),
            source,
        })?;
    response
        .json()
        .await
        .map_err(|source| crate::errors::LauncherError::Network {
            context: "parsing STFC mod releases response".into(),
            source,
        })
}
```

- [x] **Step 4: Wire module**

Add to `lib.rs`:

```rust
mod github_releases;
```

- [x] **Step 5: Verify**

Run:

```bash
cd src-tauri && cargo test github_releases::tests
```

Expected: both release selection tests pass.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/github_releases.rs src-tauri/tests/fixtures/github_releases.json src-tauri/src/lib.rs src-tauri/src/models.rs
git commit -m "feat: select mod release assets"
```

## Task 6: Mod Download, Checksum, Zstd Extraction, And Install

**Files:**
- Create: `src-tauri/src/mod_manager.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`

- [x] **Step 1: Write checksum parser tests**

Create `src-tauri/src/mod_manager.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sha256_file_with_filename() {
        let parsed = parse_sha256("abcdef0123456789  stfc-community-mod-windows-x64.tar.zst\n")
            .expect("checksum");
        assert_eq!(parsed, "abcdef0123456789");
    }

    #[test]
    fn rejects_empty_sha256_file() {
        let error = parse_sha256("\n").expect_err("empty checksum rejected");
        assert!(error.to_string().contains("checksum file was empty"));
    }
}
```

Run:

```bash
cd src-tauri && cargo test mod_manager::tests
```

Expected: FAIL because parser does not exist.

- [x] **Step 2: Implement checksum parsing and hashing**

Write initial `src-tauri/src/mod_manager.rs`:

```rust
use crate::errors::{io_context, LauncherError, LauncherResult};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn parse_sha256(text: &str) -> LauncherResult<String> {
    let first = text
        .split_whitespace()
        .next()
        .ok_or_else(|| LauncherError::InvalidData {
            context: "parsing checksum".into(),
            message: "checksum file was empty".into(),
        })?;
    if first.len() != 64 && first.len() != 16 {
        return Err(LauncherError::InvalidData {
            context: "parsing checksum".into(),
            message: format!("checksum length {} is invalid", first.len()),
        });
    }
    Ok(first.to_ascii_lowercase())
}

pub fn sha256_file(path: &Path) -> LauncherResult<String> {
    let mut file = fs::File::open(path)
        .map_err(|err| io_context(format!("opening {}", path.display()), err))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| io_context(format!("reading {}", path.display()), err))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}
```

- [x] **Step 3: Write archive extraction test**

Add test in `mod_manager.rs`:

```rust
#[test]
fn extracts_single_file_from_tar_zst() {
    let root = tempfile::tempdir().expect("tempdir");
    let archive = root.path().join("mod.tar.zst");
    create_test_tar_zst(&archive, "version.dll", b"mod-bytes");
    let target = root.path().join("install");

    extract_mod_archive(&archive, &target).expect("extract");

    assert_eq!(
        std::fs::read(target.join("version.dll")).expect("read extracted"),
        b"mod-bytes"
    );
}

fn create_test_tar_zst(path: &Path, name: &str, bytes: &[u8]) {
    let file = std::fs::File::create(path).expect("archive");
    let encoder = zstd::Encoder::new(file, 0).expect("zstd encoder");
    let mut tar = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, name, bytes).expect("append");
    let encoder = tar.into_inner().expect("tar inner");
    encoder.finish().expect("zstd finish");
}
```

Run:

```bash
cd src-tauri && cargo test mod_manager::tests::extracts_single_file_from_tar_zst
```

Expected: FAIL because extraction does not exist.

- [x] **Step 4: Implement archive extraction**

Append to `mod_manager.rs`:

```rust
pub fn extract_mod_archive(archive: &Path, target_dir: &Path) -> LauncherResult<()> {
    if target_dir.exists() {
        fs::remove_dir_all(target_dir)
            .map_err(|err| io_context(format!("removing {}", target_dir.display()), err))?;
    }
    fs::create_dir_all(target_dir)
        .map_err(|err| io_context(format!("creating {}", target_dir.display()), err))?;

    let file = fs::File::open(archive)
        .map_err(|err| io_context(format!("opening {}", archive.display()), err))?;
    let decoder = zstd::Decoder::new(file).map_err(|err| io_context("creating zstd decoder", err))?;
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(target_dir)
        .map_err(|err| io_context(format!("extracting archive into {}", target_dir.display()), err))
}
```

- [x] **Step 5: Write install replacement test**

Add test:

```rust
#[test]
fn installs_extracted_library_atomically() {
    let root = tempfile::tempdir().expect("tempdir");
    let staging = root.path().join("staging");
    let mods = root.path().join("mods");
    std::fs::create_dir_all(&staging).expect("staging");
    std::fs::create_dir_all(&mods).expect("mods");
    std::fs::write(staging.join("version.dll"), b"new").expect("staged lib");
    std::fs::write(mods.join("version.dll"), b"old").expect("old lib");

    install_staged_library(&staging, &mods, crate::models::Platform::Windows).expect("install");

    assert_eq!(std::fs::read(mods.join("version.dll")).expect("read"), b"new");
}
```

Run:

```bash
cd src-tauri && cargo test mod_manager::tests::installs_extracted_library_atomically
```

Expected: FAIL because install function does not exist.

- [x] **Step 6: Implement atomic library replacement**

Append:

```rust
pub fn platform_library_name(platform: crate::models::Platform) -> &'static str {
    match platform {
        crate::models::Platform::Windows => "version.dll",
        crate::models::Platform::MacOs => "libstfc-community-mod.dylib",
    }
}

pub fn install_staged_library(
    staging_dir: &Path,
    mods_dir: &Path,
    platform: crate::models::Platform,
) -> LauncherResult<PathBuf> {
    let file_name = platform_library_name(platform);
    let source = staging_dir.join(file_name);
    if !source.is_file() {
        return Err(LauncherError::InvalidData {
            context: "installing mod library".into(),
            message: format!("archive did not contain {file_name}"),
        });
    }
    fs::create_dir_all(mods_dir)
        .map_err(|err| io_context(format!("creating {}", mods_dir.display()), err))?;
    let target = mods_dir.join(file_name);
    let replacement = mods_dir.join(format!("{file_name}.new"));
    if replacement.exists() {
        fs::remove_file(&replacement)
            .map_err(|err| io_context(format!("removing {}", replacement.display()), err))?;
    }
    fs::copy(&source, &replacement)
        .map_err(|err| io_context(format!("copying {} to {}", source.display(), replacement.display()), err))?;
    fs::rename(&replacement, &target)
        .map_err(|err| io_context(format!("renaming {} to {}", replacement.display(), target.display()), err))?;
    Ok(target)
}
```

- [x] **Step 7: Add command shell for mod channel**

Add commands:

```rust
#[tauri::command]
pub fn set_mod_channel(
    state: State<'_, AppState>,
    channel: crate::models::ModChannel,
) -> CommandResult<crate::models::LauncherStatus> {
    {
        let mut persisted = state.persisted.lock().map_err(|_| ErrorDto {
            kind: "state".into(),
            message: "launcher state lock is poisoned".into(),
        })?;
        persisted.mod_channel = channel;
        crate::storage::save_state(&state.paths, &persisted).map_err(ErrorDto::from)?;
    }

    let mut status = state.status.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher status lock is poisoned".into(),
    })?;
    status.mod_status.channel = channel;
    Ok(status.clone())
}
```

Register `set_mod_channel`.

- [x] **Step 8: Wire module and verify**

Add to `lib.rs`:

```rust
mod mod_manager;
```

Run:

```bash
cd src-tauri && cargo test mod_manager::tests
pnpm build
```

Expected: all mod manager tests pass.

- [x] **Step 9: Commit**

```bash
git add src-tauri/src/mod_manager.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: install managed mod archives"
```

## Task 7: Migration And Windows Fallback State

**Files:**
- Create: `src-tauri/src/migration.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write migration tests**

Create `src-tauri/src/migration.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_legacy_windows_files() {
        let root = tempfile::tempdir().expect("tempdir");
        let game = root.path().join("game");
        std::fs::create_dir_all(&game).expect("game");
        std::fs::write(game.join("version.dll"), b"dll").expect("dll");
        std::fs::write(game.join("community_patch_settings.toml"), b"config").expect("config");
        std::fs::write(game.join("community_patch.log"), b"log").expect("log");

        let plan = plan_windows_legacy_cleanup(&game).expect("plan");

        assert!(plan.stale_dll.is_some());
        assert_eq!(plan.files_to_move.len(), 2);
    }

    #[test]
    fn moves_legacy_files_into_managed_location() {
        let root = tempfile::tempdir().expect("tempdir");
        let game = root.path().join("game");
        let managed = crate::storage::ManagedPaths::from_root(root.path().join("managed"));
        managed.ensure_dirs().expect("managed dirs");
        std::fs::create_dir_all(&game).expect("game");
        std::fs::write(game.join("community_patch_settings.toml"), b"config").expect("config");

        let plan = plan_windows_legacy_cleanup(&game).expect("plan");
        apply_file_moves(&plan, &managed).expect("moves");

        assert!(managed.config_file.exists());
        assert!(!game.join("community_patch_settings.toml").exists());
    }
}
```

Run:

```bash
cd src-tauri && cargo test migration::tests
```

Expected: FAIL because migration code does not exist.

- [x] **Step 2: Implement migration planning**

Write `src-tauri/src/migration.rs`:

```rust
use crate::errors::{io_context, LauncherResult};
use crate::storage::ManagedPaths;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCleanupPlan {
    pub stale_dll: Option<PathBuf>,
    pub files_to_move: Vec<LegacyFileMove>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LegacyFileMove {
    pub source: PathBuf,
    pub destination_kind: LegacyDestination,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LegacyDestination {
    Config,
    Log,
}

pub fn plan_windows_legacy_cleanup(game_root: &Path) -> LauncherResult<LegacyCleanupPlan> {
    let mut files_to_move = Vec::new();
    let config = game_root.join("community_patch_settings.toml");
    if config.exists() {
        files_to_move.push(LegacyFileMove {
            source: config,
            destination_kind: LegacyDestination::Config,
        });
    }
    let log = game_root.join("community_patch.log");
    if log.exists() {
        files_to_move.push(LegacyFileMove {
            source: log,
            destination_kind: LegacyDestination::Log,
        });
    }
    let stale_dll = game_root.join("version.dll");
    Ok(LegacyCleanupPlan {
        stale_dll: stale_dll.exists().then_some(stale_dll),
        files_to_move,
    })
}

pub fn apply_file_moves(plan: &LegacyCleanupPlan, paths: &ManagedPaths) -> LauncherResult<Vec<PathBuf>> {
    paths.ensure_dirs()?;
    let mut moved = Vec::new();
    for file_move in &plan.files_to_move {
        let destination = match file_move.destination_kind {
            LegacyDestination::Config => paths.config_file.clone(),
            LegacyDestination::Log => {
                let name = file_move
                    .source
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                paths.logs_dir.join(name)
            }
        };
        if destination.exists() {
            fs::remove_file(&destination)
                .map_err(|err| io_context(format!("removing {}", destination.display()), err))?;
        }
        fs::rename(&file_move.source, &destination).map_err(|err| {
            io_context(
                format!("moving {} to {}", file_move.source.display(), destination.display()),
                err,
            )
        })?;
        moved.push(destination);
    }
    Ok(moved)
}

pub fn remove_stale_dll(plan: &LegacyCleanupPlan) -> LauncherResult<Option<PathBuf>> {
    if let Some(path) = &plan.stale_dll {
        fs::remove_file(path).map_err(|err| io_context(format!("removing {}", path.display()), err))?;
        Ok(Some(path.clone()))
    } else {
        Ok(None)
    }
}
```

- [x] **Step 3: Add migration commands**

Add commands:

```rust
#[tauri::command]
pub fn get_windows_legacy_cleanup_plan(game_root: String) -> CommandResult<crate::migration::LegacyCleanupPlan> {
    crate::migration::plan_windows_legacy_cleanup(std::path::Path::new(&game_root)).map_err(ErrorDto::from)
}

#[tauri::command]
pub fn apply_managed_migration(
    state: State<'_, AppState>,
    game_root: String,
    remove_stale_dll: bool,
) -> CommandResult<()> {
    let plan = crate::migration::plan_windows_legacy_cleanup(std::path::Path::new(&game_root)).map_err(ErrorDto::from)?;
    let moved = crate::migration::apply_file_moves(&plan, &state.paths).map_err(ErrorDto::from)?;
    state
        .diagnostics
        .info("migration", &format!("moved {} legacy files", moved.len()))
        .map_err(ErrorDto::from)?;
    if remove_stale_dll {
        let removed = crate::migration::remove_stale_dll(&plan).map_err(ErrorDto::from)?;
        if let Some(path) = removed {
            state
                .diagnostics
                .info("migration", &format!("removed stale DLL {}", path.display()))
                .map_err(ErrorDto::from)?;
        }
    }
    Ok(())
}
```

Register both commands.

- [x] **Step 4: Wire module and verify**

Add to `lib.rs`:

```rust
mod migration;
```

Run:

```bash
cd src-tauri && cargo test migration::tests
pnpm build
```

Expected: migration tests pass and frontend build passes.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/migration.rs src-tauri/src/models.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add legacy migration planning"
```

## Task 8: Port Config Model And TOML Generation To TypeScript

**Files:**
- Create: `src/lib/config/types.ts`
- Create: `src/lib/config/defaults.ts`
- Create: `src/lib/config/definitions.ts`
- Create: `src/lib/config/structure.ts`
- Create: `src/lib/config/toml.ts`
- Create: `src/lib/config/ui/*.ts`
- Create: `src/lib/config/letters.ts`
- Create: `src/lib/config/toml.test.ts`
- Modify: `package.json`

- [x] **Step 1: Add TOML dependency**

Run:

```bash
pnpm add toml@^4
```

Expected: `package.json` includes `toml`.

- [x] **Step 2: Copy config source model from modconfig**

Run:

```bash
mkdir -p src/lib/config/ui
cp ../modconfig/src/lib/definitions/config.ts src/lib/config/types.ts
cp ../modconfig/src/lib/definitions/structure.ts src/lib/config/structure.ts
cp ../modconfig/src/lib/definitions/ui/*.ts src/lib/config/ui/
cp ../modconfig/src/lib/toml-definition.ts src/lib/config/definitions.ts
cp ../modconfig/src/lib/toml-config.ts src/lib/config/defaults.ts
cp ../modconfig/src/lib/toml-handler.ts src/lib/config/toml.ts
cp ../modconfig/src/lib/utils/letters.ts src/lib/config/letters.ts
```

Then update imports:

- In `defaults.ts`, change imports from `./definitions/config` to `./types`.
- In `toml.ts`, change imports from `./definitions/config` to `./types`.
- In `toml.ts`, change imports from `./toml-definition` to `./definitions`.
- In `toml.ts`, change imports from `./definitions/structure` to `./structure`.
- In `toml.ts`, change imports from `./utils/letters` to `./letters`.
- In `definitions.ts`, change imports from `./definitions/ui/control` to `./ui/control`, and make the same `./ui/...` change for every UI definition import.
- In `definitions.ts`, change imports from `./definitions/config` to `./types`.
- In `definitions.ts`, change imports from `./definitions/structure` to `./structure`.
- In `definitions.ts`, change imports from `./toml-config` to `./defaults`.
- In `definitions.ts`, replace fallback generated descriptions for missing manual metadata with `"Generated definition for config key without manual metadata"`.
- In every file under `src/lib/config/ui/`, change imports from `../structure` to `../structure`.

- [x] **Step 3: Normalize structure exports**

Make sure `src/lib/config/structure.ts` exports these names from the copied modconfig structure:

```ts
import type { SyncTargetConfiguration, TomlConfig } from "./types";

export const groupDisplayNames: Record<string, string> = {
  Buffs: "Buffs",
  Config: "Config",
  Control: "Control",
  Display: "Graphics",
  UI: "User Interface",
  Shortcuts: "Hotkeys / Shortcuts",
  Sync: "Data Sync",
};

export type ConfigDefinition = {
  group?: string;
  key: string;
  label: string;
  type: "checkbox" | "slider" | "number" | "textbox" | "banner";
  description?: string;
  subgroup?: string;
  min?: number;
  max?: number;
  step?: number;
  isGenerated?: boolean;
  isHidden?: boolean;
};

export type EditableTarget = { key: string; config: SyncTargetConfiguration };
```

If copied code references `isPlaceholder`, rename that property to `isGenerated` in both `structure.ts` and `definitions.ts`.

- [x] **Step 4: Write config TOML tests**

Create `src/lib/config/toml.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { allDefinitions } from "./definitions";
import { defaultConfig } from "./defaults";
import { generateToml } from "./toml";

describe("config TOML generation", () => {
  it("ports field metadata from modconfig", () => {
    expect(allDefinitions.some((definition) => definition.key === "hotkeys_enabled")).toBe(true);
    expect(allDefinitions.some((definition) => definition.group === "graphics")).toBe(true);
  });

  it("generates expected sections from defaults", () => {
    const toml = generateToml(defaultConfig, false, false, false, false);

    expect(toml).toContain("[control]");
    expect(toml).toContain("hotkeys_enabled = true");
    expect(toml).toContain("[graphics]");
    expect(toml).toContain("[sync]");
  });
});
```

Run:

```bash
pnpm test src/lib/config/toml.test.ts
```

Expected: FAIL until imports and copied paths are corrected.

- [x] **Step 5: Fix imports and exported names**

Make sure `src/lib/config/toml.ts` exports:

```ts
export function generateToml(
  config: Partial<TomlConfig>,
  includeMainHeader = true,
  includeSectionHeader = true,
  includeSubgroupHeader = true,
  includeItemHeader = true,
): string
```

Make sure it imports:

```ts
import type { TomlConfig } from "./types";
import type { ConfigDefinition } from "./structure";
import { allDefinitions } from "./definitions";
import { groupDisplayNames } from "./structure";
```

- [x] **Step 6: Verify**

Run:

```bash
pnpm test src/lib/config/toml.test.ts
pnpm build
```

Expected: config TOML tests pass and frontend build passes.

- [x] **Step 7: Commit**

```bash
git add package.json pnpm-lock.yaml src/lib/config
git commit -m "feat: port config TOML model"
```

## Task 9: Rust ConfigService And Raw Config Actions

**Files:**
- Create: `src-tauri/src/config_service.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write config service tests**

Create `src-tauri/src/config_service.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_missing_config_as_empty_string() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = crate::storage::ManagedPaths::from_root(root.path().to_path_buf());
        let service = ConfigService::new(paths.config_file.clone());

        assert_eq!(service.read_config().expect("read"), "");
    }

    #[test]
    fn writes_config_and_preserves_text() {
        let root = tempfile::tempdir().expect("tempdir");
        let paths = crate::storage::ManagedPaths::from_root(root.path().to_path_buf());
        let service = ConfigService::new(paths.config_file.clone());

        service.write_config("[control]\nhotkeys_enabled = true\n").expect("write");

        assert_eq!(
            service.read_config().expect("read"),
            "[control]\nhotkeys_enabled = true\n"
        );
    }
}
```

Run:

```bash
cd src-tauri && cargo test config_service::tests
```

Expected: FAIL because service does not exist.

- [x] **Step 2: Implement ConfigService**

Write `src-tauri/src/config_service.rs`:

```rust
use crate::errors::{io_context, LauncherResult};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConfigService {
    config_file: PathBuf,
}

impl ConfigService {
    pub fn new(config_file: PathBuf) -> Self {
        Self { config_file }
    }

    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    pub fn read_config(&self) -> LauncherResult<String> {
        if !self.config_file.exists() {
            return Ok(String::new());
        }
        fs::read_to_string(&self.config_file)
            .map_err(|err| io_context(format!("reading {}", self.config_file.display()), err))
    }

    pub fn write_config(&self, text: &str) -> LauncherResult<()> {
        if let Some(parent) = self.config_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| io_context(format!("creating {}", parent.display()), err))?;
        }
        fs::write(&self.config_file, text)
            .map_err(|err| io_context(format!("writing {}", self.config_file.display()), err))
    }
}
```

- [x] **Step 3: Add config commands**

Add commands:

```rust
#[tauri::command]
pub fn read_raw_config(state: State<'_, AppState>) -> CommandResult<String> {
    crate::config_service::ConfigService::new(state.paths.config_file.clone())
        .read_config()
        .map_err(ErrorDto::from)
}

#[tauri::command]
pub fn save_raw_config(state: State<'_, AppState>, text: String) -> CommandResult<()> {
    crate::config_service::ConfigService::new(state.paths.config_file.clone())
        .write_config(&text)
        .map_err(ErrorDto::from)
}

#[tauri::command]
pub async fn open_raw_config(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    crate::config_service::ConfigService::new(state.paths.config_file.clone())
        .write_config(&crate::config_service::ConfigService::new(state.paths.config_file.clone()).read_config().map_err(ErrorDto::from)?)
        .map_err(ErrorDto::from)?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(state.paths.config_file.to_string_lossy().to_string(), None::<&str>)
        .map_err(|err| ErrorDto {
            kind: "openRawConfig".into(),
            message: err.to_string(),
        })?;
    Ok(())
}
```

Register commands.

- [x] **Step 4: Wire module and verify**

Add to `lib.rs`:

```rust
mod config_service;
```

Run:

```bash
cd src-tauri && cargo test config_service::tests
pnpm build
```

Expected: config service tests pass and frontend build passes.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/config_service.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add config file service"
```

## Task 10: Frontend Command Wrappers And State Types

**Files:**
- Create: `src/types/launcher.ts`
- Create: `src/lib/commands.ts`
- Create: `src/lib/commands.test.ts`

- [x] **Step 1: Write command wrapper tests**

Create `src/lib/commands.test.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getLauncherStatus, setModChannel } from "./commands";

describe("command wrappers", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("invokes launcher status command", async () => {
    vi.mocked(invoke).mockResolvedValue({ launcherUpdateAvailable: false });

    await getLauncherStatus();

    expect(invoke).toHaveBeenCalledWith("get_launcher_status");
  });

  it("invokes mod channel command with prerelease", async () => {
    vi.mocked(invoke).mockResolvedValue({});

    await setModChannel("prerelease");

    expect(invoke).toHaveBeenCalledWith("set_mod_channel", { channel: "prerelease" });
  });
});
```

Run:

```bash
pnpm test src/lib/commands.test.ts
```

Expected: FAIL because wrappers do not exist.

- [x] **Step 2: Implement launcher types**

Write `src/types/launcher.ts`:

```ts
export type ModChannel = "stable" | "prerelease";
export type LaunchMode = "managed" | "windowsProxyDll";

export type GameStatus = {
  known: boolean;
  path: string | null;
  installedVersion: number | null;
  updateAvailable: boolean;
};

export type ModStatus = {
  installed: boolean;
  installedVersion: string | null;
  latestVersion: string | null;
  channel: ModChannel;
  updateAvailable: boolean;
  launchMode: LaunchMode;
};

export type LauncherStatus = {
  game: GameStatus;
  modStatus: ModStatus;
  launcherUpdateAvailable: boolean;
};

export type ProgressEvent = {
  operation: string;
  phase: string;
  message: string;
  current: number | null;
  total: number | null;
};
```

- [x] **Step 3: Implement command wrappers**

Write `src/lib/commands.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { LauncherStatus, ModChannel, ProgressEvent } from "@/types/launcher";

export function getLauncherStatus(): Promise<LauncherStatus> {
  return invoke("get_launcher_status");
}

export function setModChannel(channel: ModChannel): Promise<LauncherStatus> {
  return invoke("set_mod_channel", { channel });
}

export function openLogs(): Promise<void> {
  return invoke("open_logs");
}

export function openRawConfig(): Promise<void> {
  return invoke("open_raw_config");
}

export function readRawConfig(): Promise<string> {
  return invoke("read_raw_config");
}

export function saveRawConfig(text: string): Promise<void> {
  return invoke("save_raw_config", { text });
}

export function validateGamePath(path: string) {
  return invoke("validate_game_path", { path });
}

export function onProgress(callback: (event: ProgressEvent) => void): Promise<() => void> {
  return listen<ProgressEvent>("launcher://progress", (event) => callback(event.payload));
}
```

- [x] **Step 4: Verify**

Run:

```bash
pnpm test src/lib/commands.test.ts
pnpm build
```

Expected: command wrapper tests pass and frontend build passes.

- [x] **Step 5: Commit**

```bash
git add src/types/launcher.ts src/lib/commands.ts src/lib/commands.test.ts
git commit -m "feat: add frontend command wrappers"
```

## Task 11: LCARS Main Window

**Files:**
- Modify: `src/App.vue`
- Create: `src/assets/lcars/Antonio-Regular.woff2`
- Create: `src/assets/lcars/Antonio-Bold.woff2`
- Create: `src/styles/lcars.css`
- Create: `src/components/lcars/LcarsShell.vue`
- Create: `src/components/lcars/LcarsButton.vue`
- Create: `src/components/lcars/DataCascade.vue`
- Create: `src/components/StatusStrip.vue`
- Create: `src/views/MainLauncher.vue`
- Create: `src/views/MainLauncher.test.ts`

- [x] **Step 1: Copy LCARS font assets**

Run:

```bash
mkdir -p src/assets/lcars
cp ~/Downloads/lcars_theme/assets/Antonio-Regular.woff2 src/assets/lcars/Antonio-Regular.woff2
cp ~/Downloads/lcars_theme/assets/Antonio-Bold.woff2 src/assets/lcars/Antonio-Bold.woff2
```

Expected: font assets exist under `src/assets/lcars`.

- [x] **Step 2: Create LCARS CSS tokens**

Write `src/styles/lcars.css`:

```css
@font-face {
  font-family: "Antonio";
  src: url("../assets/lcars/Antonio-Regular.woff2") format("woff2");
  font-weight: 400;
}

@font-face {
  font-family: "Antonio";
  src: url("../assets/lcars/Antonio-Bold.woff2") format("woff2");
  font-weight: 700;
}

:root {
  color-scheme: dark;
  --lcars-black: #000;
  --lcars-violet: #c9f;
  --lcars-orange: #f80;
  --lcars-tan: #ffaa90;
  --lcars-blue: #89f;
  --lcars-red: #c44;
  --lcars-gold: #fa0;
  --lcars-text: #c9f;
  --lcars-radius: 72px;
  font-family: "Antonio", "Arial Narrow", sans-serif;
}

body {
  margin: 0;
  background: var(--lcars-black);
  color: var(--lcars-text);
  overflow: hidden;
}

button {
  font: inherit;
}
```

- [x] **Step 3: Write main window test**

Create `src/views/MainLauncher.test.ts`:

```ts
import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import MainLauncher from "./MainLauncher.vue";

vi.mock("@/lib/commands", () => ({
  getLauncherStatus: vi.fn(async () => ({
    game: { known: true, path: "/game", installedVersion: 168, updateAvailable: true },
    modStatus: {
      installed: true,
      installedVersion: "v1.0.0",
      latestVersion: "v1.1.0",
      channel: "stable",
      updateAvailable: true,
      launchMode: "managed",
    },
    launcherUpdateAvailable: false,
  })),
  setModChannel: vi.fn(),
  openLogs: vi.fn(),
  openRawConfig: vi.fn(),
  onProgress: vi.fn(async () => vi.fn()),
}));

describe("MainLauncher", () => {
  it("renders permanent and conditional actions", async () => {
    const wrapper = mount(MainLauncher);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(wrapper.text()).toContain("Launch Game");
    expect(wrapper.text()).toContain("Open Raw Config");
    expect(wrapper.text()).toContain("Open Config Editor");
    expect(wrapper.text()).toContain("Open Logs");
    expect(wrapper.text()).toContain("Update Game");
    expect(wrapper.text()).toContain("Update Mod");
    expect(wrapper.text()).toContain("Stable");
  });
});
```

Run:

```bash
pnpm test src/views/MainLauncher.test.ts
```

Expected: FAIL because view does not exist.

- [x] **Step 4: Implement LCARS button**

Write `src/components/lcars/LcarsButton.vue`:

```vue
<script setup lang="ts">
defineProps<{
  tone?: "violet" | "orange" | "tan" | "blue" | "red" | "gold";
  disabled?: boolean;
}>();

const emit = defineEmits<{ click: [] }>();
</script>

<template>
  <button
    class="lcars-button"
    :class="[`tone-${tone ?? 'violet'}`]"
    :disabled="disabled"
    @click="emit('click')">
    <slot />
  </button>
</template>

<style scoped>
.lcars-button {
  min-width: 132px;
  height: 52px;
  border: 0;
  border-radius: 26px 0 0 26px;
  color: #000;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
  padding: 0 18px 8px 12px;
  font-weight: 700;
  text-transform: uppercase;
  cursor: pointer;
}
.lcars-button:disabled {
  opacity: 0.45;
  cursor: default;
}
.tone-violet { background: var(--lcars-violet); }
.tone-orange { background: var(--lcars-orange); }
.tone-tan { background: var(--lcars-tan); }
.tone-blue { background: var(--lcars-blue); }
.tone-red { background: var(--lcars-red); }
.tone-gold { background: var(--lcars-gold); }
</style>
```

- [x] **Step 5: Implement data cascade and shell**

Write `src/components/lcars/DataCascade.vue`:

```vue
<template>
  <div class="data-cascade" aria-hidden="true">
    <div v-for="column in 14" :key="column" class="data-column">
      <span v-for="row in 7" :key="row">{{ String(column * 137 + row * 29).padStart(5, "0") }}</span>
    </div>
  </div>
</template>

<style scoped>
.data-cascade {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  height: 96px;
  overflow: hidden;
  color: var(--lcars-orange);
  font-size: 16px;
  line-height: 1;
  opacity: 0.9;
}
.data-column {
  display: grid;
  gap: 2px;
  text-align: right;
}
</style>
```

Write `src/components/lcars/LcarsShell.vue`:

```vue
<template>
  <section class="lcars-shell">
    <aside class="left-frame">
      <div class="panel panel-top">LCARS</div>
      <div class="panel panel-fill">02-262000</div>
      <div class="panel panel-bottom">10-31</div>
    </aside>
    <main class="content-frame">
      <header class="top-frame">
        <div class="banner">LCARS ACCESS 333</div>
        <slot name="cascade" />
      </header>
      <div class="bar-panel">
        <div class="bar blue"></div>
        <div class="bar orange"></div>
        <div class="bar violet"></div>
        <div class="bar red"></div>
      </div>
      <section class="content">
        <slot />
      </section>
    </main>
  </section>
</template>

<style scoped>
.lcars-shell {
  display: grid;
  grid-template-columns: 164px 1fr;
  gap: 8px;
  width: 100vw;
  height: 100vh;
  padding: 10px;
  box-sizing: border-box;
  background: #000;
}
.left-frame {
  display: grid;
  grid-template-rows: 90px 1fr 70px;
  color: #000;
  font-weight: 700;
  text-align: right;
}
.panel {
  padding: 12px;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
}
.panel-top {
  background: var(--lcars-blue);
  border-radius: 0 0 0 var(--lcars-radius);
}
.panel-fill {
  background: var(--lcars-red);
  border-top: 8px solid #000;
  border-bottom: 8px solid #000;
}
.panel-bottom {
  background: var(--lcars-orange);
  border-radius: var(--lcars-radius) 0 0 0;
}
.content-frame {
  display: grid;
  grid-template-rows: 156px 24px 1fr;
  min-width: 0;
}
.top-frame {
  display: grid;
  align-items: end;
}
.banner {
  color: var(--lcars-orange);
  font-size: 42px;
  line-height: 1;
  text-align: right;
}
.bar-panel {
  display: grid;
  grid-template-columns: 4fr 1fr 2fr 1fr;
  gap: 8px;
}
.bar { height: 24px; }
.blue { background: var(--lcars-blue); }
.orange { background: var(--lcars-orange); }
.violet { background: var(--lcars-violet); }
.red { background: var(--lcars-red); }
.content {
  min-width: 0;
  min-height: 0;
  padding: 22px 0 0 22px;
}
</style>
```

- [x] **Step 6: Implement status strip and main view**

Write `src/components/StatusStrip.vue`:

```vue
<script setup lang="ts">
defineProps<{ message: string; warning?: string }>();
</script>

<template>
  <div class="status-strip">
    <span>{{ message }}</span>
    <strong v-if="warning">{{ warning }}</strong>
  </div>
</template>

<style scoped>
.status-strip {
  min-height: 28px;
  color: var(--lcars-tan);
  font-size: 18px;
  display: flex;
  gap: 18px;
  align-items: center;
}
strong {
  color: var(--lcars-gold);
}
</style>
```

Write `src/views/MainLauncher.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import DataCascade from "@/components/lcars/DataCascade.vue";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import LcarsShell from "@/components/lcars/LcarsShell.vue";
import StatusStrip from "@/components/StatusStrip.vue";
import { getLauncherStatus, openLogs, openRawConfig, setModChannel } from "@/lib/commands";
import type { LauncherStatus } from "@/types/launcher";

const status = ref<LauncherStatus | null>(null);
const message = ref("Initializing launcher");

const channelLabel = computed(() =>
  status.value?.modStatus.channel === "prerelease" ? "Prerelease" : "Stable",
);

const warning = computed(() => {
  if (!status.value) return "";
  if (status.value.game.updateAvailable || status.value.modStatus.updateAvailable) {
    return "Updates available";
  }
  return "";
});

async function refresh() {
  status.value = await getLauncherStatus();
  message.value = status.value.game.known ? "Game located" : "Game location required on launch";
}

async function toggleChannel() {
  const next = status.value?.modStatus.channel === "prerelease" ? "stable" : "prerelease";
  status.value = await setModChannel(next);
}

async function openConfigEditor() {
  const { WebviewWindow } = await import("@tauri-apps/api/window");
  const existing = await WebviewWindow.getByLabel("config-editor");
  if (existing) {
    await existing.setFocus();
    return;
  }
  new WebviewWindow("config-editor", {
    title: "STFC Mod Config",
    url: "/",
    width: 980,
    height: 720,
  });
}

function launchGame() {
  message.value = "Launch requested";
}

function updateGame() {
  message.value = "Game update requested";
}

function updateMod() {
  message.value = "Mod update requested";
}

onMounted(refresh);
</script>

<template>
  <LcarsShell>
    <template #cascade>
      <DataCascade />
    </template>

    <div class="main-grid">
      <div class="actions">
        <LcarsButton tone="orange" @click="launchGame">Launch Game</LcarsButton>
        <LcarsButton v-if="status?.game.updateAvailable" tone="gold" @click="updateGame">Update Game</LcarsButton>
        <LcarsButton v-if="status?.modStatus.updateAvailable" tone="blue" @click="updateMod">Update Mod</LcarsButton>
        <LcarsButton tone="violet" @click="openRawConfig">Open Raw Config</LcarsButton>
        <LcarsButton tone="tan" @click="openConfigEditor">Open Config Editor</LcarsButton>
        <LcarsButton tone="red" @click="openLogs">Open Logs</LcarsButton>
      </div>

      <button class="channel-toggle" @click="toggleChannel">{{ channelLabel }}</button>
      <StatusStrip :message="message" :warning="warning" />
    </div>
  </LcarsShell>
</template>

<style scoped>
.main-grid {
  position: relative;
  height: 100%;
  display: grid;
  grid-template-rows: 1fr auto;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  align-content: start;
  gap: 12px;
  max-width: 460px;
}
.channel-toggle {
  position: absolute;
  right: 18px;
  top: 4px;
  border: 0;
  border-radius: 0 26px 26px 0;
  background: var(--lcars-blue);
  color: #000;
  height: 52px;
  min-width: 130px;
  padding: 0 18px 8px;
  text-transform: uppercase;
  font-weight: 700;
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
}
</style>
```

- [x] **Step 7: Update app entry**

Modify `src/App.vue`:

```vue
<script setup lang="ts">
import { computed } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import MainLauncher from "@/views/MainLauncher.vue";
import ConfigEditor from "@/views/ConfigEditor.vue";
import "@/styles/lcars.css";

const windowLabel = getCurrentWindow().label;
const activeView = computed(() => (windowLabel === "config-editor" ? ConfigEditor : MainLauncher));
</script>

<template>
  <component :is="activeView" />
</template>
```

Create initial `src/views/ConfigEditor.vue`:

```vue
<template>
  <main class="config-loading">Config editor loading</main>
</template>

<style scoped>
.config-loading {
  padding: 24px;
  color: var(--lcars-tan);
}
</style>
```

- [x] **Step 8: Verify**

Run:

```bash
pnpm test src/views/MainLauncher.test.ts
pnpm build
```

Expected: main launcher test passes and frontend build passes.

- [x] **Step 9: Commit**

```bash
git add src/App.vue src/assets/lcars src/styles/lcars.css src/components/lcars src/components/StatusStrip.vue src/views/MainLauncher.vue src/views/MainLauncher.test.ts src/views/ConfigEditor.vue
git commit -m "feat: add LCARS main launcher UI"
```

## Task 12: Config Editor Window

**Files:**
- Modify: `src/views/ConfigEditor.vue`
- Create: `src/views/ConfigEditor.test.ts`
- Create: `src/components/config/ConfigSection.vue`
- Create: `src/components/config/ConfigField.vue`
- Create: `src/components/config/SyncTargetsEditor.vue`

- [x] **Step 1: Write dirty close and preview test**

Create `src/views/ConfigEditor.test.ts`:

```ts
import { mount } from "@vue/test-utils";
import { describe, expect, it, vi } from "vitest";
import ConfigEditor from "./ConfigEditor.vue";

vi.mock("@/lib/commands", () => ({
  readRawConfig: vi.fn(async () => ""),
  saveRawConfig: vi.fn(async () => undefined),
}));

describe("ConfigEditor", () => {
  it("starts with TOML preview collapsed and enables save when dirty", async () => {
    const wrapper = mount(ConfigEditor);
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(wrapper.text()).toContain("Control Panel");
    expect(wrapper.text()).toContain("Show TOML Preview");
    expect(wrapper.find("textarea").exists()).toBe(false);

    await wrapper.find("input[type='checkbox']").setValue(false);

    expect(wrapper.text()).toContain("Unsaved changes");
    expect(wrapper.find("button.save").attributes("disabled")).toBeUndefined();
  });
});
```

Run:

```bash
pnpm test src/views/ConfigEditor.test.ts
```

Expected: FAIL because editor does not exist.

- [x] **Step 2: Implement generic config field**

Write `src/components/config/ConfigField.vue`:

```vue
<script setup lang="ts">
import type { ConfigDefinition } from "@/lib/config/definitions";

const props = defineProps<{
  definition: ConfigDefinition;
  modelValue: boolean | number | string;
}>();

const emit = defineEmits<{ "update:modelValue": [boolean | number | string] }>();

function update(event: Event) {
  const target = event.target as HTMLInputElement;
  if (props.definition.type === "checkbox") {
    emit("update:modelValue", target.checked);
  } else if (props.definition.type === "number" || props.definition.type === "slider") {
    emit("update:modelValue", Number(target.value));
  } else {
    emit("update:modelValue", target.value);
  }
}
</script>

<template>
  <label class="config-field">
    <span>{{ definition.key }}</span>
    <input
      v-if="definition.type === 'checkbox'"
      type="checkbox"
      :checked="Boolean(modelValue)"
      @change="update" />
    <input
      v-else-if="definition.type === 'slider'"
      type="range"
      :min="definition.min ?? 0"
      :max="definition.max ?? 100"
      :step="definition.step ?? 1"
      :value="Number(modelValue)"
      @input="update" />
    <input
      v-else-if="definition.type === 'number'"
      type="number"
      :value="Number(modelValue)"
      @input="update" />
    <input
      v-else
      type="text"
      :value="String(modelValue ?? '')"
      @input="update" />
  </label>
</template>

<style scoped>
.config-field {
  display: grid;
  grid-template-columns: 1fr minmax(160px, 260px);
  gap: 12px;
  align-items: center;
  color: var(--lcars-tan);
}
input {
  background: #111;
  border: 1px solid var(--lcars-violet);
  color: var(--lcars-tan);
  padding: 6px 8px;
}
input[type="checkbox"] {
  width: 22px;
  height: 22px;
}
</style>
```

- [x] **Step 3: Implement config section**

Write `src/components/config/ConfigSection.vue`:

```vue
<script setup lang="ts">
import ConfigField from "./ConfigField.vue";
import type { ConfigDefinition } from "@/lib/config/definitions";

defineProps<{
  title: string;
  definitions: ConfigDefinition[];
  section: Record<string, boolean | number | string>;
}>();

const emit = defineEmits<{ updateField: [key: string, value: boolean | number | string] }>();
</script>

<template>
  <section class="config-section">
    <h2>{{ title }}</h2>
    <ConfigField
      v-for="definition in definitions"
      :key="definition.key"
      :definition="definition"
      :model-value="section[definition.key] ?? ''"
      @update:model-value="emit('updateField', definition.key, $event)" />
  </section>
</template>

<style scoped>
.config-section {
  display: grid;
  gap: 14px;
}
h2 {
  color: var(--lcars-orange);
  margin: 0;
  text-transform: uppercase;
}
</style>
```

- [x] **Step 4: Implement sync target list component**

Write `src/components/config/SyncTargetsEditor.vue`:

```vue
<script setup lang="ts">
defineProps<{ targets: Record<string, Record<string, unknown>> }>();
</script>

<template>
  <section class="sync-targets">
    <h3>Sync Targets</h3>
    <p v-if="Object.keys(targets).length === 0">No sync targets configured.</p>
    <ul v-else>
      <li v-for="name in Object.keys(targets)" :key="name">{{ name }}</li>
    </ul>
  </section>
</template>

<style scoped>
.sync-targets {
  color: var(--lcars-tan);
}
h3 {
  color: var(--lcars-blue);
}
</style>
```

- [x] **Step 5: Implement config editor**

Write `src/views/ConfigEditor.vue`:

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import ConfigSection from "@/components/config/ConfigSection.vue";
import DataCascade from "@/components/lcars/DataCascade.vue";
import LcarsButton from "@/components/lcars/LcarsButton.vue";
import LcarsShell from "@/components/lcars/LcarsShell.vue";
import { readRawConfig, saveRawConfig } from "@/lib/commands";
import { defaultConfig } from "@/lib/config/defaults";
import { generateToml } from "@/lib/config/toml";
import type { TomlConfig } from "@/lib/config/types";

const config = ref<TomlConfig>(structuredClone(defaultConfig));
const savedToml = ref("");
const showToml = ref(false);
const activeSection = ref<keyof TomlConfig>("control");

const sections = [
  { key: "control", label: "Control Panel" },
  { key: "graphics", label: "Graphics Settings" },
  { key: "shortcuts", label: "Keyboard Shortcuts" },
  { key: "sync", label: "Sync Options" },
  { key: "ui", label: "Interface" },
  { key: "buffs", label: "Buffs" },
  { key: "config", label: "Configuration" },
  { key: "patches", label: "Patches" },
] as const;

const generatedToml = computed(() => generateToml(config.value, true, true, true, true));
const dirty = computed(() => generatedToml.value !== savedToml.value);
const currentSection = computed(() => config.value[activeSection.value] as Record<string, boolean | number | string>);
const currentDefinitions = computed(() =>
  Object.keys(currentSection.value).map((key) => ({
    group: activeSection.value,
    key,
    type: typeof currentSection.value[key] === "boolean" ? "checkbox" : typeof currentSection.value[key] === "number" ? "number" : "textbox",
    description: key,
  })),
);

function updateField(key: string, value: boolean | number | string) {
  config.value = {
    ...config.value,
    [activeSection.value]: {
      ...(config.value[activeSection.value] as Record<string, unknown>),
      [key]: value,
    },
  };
}

async function save() {
  await saveRawConfig(generatedToml.value);
  savedToml.value = generatedToml.value;
}

onMounted(async () => {
  const raw = await readRawConfig();
  savedToml.value = raw || generateToml(config.value, true, true, true, true);
});
</script>

<template>
  <LcarsShell>
    <template #cascade>
      <DataCascade />
    </template>

    <div class="editor">
      <aside class="tabs">
        <button
          v-for="section in sections"
          :key="section.key"
          :class="{ active: activeSection === section.key }"
          @click="activeSection = section.key">
          {{ section.label }}
        </button>
      </aside>

      <main class="panel">
        <div class="toolbar">
          <span v-if="dirty">Unsaved changes</span>
          <LcarsButton class="save" tone="orange" :disabled="!dirty" @click="save">Save</LcarsButton>
          <LcarsButton tone="blue" @click="showToml = !showToml">
            {{ showToml ? "Hide TOML Preview" : "Show TOML Preview" }}
          </LcarsButton>
        </div>

        <ConfigSection
          :title="sections.find((section) => section.key === activeSection)?.label ?? 'Config'"
          :definitions="currentDefinitions"
          :section="currentSection"
          @update-field="updateField" />

        <textarea v-if="showToml" readonly :value="generatedToml" />
      </main>
    </div>
  </LcarsShell>
</template>

<style scoped>
.editor {
  display: grid;
  grid-template-columns: 190px 1fr;
  gap: 18px;
  height: 100%;
  min-height: 0;
}
.tabs {
  display: grid;
  align-content: start;
  gap: 8px;
}
.tabs button {
  background: var(--lcars-violet);
  color: #000;
  border: 0;
  min-height: 38px;
  padding: 6px 12px;
  text-align: right;
  font-weight: 700;
}
.tabs button.active {
  background: var(--lcars-orange);
}
.panel {
  min-height: 0;
  overflow: auto;
  display: grid;
  gap: 18px;
  align-content: start;
}
.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  color: var(--lcars-gold);
}
textarea {
  min-height: 220px;
  background: #080808;
  color: var(--lcars-tan);
  border: 1px solid var(--lcars-violet);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}
</style>
```

- [x] **Step 6: Verify**

Run:

```bash
pnpm test src/views/ConfigEditor.test.ts
pnpm build
```

Expected: config editor test passes and frontend build passes.

- [x] **Step 7: Commit**

```bash
git add src/views/ConfigEditor.vue src/views/ConfigEditor.test.ts src/components/config
git commit -m "feat: add config editor window"
```

## Task 13: Xsolla Parser, Action Runner, And Game Update Finalization

**Files:**
- Create: `src-tauri/src/xsolla.rs`
- Create: `src-tauri/src/game_updater.rs`
- Create: `src-tauri/src/rsync_patch.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Add Xsolla XML fixture**

Create `src-tauri/tests/fixtures/xsolla_plan.xml`:

```xml
<update>
  <action type="torrent_download" alt_data_link="https://example.test/update.7z" data_size="1234" alt_to="$temp_path/update.7z" />
  <action type="extract" file="$temp_path/update.7z" to="$temp_path/extracted" />
  <action type="patch" binaries="$game_path" patch="$temp_path/extracted" />
  <action type="wait_actions" />
  <action type="version" version="169" />
</update>
```

- [x] **Step 2: Write Xsolla parser tests**

Create `src-tauri/src/xsolla.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_update_actions() {
        let plan = parse_update_plan(include_str!("../tests/fixtures/xsolla_plan.xml")).expect("parse");

        assert_eq!(plan.target_version, Some(169));
        assert_eq!(plan.actions.len(), 5);
        assert!(matches!(plan.actions[0], XsollaAction::Download { .. }));
        assert!(matches!(plan.actions[4], XsollaAction::Version { version: 169 }));
    }

    #[test]
    fn rejects_patch_path_escape() {
        let error = normalize_relative_patch_path("../escape").expect_err("path escape rejected");
        assert!(error.to_string().contains("invalid patch path"));
    }
}
```

Run:

```bash
cd src-tauri && cargo test xsolla::tests
```

Expected: FAIL because parser does not exist.

- [x] **Step 3: Implement Xsolla parser and path guard**

Write `src-tauri/src/xsolla.rs`:

```rust
use crate::errors::{LauncherError, LauncherResult};
use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsollaPlan {
    pub target_version: Option<u32>,
    pub actions: Vec<XsollaAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XsollaAction {
    Download { url: String, size: u64, to: String },
    Extract { file: String, to: String },
    Patch { binaries: String, patch: String },
    Wait,
    Version { version: u32 },
}

pub fn parse_update_plan(xml: &str) -> LauncherResult<XsollaPlan> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    let mut actions = Vec::new();
    let mut target_version = None;
    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Empty(event)) | Ok(Event::Start(event)) if event.name().as_ref() == b"action" => {
                let attrs = event
                    .attributes()
                    .map(|attr| attr.map_err(|err| err.to_string()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|message| LauncherError::InvalidData {
                        context: "parsing Xsolla action attributes".into(),
                        message,
                    })?;
                let get = |name: &[u8]| -> Option<String> {
                    attrs
                        .iter()
                        .find(|attr| attr.key.as_ref() == name)
                        .map(|attr| String::from_utf8_lossy(attr.value.as_ref()).to_string())
                };
                match get(b"type").as_deref() {
                    Some("torrent_download") => actions.push(XsollaAction::Download {
                        url: get(b"alt_data_link").unwrap_or_default(),
                        size: get(b"data_size").and_then(|value| value.parse().ok()).unwrap_or(0),
                        to: get(b"alt_to").unwrap_or_default(),
                    }),
                    Some("extract") => actions.push(XsollaAction::Extract {
                        file: get(b"file").unwrap_or_default(),
                        to: get(b"to").unwrap_or_default(),
                    }),
                    Some("patch") => actions.push(XsollaAction::Patch {
                        binaries: get(b"binaries").unwrap_or_default(),
                        patch: get(b"patch").unwrap_or_default(),
                    }),
                    Some("wait_actions") => actions.push(XsollaAction::Wait),
                    Some("version") => {
                        let version = get(b"version").and_then(|value| value.parse().ok()).ok_or_else(|| {
                            LauncherError::InvalidData {
                                context: "parsing Xsolla version action".into(),
                                message: "version action missing numeric version".into(),
                            }
                        })?;
                        target_version = Some(version);
                        actions.push(XsollaAction::Version { version });
                    }
                    Some("extracted_size") => {}
                    Some(other) => {
                        return Err(LauncherError::InvalidData {
                            context: "parsing Xsolla action".into(),
                            message: format!("unknown action type {other}"),
                        });
                    }
                    None => {}
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => {
                return Err(LauncherError::InvalidData {
                    context: "parsing Xsolla XML".into(),
                    message: err.to_string(),
                });
            }
        }
        buffer.clear();
    }

    Ok(XsollaPlan { target_version, actions })
}

pub fn normalize_relative_patch_path(path: &str) -> LauncherResult<String> {
    let mut components = Vec::new();
    for component in path.trim().split(['/', '\\']) {
        if component.is_empty() || component == "." || component == ".." {
            return Err(LauncherError::InvalidData {
                context: "normalizing patch path".into(),
                message: format!("invalid patch path {path}"),
            });
        }
        components.push(component);
    }
    if components.is_empty() {
        return Err(LauncherError::InvalidData {
            context: "normalizing patch path".into(),
            message: format!("invalid patch path {path}"),
        });
    }
    Ok(components.join("/"))
}
```

- [x] **Step 4: Write finalization ordering test**

Create `src-tauri/src/game_updater.rs`:

```rust
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

        assert_eq!(std::fs::read(game.join("prime.exe")).expect("patched"), b"patched");
        assert_eq!(std::fs::read_to_string(game.join(".version")).expect("version"), "&game=169");
    }
}
```

Run:

```bash
cd src-tauri && cargo test game_updater::tests
```

Expected: FAIL because finalization does not exist.

- [x] **Step 5: Implement finalization helpers**

Write `src-tauri/src/game_updater.rs`:

```rust
use crate::errors::{io_context, LauncherResult};
use std::fs;
use std::path::{Path, PathBuf};

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
    fs::create_dir_all(target).map_err(|err| io_context(format!("creating {}", target.display()), err))?;
    for entry in fs::read_dir(source).map_err(|err| io_context(format!("reading {}", source.display()), err))? {
        let entry = entry.map_err(|err| io_context(format!("reading entry in {}", source.display()), err))?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory_contents(&source_path, &target_path)?;
        } else {
            if target_path.exists() {
                fs::remove_file(&target_path)
                    .map_err(|err| io_context(format!("removing {}", target_path.display()), err))?;
            }
            fs::copy(&source_path, &target_path).map_err(|err| {
                io_context(format!("copying {} to {}", source_path.display(), target_path.display()), err)
            })?;
        }
    }
    Ok(())
}

fn apply_deferred_deletes(staging_root: &Path, game_root: &Path, pending_deletes: &[PathBuf]) -> LauncherResult<()> {
    for relative in pending_deletes {
        if staging_root.join(relative).exists() {
            continue;
        }
        let target = game_root.join(relative);
        if target.exists() {
            fs::remove_file(&target).map_err(|err| io_context(format!("deleting {}", target.display()), err))?;
        }
    }
    Ok(())
}

fn write_installed_game_version(game_root: &Path, version: u32) -> LauncherResult<()> {
    fs::write(game_root.join(".version"), format!("&game={version}"))
        .map_err(|err| io_context(format!("writing {}", game_root.join(".version").display()), err))
}
```

- [x] **Step 6: Add librsync wrapper skeleton**

Write `src-tauri/src/rsync_patch.rs`:

```rust
use crate::errors::{io_context, LauncherResult};
use std::fs;
use std::path::Path;

pub fn apply_rsync_patch(source: &Path, patch: &Path, output: &Path) -> LauncherResult<()> {
    let source_file = fs::File::open(source)
        .map_err(|err| io_context(format!("opening {}", source.display()), err))?;
    let patch_file = fs::File::open(patch)
        .map_err(|err| io_context(format!("opening {}", patch.display()), err))?;
    let mut output_file = fs::File::create(output)
        .map_err(|err| io_context(format!("creating {}", output.display()), err))?;

    librsync::patch(source_file, patch_file, &mut output_file).map_err(|err| {
        crate::errors::LauncherError::Operation {
            context: "applying rsync patch".into(),
            message: err.to_string(),
        }
    })
}
```

- [x] **Step 7: Add Xsolla action runner types**

Append to `src-tauri/src/game_updater.rs`:

```rust
use crate::errors::LauncherError;
use crate::xsolla::{normalize_relative_patch_path, XsollaAction, XsollaPlan};
use serde::Deserialize;

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

pub fn extract_7z_archive(archive: &Path, destination: &Path) -> LauncherResult<()> {
    fs::create_dir_all(destination)
        .map_err(|err| io_context(format!("creating {}", destination.display()), err))?;
    sevenz_rust2::decompress_file(archive, destination).map_err(|err| LauncherError::Operation {
        context: "extracting Xsolla 7z archive".into(),
        message: err.to_string(),
    })
}
```

- [x] **Step 8: Add Xsolla action runner**

Append to `src-tauri/src/game_updater.rs`:

```rust
pub async fn run_update_plan(
    client: &reqwest::Client,
    plan: &XsollaPlan,
    context: &GameUpdateContext,
) -> LauncherResult<()> {
    if context.xsolla_temp_root.exists() {
        fs::remove_dir_all(&context.xsolla_temp_root).map_err(|err| {
            io_context(format!("removing {}", context.xsolla_temp_root.display()), err)
        })?;
    }
    if context.staging_root.exists() {
        fs::remove_dir_all(&context.staging_root).map_err(|err| {
            io_context(format!("removing {}", context.staging_root.display()), err)
        })?;
    }
    fs::create_dir_all(&context.xsolla_temp_root).map_err(|err| {
        io_context(format!("creating {}", context.xsolla_temp_root.display()), err)
    })?;
    fs::create_dir_all(&context.staging_root).map_err(|err| {
        io_context(format!("creating {}", context.staging_root.display()), err)
    })?;

    let mut pending_deletes = Vec::new();
    let mut pending_version = None;

    for action in &plan.actions {
        match action {
            XsollaAction::Download { url, to, .. } => {
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
                let archive = PathBuf::from(substitute_paths(file, context));
                let destination = PathBuf::from(substitute_paths(to, context));
                extract_7z_archive(&archive, &destination)?;
            }
            XsollaAction::Patch { patch, .. } => {
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
                                    format!("renaming {} to {}", output.display(), staged_target.display()),
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
                                    format!("copying {} to {}", patch_path.display(), staged_target.display()),
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
            XsollaAction::Wait => {}
            XsollaAction::Version { version } => pending_version = Some(*version),
        }
    }

    finalize_update(
        &context.staging_root,
        &context.game_root,
        &pending_deletes,
        pending_version,
    )
}
```

- [x] **Step 9: Wire modules and verify**

Add to `lib.rs`:

```rust
mod xsolla;
mod game_updater;
mod rsync_patch;
```

Run:

```bash
cd src-tauri && cargo test xsolla::tests game_updater::tests
```

Expected: parser and finalization tests pass.

- [x] **Step 10: Commit**

```bash
git add src-tauri/src/xsolla.rs src-tauri/src/game_updater.rs src-tauri/src/rsync_patch.rs src-tauri/tests/fixtures/xsolla_plan.xml src-tauri/src/lib.rs
git commit -m "feat: add Xsolla update core"
```

## Task 14: Launch Service

**Files:**
- Create: `src-tauri/src/launch.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Write launch command construction tests**

Create `src-tauri/src/launch.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_launch_plan_uses_dylib_injection() {
        let plan = build_launch_plan(
            crate::models::Platform::MacOs,
            std::path::Path::new("/game"),
            std::path::Path::new("/mods/libstfc-community-mod.dylib"),
            crate::models::LaunchMode::Managed,
        )
        .expect("launch plan");

        assert_eq!(plan.executable, "/game/Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command");
        assert_eq!(
            plan.environment.get("DYLD_INSERT_LIBRARIES").map(String::as_str),
            Some("/mods/libstfc-community-mod.dylib")
        );
    }

    #[test]
    fn windows_fallback_uses_prime_exe() {
        let plan = build_launch_plan(
            crate::models::Platform::Windows,
            std::path::Path::new("C:/Games/STFC/game"),
            std::path::Path::new("C:/Games/STFC/game/version.dll"),
            crate::models::LaunchMode::WindowsProxyDll,
        )
        .expect("launch plan");

        assert!(plan.executable.ends_with("prime.exe"));
        assert!(plan.environment.is_empty());
    }
}
```

Run:

```bash
cd src-tauri && cargo test launch::tests
```

Expected: FAIL because launch code does not exist.

- [x] **Step 2: Implement launch plan construction**

Write `src-tauri/src/launch.rs`:

```rust
use crate::errors::{LauncherError, LauncherResult};
use crate::models::{LaunchMode, Platform};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub executable: String,
    pub args: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub working_dir: Option<PathBuf>,
}

pub fn build_launch_plan(
    platform: Platform,
    game_root: &Path,
    mod_library: &Path,
    launch_mode: LaunchMode,
) -> LauncherResult<LaunchPlan> {
    match (platform, launch_mode) {
        (Platform::MacOs, LaunchMode::Managed) => {
            let executable = game_root.join("Star Trek Fleet Command.app/Contents/MacOS/Star Trek Fleet Command");
            let mut environment = BTreeMap::new();
            environment.insert(
                "DYLD_INSERT_LIBRARIES".into(),
                mod_library.to_string_lossy().to_string(),
            );
            environment.insert(
                "DYLD_LIBRARY_PATH".into(),
                mod_library.parent().unwrap_or_else(|| Path::new("")).to_string_lossy().to_string(),
            );
            Ok(LaunchPlan {
                executable: executable.to_string_lossy().to_string(),
                args: Vec::new(),
                environment,
                working_dir: executable.parent().map(Path::to_path_buf),
            })
        }
        (Platform::Windows, LaunchMode::Managed) => {
            let executable = game_root.join("prime.exe");
            let mut environment = BTreeMap::new();
            environment.insert(
                "PATH".into(),
                mod_library.parent().unwrap_or_else(|| Path::new("")).to_string_lossy().to_string(),
            );
            Ok(LaunchPlan {
                executable: executable.to_string_lossy().to_string(),
                args: Vec::new(),
                environment,
                working_dir: Some(game_root.to_path_buf()),
            })
        }
        (Platform::Windows, LaunchMode::WindowsProxyDll) => Ok(LaunchPlan {
            executable: game_root.join("prime.exe").to_string_lossy().to_string(),
            args: Vec::new(),
            environment: BTreeMap::new(),
            working_dir: Some(game_root.to_path_buf()),
        }),
        (Platform::MacOs, LaunchMode::WindowsProxyDll) => Err(LauncherError::InvalidData {
            context: "building launch plan".into(),
            message: "Windows proxy DLL mode is not valid on macOS".into(),
        }),
    }
}

pub fn run_launch_plan(plan: &LaunchPlan) -> LauncherResult<()> {
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
```

- [x] **Step 3: Add launch command shell**

Add command:

```rust
#[tauri::command]
pub fn launch_game(state: State<'_, AppState>) -> CommandResult<()> {
    let persisted = state.persisted.lock().map_err(|_| ErrorDto {
        kind: "state".into(),
        message: "launcher state lock is poisoned".into(),
    })?;
    let game_path = persisted.game_path.clone().ok_or_else(|| ErrorDto {
        kind: "gamePath".into(),
        message: "game path is not known".into(),
    })?;
    let platform = crate::models::current_platform();
    let mod_library = state.paths.mods_dir.join(crate::mod_manager::platform_library_name(platform));
    let plan = crate::launch::build_launch_plan(platform, &game_path, &mod_library, persisted.launch_mode)
        .map_err(ErrorDto::from)?;
    state
        .diagnostics
        .info("launch", &format!("launching with mode {:?}", persisted.launch_mode))
        .map_err(ErrorDto::from)?;
    crate::launch::run_launch_plan(&plan).map_err(ErrorDto::from)
}
```

Register `launch_game`.

- [x] **Step 4: Wire module and verify**

Add to `lib.rs`:

```rust
mod launch;
```

Run:

```bash
cd src-tauri && cargo test launch::tests
pnpm build
```

Expected: launch tests pass and frontend build passes.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/launch.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: add platform launch planning"
```

## Task 15: Self-Update Service

**Files:**
- Create: `src-tauri/src/self_update.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`

- [x] **Step 1: Generate updater signing keys**

Run:

```bash
mkdir -p ~/.tauri
pnpm tauri signer generate -w ~/.tauri/stfc-mod-launcher.key
```

Expected: the command prints a public key and writes the private key to `~/.tauri/stfc-mod-launcher.key`. Store the private key outside the repo. Copy the printed public key for the next step.

- [x] **Step 2: Add updater config**

Modify `src-tauri/tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "pubkey": "paste the public key printed by pnpm tauri signer generate",
      "endpoints": [
        "https://github.com/netniV/stfc-mod-new-launcher/releases/latest/download/latest.json"
      ]
    }
  }
}
```

Use the actual repository owner/name for this launcher if it differs from `netniV/stfc-mod-new-launcher`. The public key string must come from the Tauri updater signing key generated for this launcher.

- [x] **Step 3: Write self-update service**

Write `src-tauri/src/self_update.rs`:

```rust
use crate::errors::{LauncherError, LauncherResult};
use serde::Serialize;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherUpdateInfo {
    pub version: String,
    pub body: Option<String>,
}

pub async fn check_for_launcher_update(app: tauri::AppHandle) -> LauncherResult<Option<LauncherUpdateInfo>> {
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
```

- [x] **Step 4: Add commands**

Add commands:

```rust
#[tauri::command]
pub async fn check_launcher_update(app: tauri::AppHandle) -> CommandResult<Option<crate::self_update::LauncherUpdateInfo>> {
    crate::self_update::check_for_launcher_update(app).await.map_err(ErrorDto::from)
}
```

Register command.

- [x] **Step 5: Wire module and verify**

Add to `lib.rs`:

```rust
mod self_update;
```

Run:

```bash
cd src-tauri && cargo test
pnpm build
```

Expected: tests and build pass.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/self_update.rs src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json
git commit -m "feat: add launcher self-update check"
```

## Task 16: Command Bridge For Update Actions And Progress Events

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/views/MainLauncher.vue`
- Modify: `src/lib/commands.ts`
- Modify: `src/types/launcher.ts`

- [x] **Step 1: Extend frontend wrappers**

Add to `src/lib/commands.ts`:

```ts
export function launchGame(): Promise<void> {
  return invoke("launch_game");
}

export function checkLauncherUpdate() {
  return invoke("check_launcher_update");
}

export function getWindowsLegacyCleanupPlan(gameRoot: string) {
  return invoke("get_windows_legacy_cleanup_plan", { gameRoot });
}

export function applyManagedMigration(gameRoot: string, removeStaleDll: boolean): Promise<void> {
  return invoke("apply_managed_migration", { gameRoot, removeStaleDll });
}
```

- [x] **Step 2: Connect main buttons to commands**

Modify `MainLauncher.vue` imports:

```ts
import {
  getLauncherStatus,
  launchGame as launchGameCommand,
  openLogs,
  openRawConfig,
  setModChannel,
} from "@/lib/commands";
```

Replace `launchGame()`:

```ts
async function launchGame() {
  if (warning.value) {
    message.value = `${warning.value}. Launching anyway.`;
  }
  await launchGameCommand();
  message.value = "Game launch started";
}
```

- [x] **Step 3: Emit progress from backend commands**

Add helper in `commands.rs`:

```rust
fn emit_progress(app: &tauri::AppHandle, event: crate::events::ProgressEvent) {
    let _ = tauri::Emitter::emit(app, "launcher://progress", event);
}
```

Use it in `launch_game` before launch:

```rust
emit_progress(
    &app,
    crate::events::ProgressEvent::message("launch", "starting", "starting game launch"),
);
```

Change `launch_game` signature to include `app: tauri::AppHandle`.

- [x] **Step 4: Verify**

Run:

```bash
pnpm test
pnpm build
cd src-tauri && cargo test
```

Expected: all tests pass.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src/views/MainLauncher.vue src/lib/commands.ts src/types/launcher.ts
git commit -m "feat: connect launcher actions"
```

## Task 17: Final Integration Checks

**Files:**
- Modify: `README.md`
- Create: `docs/runtime-contracts.md`

- [x] **Step 1: Document runtime contracts**

Write `docs/runtime-contracts.md`:

```markdown
# Runtime Contracts

## Mod Release Assets

The launcher expects these assets on STFC mod GitHub releases:

- `stfc-community-mod-windows-x64.tar.zst`
- `stfc-community-mod-windows-x64.tar.zst.sha256`
- `stfc-community-mod-macos-universal.tar.zst`
- `stfc-community-mod-macos-universal.tar.zst.sha256`

The Windows archive contains `version.dll` at the archive root.
The macOS archive contains `libstfc-community-mod.dylib` at the archive root.

## Launcher Self-Update

The launcher repository publishes Tauri updater artifacts and a `latest.json` endpoint consumed by the Tauri updater plugin.

## Windows Launch Modes

Managed mode keeps mod files in launcher app data.
Fallback proxy-DLL mode copies `version.dll` into the game folder and keeps config/logs there, matching the current Windows mod behavior.
```

- [x] **Step 2: Update README**

Append to `README.md`:

```markdown
## Launcher Runtime

This launcher is a Tauri 2 app for macOS and Windows. It downloads the platform mod library from GitHub releases into managed app data, supports a stable/prerelease mod channel toggle, launches STFC with mod injection, and opens a separate config editor window.

Runtime asset contracts are documented in `docs/runtime-contracts.md`.
```

- [x] **Step 3: Run full validation**

Run:

```bash
pnpm test
pnpm build
cd src-tauri && cargo test
pnpm tauri build --debug
git diff --check
```

Expected: all tests pass, frontend build passes, Tauri debug build succeeds, whitespace check passes.

- [x] **Step 4: Review final diff**

Run:

```bash
git status --short
git diff --stat
```

Expected: only intended launcher files, docs, dependency manifests, and lockfiles are changed.

- [x] **Step 5: Commit**

```bash
git add README.md docs/runtime-contracts.md
git commit -m "docs: add runtime contracts"
```

## Execution Notes

- Prefer managed Windows injection first. The proxy-DLL fallback is already approved by the spec, but status/logs must clearly identify fallback mode.
- Do not copy the mod into the STFC game folder in managed mode.
- Do not advance installed mod state until checksum verification, extraction, and atomic replacement succeed.
- Do not write STFC `.version` until Xsolla finalization succeeds.
- Do not add a persistent `Locate Game` button to the main UI. Trigger location only from `Launch Game` when the game path is unknown.
- Keep CI publishing automation outside this implementation plan.
