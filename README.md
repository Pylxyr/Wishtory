# Wishtory

Wishtory is a Windows desktop app that keeps your **Genshin Impact**, **Honkai: Star Rail** and **Zenless Zone Zero** wish / warp / signal history in one local database, with pity counters, 50/50 state, statistics, and UIGF import/export.

## Safety model

Wishtory is designed to be as safe as the well-known community tools: it uses a read-only look at files the game already wrote, plus the official history API.

- It never attaches to, injects into, or talks to the game process.
- It never modifies, locks, or writes to any game file or folder.
- It never sets a system proxy and never uses man-in-the-middle techniques.
- It never needs administrator rights (the installer is per-user).
- It only performs **read-only** file access (from Rust) to:
  - `Player.log` / `output_log.txt` (to find where the game is installed), and
  - the newest `webCaches/<version>/Cache/Cache_Data/data_2` (to find the history link the game already created). If the file is locked, it is copied to your temp folder first.
- The history link (authkey) is only ever sent to the official HoYoverse history API for the matching game. Redirects are disabled, so it cannot be forwarded elsewhere.

This is the same approach used by Paimon.moe's PowerShell scripts and other community tools.

### Disclaimer

No tool can offer a 100% guarantee. HoYoverse can change its terms of service or its detection at any time, and nothing here can promise how the publisher will treat any third-party tool. Use Wishtory at your own risk.

Wishtory is an unofficial fan project and is not affiliated with or endorsed by HoYoverse / COGNOSPHERE.

## Usage

1. **Open the history page in-game** (Wish → History / Warp → Records / Signal Search → History) and let it load.
2. **Switch to Wishtory and click Refresh.**

If you see "history page not found", you haven't opened the page yet (or the link expired). Open it again and retry. You can also paste a history link manually on the Dashboard.

## Build

Prerequisites: Windows 10/11, [Node.js 20+](https://nodejs.org), [Rust stable (MSVC)](https://rustup.rs), Visual Studio Build Tools (C++ workload), and WebView2 (preinstalled on Win 10/11).

```powershell
npm install
# placeholder icons are included; to replace them: npm run tauri icon path\to\your-icon.png
npm run tauri dev
npm run tauri build
```

Checks that CI also runs: `npm run check` and `cargo test --manifest-path src-tauri/Cargo.toml`.

## Releasing

Releases are built by GitHub Actions and contain only the NSIS setup installer (`Wishtory_<version>_x64-setup.exe`) and a SHA256 checksum file.

1. Set the same new version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, then push to `main`.
2. Open **Actions -> Release -> Run workflow** on GitHub and click the button.

The workflow checks that the three versions agree, runs the tests, builds on `windows-latest`, creates the tag and GitHub Release, and attaches the setup exe and its `.sha256`. Pushing a tag like `v0.2.0` runs the same workflow; the tag must match the project version. The installer is not code-signed, so SmartScreen may warn on first run.

Data is stored in `%APPDATA%\app.wishtory.desktop\wishtory.db`.

## UIGF support declaration

Wishtory imports and exports the [UIGF](https://uigf.org) interchange standard:

- UIGF v4.x (hk4e, hkrpg, nap), export as v4.2
- Import of older UIGF (v2.x / v3.x, hk4e) and SRGF v1.0 (hkrpg)
- Times are kept as the account's server-local time and normalised to `YYYY-MM-DD HH:MM:SS`; a missing time zone is guessed from the UID region
- Rows without a rarity cannot be used for pity and are skipped (the import summary reports how many)
- Export to UIGF v3.0 (hk4e) and SRGF v1.0 (hkrpg) for older trackers

### Paimon.moe

Paimon.moe's own backup file (the JSON from its settings page) is not UIGF: from the public converter source it stores per-banner pulls with an item name, time and gacha code, but no record ids and no rarity. Importing it directly would mean inventing ids, which would create duplicates the next time you refresh from the game. Native Paimon.moe import and export is therefore not supported yet. Convert a Paimon.moe backup to UIGF first (for example with the community "paimon wish uigf converter") and import that. Whether Paimon.moe itself accepts a Wishtory UIGF export has not been verified.

## Project layout

```
src/            Svelte 5 frontend (pages, components, runes-based store)
src-tauri/src/  Rust: discover, authkey (read-only cache parsing), fsutil, fetch, db, pity, uigf, commands
.github/        CI and Release workflows
```

## License

MIT
