# STFC Mod New Launcher

Cross-platform Tauri 2 launcher for the STFC Community Mod.

## Launcher Runtime

This launcher is a Tauri 2 app for macOS and Windows. It downloads the platform mod library from GitHub releases into managed app data, supports a stable/prerelease mod channel toggle, launches STFC with mod injection, and opens a separate config editor window.

Runtime asset contracts are documented in `docs/runtime-contracts.md`.

## Tech Stack

- Tauri 2
- Rust 2021
- Vue 3
- TypeScript
- Vite
- Vitest

## Development

```bash
# Install dependencies
pnpm install

# Run development server
pnpm tauri dev

# Run tests
pnpm test

# Build for production
pnpm tauri build
```

## Project Structure

```
src-tauri/          # Rust backend
src/                # Vue frontend
src/lib/config/     # Config TOML model (ported from modconfig)
src/views/          # MainLauncher, ConfigEditor
src/components/     # LCARS UI components, config components
```

## Commands

- `pnpm dev` - Start Vite dev server
- `pnpm build` - Build frontend
- `pnpm test` - Run Vitest tests
- `pnpm tauri dev` - Run Tauri in dev mode
- `pnpm tauri build` - Build Tauri app
- `cd src-tauri && cargo test` - Run Rust tests
```