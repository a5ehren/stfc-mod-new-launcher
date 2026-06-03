# Cross-Platform STFC Mod Launcher Design

Date: 2026-06-03

## Context

This project is currently a Tauri 2, Vue, and TypeScript scaffold. It will become the macOS and Windows launcher for the STFC Community Mod.

The launcher replaces the current Swift macOS launcher in `../stfc-mod/macos-launcher` and preserves its user-facing behavior:

- Detect the installed STFC game.
- Check for and apply official STFC game updates through the Xsolla update plan.
- Launch STFC with the mod injected.
- Open the raw mod configuration.
- Surface update and launch failures in the UI.

The new launcher also adds:

- Mod install and updates from GitHub releases.
- Launcher self-updates from this launcher repository.
- A built-in config editor, ported from `../modconfig`.
- Windows support.
- A managed local mod install so the launcher does not ship with the mod library.

The UI should look like the current Swift launcher and use the classic-standard LCARS theme at `~/Downloads/lcars_theme` as the visual source: black background, Antonio font, classic LCARS colors, rounded frame geometry, data-cascade decoration, colored bars, and optional beep sounds. Any vendored theme assets must preserve required attribution and license notices.

## Scope

This spec covers runtime app behavior and architecture.

In scope:

- Tauri 2 app architecture.
- macOS and Windows behavior.
- Main launcher UI and config editor UI.
- STFC game discovery.
- STFC game update behavior.
- Mod release channel selection.
- Mod download, checksum verification, install, and update.
- Launcher self-update behavior.
- Migration of legacy config/log files.
- Stale Windows `version.dll` cleanup in managed mode.
- Launch and injection behavior.
- Logging and diagnostics.
- Test strategy.
- Release asset contracts required by the runtime.

Out of scope:

- Full CI and release publishing automation.
- Installer design.
- macOS notarization and signing workflow.
- Complete rollback for STFC game updates. Game updates are too large for practical rollback, so the safety boundary is staging and finalization ordering.

## Chosen Approach

Use a Tauri 2 app where the Vue frontend owns presentation and user interaction, while the Rust backend owns every operation that touches the filesystem, network update artifacts, process launch, game patching, migration, and self-update plumbing.

This keeps platform-specific and failure-prone behavior in Rust services with explicit interfaces and leaves Vue focused on LCARS UI state, progress display, and config editing.

## Backend Services

### GameLocator

`GameLocator` finds the STFC game root.

Detection order:

1. Persisted launcher choice.
2. Official launcher settings files.
3. Platform default install paths.
4. Manual locate flow when launch is requested and no valid game root is known.

The manual locate flow is not a permanent main-window button. It is triggered from `Launch Game` when required, validates the selected root, and persists it.

### GameUpdater

`GameUpdater` ports the existing Swift Xsolla update behavior to Rust.

Required semantics:

- Fetch the Xsolla update plan for the installed game version.
- Parse download, extract, patch, wait, and version actions.
- Download and extract into staging paths.
- Normalize relative patch paths and prevent staging escapes.
- Apply patch rules from staging.
- Defer delete rules until finalization.
- Defer writing `.version` until staged file operations succeed.
- Surface download, extract, patch, finalizing, cleanup, complete, and failure progress.

The existing hardening from the Swift launcher remains part of the design: final copy helpers must fail loudly, `.version` must not advance early, and deferred deletes must not remove a path that has a staged replacement.

Windows support should use the corresponding Xsolla platform identifier and path conventions. macOS behavior should preserve the existing `mac_os` flow.

### ModManager

`ModManager` handles STFC Community Mod installs and updates from GitHub releases.

Behavior:

- Default to the stable channel.
- Allow the user to toggle to the prerelease channel from the main window.
- Stable channel ignores prerelease releases.
- Prerelease channel includes prereleases.
- Select platform-specific release assets.
- Download to a staging directory.
- Verify SHA-256 checksum before install.
- Decompress the zstd archive.
- Atomically replace the installed managed mod library.
- Record installed version, channel, source release, checksum, and install time.

The launcher must not bundle the mod library. It downloads the platform-specific mod after the launcher starts or when an update is requested.

### MigrationService

`MigrationService` handles legacy files and cleanup.

Managed mode:

- Prompt when legacy config/logs are found.
- Move legacy config/logs into launcher-managed app data.
- Prompt before deleting a stale `version.dll` from the Windows game folder.
- Log every migrated or removed path.

Windows fallback proxy-DLL mode:

- If managed-folder injection is not technically reliable, the approved fallback is to copy `version.dll` into the game folder.
- In that mode, config and logs remain in the game folder, matching the current Windows mod behavior.
- The launcher must make fallback mode visible in status and logs.

Migration must stop on a failed move/delete and report the exact path and action that failed.

### LaunchService

`LaunchService` starts STFC with the installed mod.

macOS:

- Replace the old `stfc-community-mod-loader` helper with Rust backend launch logic.
- Launch the game with the managed dylib injected from launcher-managed storage.
- Preserve the old entitlement verification/application behavior where required for mod loading.

Windows:

- Prefer managed-folder injection without copying `version.dll` into the game folder.
- If that is not technically reliable, use the approved fallback: copy `version.dll` to the game folder and keep config/logs there like the current mod.
- Record whether the current launch path is managed injection or fallback proxy-DLL mode.

Launch warnings:

- If game or mod updates are available, warn every time and allow launch.
- Users must be able to stay on old game clients.
- If the mod is missing, prompt to install it before launch because there is nothing to inject.

### ConfigService

`ConfigService` owns reading and writing `community_patch_settings.toml`.

Behavior:

- Load the managed config file in managed mode.
- Load the game-folder config in Windows fallback proxy-DLL mode.
- If no config exists, start from defaults.
- Save only on explicit user action.
- Provide parse errors, write errors, and resolved config path to the UI.

The config model should be ported from `../modconfig`: typed config shape, defaults, field metadata, TOML parse/merge, TOML generation, and dynamic sync targets.

### DiagnosticsService

`DiagnosticsService` writes structured launcher logs to disk.

Behavior:

- Log startup checks, update checks, selected channels, downloads, checksum verification, migration, game updates, launch mode, launch command construction, and failures.
- Expose an `Open Logs` action in the main UI.
- Use logs as the support source of truth rather than a visible diagnostics console.

### SelfUpdateService

`SelfUpdateService` wraps the Tauri updater plugin.

Behavior:

- Check this launcher repository for updates on startup.
- Never install without user confirmation.
- Download and install through Tauri updater artifacts.
- Relaunch after a successful install.

The Tauri updater configuration requires signed updater artifacts, `bundle.createUpdaterArtifacts`, updater endpoints, and a configured public key. The process plugin is needed for relaunch behavior.

## Storage Layout

Default storage is per-user Tauri app data under the launcher identifier `com.stfcmod.launcher`.

Managed storage should include:

- `mods/` for installed platform mod libraries.
- `downloads/` or `staging/` for temporary update work.
- `community_patch_settings.toml` for managed config.
- `logs/` for launcher and migrated mod logs.
- `state.json` or equivalent for installed mod version, channel, launch mode, game path, migration status, and self-update state.

Windows fallback proxy-DLL mode is the exception: `version.dll`, config, and mod logs stay in the STFC game folder to match current Windows behavior.

## Main Window UI

The main window recreates the current Swift launcher's LCARS layout using the classic-standard theme.

Always visible actions:

- `Launch Game`
- `Open Raw Config`
- `Open Config Editor`
- `Open Logs`

Conditional actions near `Launch Game`:

- `Update Game`, only when a game update is available.
- `Update Mod`, only when the selected channel has a newer mod.

Those update actions must use distinct LCARS colors from inactive/placeholding panels.

Separate corner action:

- `Mod Channel` toggle, defaulting to stable and switchable to prerelease.

Status area:

- Show compact current operation status.
- Show progress for downloads, decompression, Xsolla phases, migration, and launch.
- Show actionable errors without hiding them only in logs.

## Config Editor Window

`Open Config Editor` opens a separate Tauri window.

Behavior:

- Load the active config through `ConfigService`.
- Keep edits in UI state until explicit `Save`.
- Prompt on dirty close with save, discard, or cancel choices.
- Keep generated TOML preview available but collapsed by default.
- Support `Open Raw Config` from the main window for direct OS-default editing.

Editor UI:

- Rebuild the `../modconfig` editor in Vue instead of embedding Astro/React.
- Use the same LCARS classic-standard visual language as the main launcher.
- Use a denser editor layout than the main window.
- Mirror modconfig sections: control, graphics, shortcuts, sync, UI, buffs, config, and patches.
- Preserve dynamic sync targets.
- Use natural controls: toggles for booleans, numeric inputs/sliders where metadata calls for them, text inputs for strings.

## Release Asset Contracts

### Mod Assets

The STFC mod GitHub release provides platform-specific zstd archives. Each archive contains only the mod library for that platform.

Required assets:

- `stfc-community-mod-windows-x64.tar.zst`, containing `version.dll` at the archive root.
- `stfc-community-mod-windows-x64.tar.zst.sha256`, containing the SHA-256 checksum for the Windows archive.
- `stfc-community-mod-macos-universal.tar.zst`, containing `libstfc-community-mod.dylib` at the archive root.
- `stfc-community-mod-macos-universal.tar.zst.sha256`, containing the SHA-256 checksum for the macOS archive.

`ModManager` selects the asset by platform and architecture. Windows is x64. macOS uses the universal archive.

### Launcher Self-Update Assets

The launcher repository provides Tauri-compatible updater artifacts.

Required runtime contract:

- Tauri updater endpoint metadata is available from the launcher repo.
- Artifacts are signed for Tauri updater verification.
- The configured public key in `tauri.conf.json` matches the release signing key.

CI automation for producing these assets is out of scope for this spec.

## Error Handling

General rules:

- Every long-running operation produces visible status and a disk log entry.
- Installed mod version is not advanced until replacement succeeds.
- Game version is not advanced until Xsolla finalization succeeds.
- Checksum failure prevents install.
- Staging directories are cleaned up or marked recoverable.
- Migration failures stop cleanup and report the failed path/action.
- Launch warnings do not block launch unless the mod is missing or the launch path is invalid.
- Game update rollback is out of scope because game updates are too large.

## Testing Strategy

Rust unit tests:

- Path normalization and path escape prevention.
- Game locator parsing and default-path validation.
- GitHub release asset selection.
- Stable versus prerelease channel behavior.
- SHA-256 checksum verification.
- zstd extraction and atomic mod install behavior.
- Migration moves and stale DLL cleanup decisions.
- Xsolla finalization ordering.

Rust/platform harness tests:

- macOS launch command/environment construction.
- Windows managed injection decision path.
- Windows fallback proxy-DLL decision path.
- Config/log path resolution in managed and fallback modes.

TypeScript tests:

- Config parse, merge, and generation parity with `../modconfig`.
- Dynamic sync target editing.
- Dirty state and save/discard/cancel behavior.

UI smoke tests:

- Main LCARS controls render.
- Conditional update buttons appear only when needed.
- Mod channel toggle changes release selection.
- Config editor opens in a separate window.
- TOML preview starts collapsed.
- Dirty close prompt appears.
- Open logs and open raw config actions route to backend commands.

Validation commands:

- `pnpm build`
- Rust unit tests through Cargo.
- Tauri build/test command appropriate to the implementation stage.
