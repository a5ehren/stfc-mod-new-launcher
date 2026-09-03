# STFC Community Mod Launcher

The launcher for the STFC Community Mod, for macOS and Windows. It keeps
your game and mod up to date, launches Star Trek Fleet Command with the
mod injected, and lets you edit mod settings without touching files by
hand.

## Installation

Download the latest installer from the Releases page:

- **macOS** — open the `.dmg` and drag the launcher into Applications.
- **Windows** — run the `.msi` installer.

The launcher updates itself, so you only need to install it once.

## First run

1. Start the launcher.
2. When asked, point it at your Star Trek Fleet Command game folder
   (**Select STFC game folder**).
3. Click **Update Mod** to download the latest mod.

That's it — click **Launch** to play.

## Everyday use

- **Launch** — starts the game with the mod loaded.
- **Update Game** — checks for and applies official game updates.
- **Update Mod** — downloads the newest mod release.
- **Stable / Prerelease** — pick the mod channel. Prerelease gets new
  features early but may be rougher.
- **Config** — lowers the bundled ModConfig editor into the launcher. Changes
  are loaded from and saved directly to the launcher's managed TOML file.

## Multi-instance mode

Run several game accounts at the same time, each in its own isolated
instance. Open the multi-instance wizard from the instances panel to set
up additional instances — the launcher handles the OS-level work (you'll
be asked for your password / a UAC prompt once per instance). Each
instance can be started, stopped, backed up, and restored independently.

## Bundled ModConfig editor

The configuration panel uses a build of the real
[STFC ModConfig](https://github.com/STFC-Mod/modconfig) project rather than a
launcher-specific copy of its interface. It is bundled with the launcher so
the editor works offline, remains compatible with the launcher release that
contains it, and can exchange TOML with the native launcher through a small
message bridge.

The generated ModConfig files under `public/_astro`, `public/flags`, and
`public/modconfig` are intentionally committed. Vite copies them into the
launcher build, so CI does not need a ModConfig checkout or a separately
running ModConfig server.

To refresh the embedded editor, check out the ModConfig repository next to
this repository as `../config`, then run:

```sh
cd ../config
pnpm install
pnpm build

cd ../launcher
pnpm sync:modconfig
```

Commit the refreshed files in all three `public` directories along with the
corresponding ModConfig source changes. `pnpm sync:modconfig` is a maintainer
command only; it is deliberately not part of the launcher CI build.

## Support

Found a bug or have a question? Open an issue on this repository.
