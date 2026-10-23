# Wishtory

Wishtory is a Windows desktop app that keeps your **Genshin Impact**, **Honkai: Star Rail** and **Zenless Zone Zero** wish / warp / signal history in one local database, with pity counters, 50/50 state, statistics, and UIGF import/export.

## Safety model

Wishtory is designed so that **it cannot get your account banned**:

- It never attaches to, injects into, or talks to the game process.
- It never modifies, locks, or writes to any game file or folder.
- It never sets a system proxy and never uses man-in-the-middle techniques.
- It never needs administrator rights (the installer is per-user).
- It only performs **read-only** file access (from Rust) to:
  - `Player.log` / `output_log.txt` (to find where the game is installed), and
  - the newest `webCaches/<version>/Cache/Cache_Data/data_2` (to find the history link the game already created). If the file is locked, it is copied to your temp folder first.
- The history link (authkey) is only ever sent to the official HoYoverse history API for the matching game.

This is the same approach used by Paimon.moe's PowerShell scripts and other community tools.

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

Data is stored in `%APPDATA%\app.wishtory.desktop\wishtory.db`.

## UIGF support declaration

Wishtory imports and exports the [UIGF](https://uigf.org) interchange standard:

- UIGF v4.x (hk4e, hkrpg, nap), export as v4.2
- Import of older UIGF (v2.x / v3.x, hk4e) and SRGF v1.0 (hkrpg)
- Export to UIGF v3.0 (hk4e) and SRGF v1.0 (hkrpg) for older trackers

## Project layout

```
src/            Svelte 5 frontend (pages, components, runes-based store)
src-tauri/src/  Rust: discover, authkey (read-only cache parsing), fetch, db, pity, uigf, commands
```

## Disclaimer

Wishtory is a fan project and is not affiliated with or endorsed by HoYoverse / COGNOSPHERE.

## License

MIT
