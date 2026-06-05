# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Install dependencies
pnpm install

# Run Tauri in dev mode (starts Vite + Rust backend)
pnpm tauri dev

# Run frontend tests (Vitest)
pnpm test
pnpm test:watch          # watch mode

# Run Rust tests
cd src-tauri && cargo test                    # all tests
cd src-tauri && cargo test test_name          # single test by name

# Run a single Vitest test file
pnpm test -- src/lib/commands.test.ts

# Lint/format (Biome)
pnpm check               # check only
pnpm lint                # check + write
pnpm format              # format + write

# Build
pnpm build:macos         # universal .app bundle (macOS)
pnpm build:windows       # MSI installer (Windows)
```

## Architecture

This is a **Tauri 2** app: a Rust backend (`src-tauri/`) with a Vue 3 frontend (`src/`). The two halves communicate via Tauri's `invoke()` IPC. All Tauri commands are registered in `src-tauri/src/lib.rs` and implemented in `src-tauri/src/commands.rs`. The frontend wraps every command in typed helpers in `src/lib/commands.ts`. Types shared across the boundary live in `src/types/launcher.ts` (TS) and `src-tauri/src/models.rs` (Rust) — keep them in sync; Rust uses `#[serde(rename_all = "camelCase")]` throughout.

### Rust backend modules

| Module | Responsibility |
|---|---|
| `app_state` | Shared `AppState` (paths, persisted state, status, diagnostics), startup recovery |
| `storage` | `ManagedPaths` discovery, JSON persistence of `PersistedState` |
| `commands` | All `#[tauri::command]` handlers |
| `models` | Serializable domain types (`LauncherStatus`, `PersistedState`, etc.) |
| `game_locator` | Locating/validating the STFC game install |
| `game_updater` | Fetching and applying Xsolla-based game update plans |
| `mod_manager` | Downloading, verifying, extracting, and installing the mod library |
| `github_releases` | GitHub Releases API queries for mod versions |
| `launch` | Building and executing the game launch plan (managed vs. proxy-DLL modes) |
| `migration` | Windows legacy file migration (proxy-DLL → managed mode) |
| `self_update` | Launcher self-update via `tauri-plugin-updater` |
| `config_service` | Read/write of the mod's TOML config file |
| `diagnostics` | Structured JSONL log file |
| `rsync_patch` | librsync delta patching for game updates |
| `xsolla` | Xsolla XML manifest parsing for game update planning |
| `errors` | `LauncherError` / `ErrorDto` types and conversions |
| `events` | `ProgressEvent` emitted to the frontend as `launcher://progress` |

### Frontend structure

- `src/views/MainLauncher.vue` — primary launcher UI
- `src/views/ConfigEditor.vue` — mod config editor (opened in a separate Tauri window)
- `src/components/lcars/` — LCARS-themed UI component library
- `src/components/config/` — config-specific form components
- `src/lib/commands.ts` — typed wrappers around all `invoke()` calls
- `src/lib/config/` — TOML config model (ported from modconfig)

### Key runtime concepts

- **Launch modes**: `managed` (mod files live in launcher app data) vs. `windowsProxyDll` (copies `version.dll` into game folder — legacy Windows path).
- **Mod channels**: `stable` / `prerelease` — controls which GitHub release is selected.
- **`AppState`**: holds two `Mutex`-guarded fields: `persisted` (what's saved to disk) and `status` (computed view surfaced to the frontend). They are updated independently; invalid persisted state on startup is quarantined and reset to defaults.
- **Progress events**: async operations emit `launcher://progress` events (type `ProgressEvent`) that the frontend subscribes to via `onProgress()`.
- **Mod release assets**: platform archives follow the naming in `docs/runtime-contracts.md` — `stfc-community-mod-{platform}.tar.zst` + `.sha256`.
- **App data root**: resolved via `ProjectDirs::from("com", "stfcmod", "launcher")` — the platform app data dir. Contains `mods/`, `staging/`, `logs/launcher.log.jsonl`, `state.json`, and `community_patch_settings.toml` (the mod config). Useful to know when debugging state or config issues.
- **Mod injection**: macOS managed mode uses `DYLD_INSERT_LIBRARIES` + `DYLD_LIBRARY_PATH`; Windows managed mode prepends the mods dir to `PATH`. The proxy-DLL mode copies `version.dll` into the game folder instead.

## Linting

Biome handles both linting and formatting. Indentation is tabs. JS/TS uses double quotes. A Husky pre-commit hook runs `biome check --write` via `git-format-staged` on staged files.
