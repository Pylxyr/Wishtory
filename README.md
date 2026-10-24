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

Checks that CI also runs: `npm run check`, `npm run build`, and `cargo test --locked --manifest-path src-tauri/Cargo.toml`.

## Releasing

Releases are built by GitHub Actions and contain only the NSIS setup installer (`Wishtory_<version>_x64-setup.exe`) and a SHA256 checksum file.

1. Set the same new version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, then push to `main`.
2. Open **Actions -> Release -> Run workflow** on GitHub and click the button.

The workflow checks that the three versions agree, runs the tests, builds on `windows-latest`, creates the tag and GitHub Release, and attaches the setup exe and its `.sha256`. Pushing a tag like `v0.2.0` runs the same workflow; the tag must match the project version. The installer is not code-signed, so SmartScreen may warn on first run.

Data is stored in `%APPDATA%\app.wishtory.desktop\wishtory.db`.

## Game metadata updates

Wishtory checks its signed, data-only catalog quietly at startup, at most once per day. Wish records and account identifiers are never sent with this request. A verified catalog is cached locally for offline use; names and rarities already stored in wish history remain usable if an item is missing from the catalog.

The weekly **Review game catalog updates** workflow reads EnkaNetwork's public game metadata and Paimon.moe's item-key catalog, validates all three games, preserves previously known entries, and opens a review PR. A maintainer reviews and merges that PR; the **Publish signed game catalog** workflow then updates the rolling data-only release. Catalog changes do not alter wish or guarantee mechanics.

See [the catalog update guide](docs/game-catalog-updates.md) for source details, secret setup, review cadence, and rollback steps.

To enable publishing, add a repository Actions secret named `CATALOG_SIGNING_KEY` with the PEM contents of the locally generated `catalog/catalog-signing-key.pem`. The matching public key is pinned in `catalog/catalog-public-key.txt`; the private key is ignored by Git. Until the secret is configured and the first catalog PR is merged, the app continues using its bundled catalog and existing offline item mappings.

Run `npm run catalog:test` for source-shape and regression fixtures, `npm run catalog:update` to build a candidate locally, and `npm run catalog:sign` to create a signed envelope when the signing key is available.

## Credits and references

Wishtory relies on these independent projects and references for compatibility and verification:

- [EnkaNetwork's API documentation and processed game stores](https://github.com/EnkaNetwork/API-docs/tree/master/store) provide the item IDs, names, rarities, and character traits used to build Wishtory's updateable game catalog.
- [Paimon.moe](https://paimon.moe/) and its [open-source item data](https://github.com/MadeBaruna/paimon-moe/tree/main/src/data) inform native Genshin backup import/export. Wishtory uses its exact item keys to match backup records reliably, and its documented read-only cache lookup approach informed history-link discovery.
- The [UIGF project](https://uigf.org/) defines the interchange format Wishtory supports for moving wish histories between compatible tools.
- HoYoverse's official announcements help us check updated game terminology and mechanics, including the [Honkai: Star Rail Path of Elation](https://www.hoyolab.com/article/43681649) and [Zenless Zone Zero's new specialties](https://zenless.hoyoverse.com/en-us/news/166000).

These links credit the projects and sources that helped make Wishtory's catalog and compatibility features possible. Wishtory is an independent fan project and is not affiliated with or endorsed by these projects or HoYoverse.

## UIGF support declaration

Wishtory imports and exports the [UIGF](https://uigf.org) interchange standard:

- UIGF v4.x (Genshin, Star Rail, ZZZ), export as v4.2 (including Star Rail event pool types 21/22)
- Import of older UIGF (v2.x / v3.x, hk4e) and SRGF v1.0 (hkrpg)
- Times are kept as the account's server-local time and normalised to `YYYY-MM-DD HH:MM:SS`; a missing time zone is guessed from the UID region
- Rows without a rarity cannot be used for pity and are skipped (the import summary reports how many)
- Export to UIGF v3.0 (hk4e) and SRGF v1.0 (hkrpg) for older trackers

### Paimon.moe

Wishtory imports and exports Paimon.moe's native Genshin local-data JSON backup. Since those rows do not contain HoYoverse record IDs or rarities, Wishtory derives rarity and item names from its bundled or updated Paimon item catalog and creates stable local IDs for repeat-import deduplication. Unknown catalog items are reported as skipped rather than assigned an assumed rarity. Paimon.moe exports cover one Genshin account at a time; API-synced item IDs are mapped through exact catalog keys or English names. An unmapped or localized name may not be exportable in this format. UIGF remains the recommended format for moving records between trackers.

## Project layout

```
src/            Svelte 5 frontend (pages, components, runes-based store)
src-tauri/src/  Rust: discover, authkey (read-only cache parsing), fsutil, fetch, db, pity, uigf, commands
.github/        CI and Release workflows
```

## License

MIT
