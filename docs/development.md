# Developer guide

For installing and using Wishtory, see the [README](../README.md).

## Build

Prerequisites: Windows 10/11, [Node.js 20+](https://nodejs.org), [Rust (MSVC)](https://rustup.rs), version 1.91 as pinned in `rust-toolchain.toml` (rustup installs it on first use), Visual Studio Build Tools (C++ workload), and WebView2 (preinstalled on Win 10/11).

```powershell
npm install
# placeholder icons are included; to replace them: npm run tauri icon path\to\your-icon.png
npm run tauri dev
npm run tauri build
```

Checks that CI also runs: `npm run catalog:test`, `npm run check`, `npm run build`, `cargo fmt --check` and `cargo test --locked` (both with `--manifest-path src-tauri/Cargo.toml`), plus `python3 -m unittest discover -s scripts -p "test_*.py"` and `actionlint` for the release automation.

## Project layout

```
src/            Svelte 5 frontend (pages, components, runes-based store)
src-tauri/src/  Rust: discover, authkey (read-only cache parsing), fsutil, fetch, db, pity, uigf,
                catalog (signed feeds), rules (pity rules), update, about, commands
catalog/        Banner list, pity rules and signing public key (the item catalog is generated and signed in CI)
scripts/        Catalog tooling (update, banner tracking, versioning, signing), release notes and the version bump script
docs/           This guide, releasing, catalog updates
.github/        CI, release and catalog workflows
```

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

This is the same approach used by Paimon.moe's PowerShell scripts and other community tools. The other network traffic is the signed catalog, banner list and pity rules and the release check, all from this repository's GitHub releases (`catalog.rs`, `update.rs`). None of it carries an account or wish data. Links the app opens in the browser are limited to a fixed list (`about.rs`).

## Game metadata updates

Wishtory checks its signed, data-only catalog quietly at startup, at most once per day. Wish records and account identifiers are never sent with this request. A verified catalog is cached locally for offline use; names and rarities already stored in wish history remain usable if an item is missing from the catalog.

The weekly **Review game catalog updates** workflow reads EnkaNetwork's public game metadata and Paimon.moe's item-key catalog, validates all three games, preserves previously known entries, and opens a review PR. A maintainer reviews and merges that PR; the **Publish signed game catalog** workflow then updates the rolling data-only release. Catalog changes do not alter wish or guarantee mechanics.

See [Game catalog updates](game-catalog-updates.md) for source details, secret setup, review cadence, and rollback steps.

To enable publishing, add a repository Actions secret named `CATALOG_SIGNING_KEY` with the PEM contents of the locally generated `catalog/catalog-signing-key.pem`. The matching public key is pinned in `catalog/catalog-public-key.txt`; the private key is ignored by Git. Until the secret is configured and the first catalog PR is merged, the app continues using its bundled catalog and existing offline item mappings.

Run `npm run catalog:test` for source-shape and regression fixtures, `npm run catalog:update` to build a candidate locally, `npm run banners:track` to draft banner events from Paimon.moe's schedule, and `npm run catalog:sign` to create a signed envelope when the signing key is available. Catalog, banner list and rules versions are set automatically when they are published.

## Import and export details

The README lists the supported formats. Details:

- UIGF v4.x covers Genshin, Star Rail and Zenless; export is v4.2 (including Star Rail event pool types 21/22).
- Older UIGF (v2.x / v3.x, hk4e) and SRGF v1.0 (hkrpg) are import-only, plus export to UIGF v3.0 (hk4e) and SRGF v1.0 (hkrpg) for older trackers.
- Times are kept as the account's server-local time and normalised to `YYYY-MM-DD HH:MM:SS`; a missing time zone is guessed from the UID region.
- Rows without a rarity cannot be used for pity and are skipped (the import summary reports how many).

### Paimon.moe

Wishtory imports and exports Paimon.moe's native Genshin local-data JSON backup. Since those rows do not contain HoYoverse record IDs or rarities, Wishtory derives rarity and item names from its bundled or updated Paimon item catalog and creates stable local IDs for repeat-import deduplication. Unknown catalog items are reported as skipped rather than assigned an assumed rarity. Paimon.moe exports cover one Genshin account at a time; API-synced item IDs are mapped through exact catalog keys or English names. An unmapped or localized name may not be exportable in this format. UIGF remains the recommended format for moving records between trackers.

## App update check

`update.rs` reads this repository's GitHub releases and compares tags with the running version. A release build is only offered releases; a pre-release build is also offered newer pre-releases. Tags that are not versions, such as `catalog-v1`, are ignored, and only release pages of this repository are ever opened. It never downloads or installs anything.
