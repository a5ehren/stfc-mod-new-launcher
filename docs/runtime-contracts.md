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