# Live game catalog: implementation plan

**Goal:** Independently updateable, signed game data with offline fallback and a reviewed publishing workflow (phase 1, done), and a single versioned catalog that explains each update and previews its effect on saved history before applying it (phase 2).

**Spec:** `docs/superpowers/specs/2026-10-24-live-game-catalog-design.md` is the source of truth. Change it in the same pull request as any behaviour it describes.

**Architecture:** A Rust core (`crates/wishtory-core`) owns the data, the database and the network. `crates/wishtory-bridge` exposes it to the Flutter app (`app/`) through flutter_rust_bridge. Node 22 scripts (`scripts/`) and GitHub Actions (`.github/workflows/`) build, review and publish the signed feeds.

**Tech stack:** Rust 1.99 (`rust-toolchain.toml`), `ring` Ed25519, `rusqlite`, `reqwest`; Flutter (`flutter-toolchain.txt`), Riverpod, flutter_rust_bridge 2.13; Node 26; GitHub Actions.

## Global constraints

- Existing SQLite records and UIGF, SRGF and Paimon formats stay compatible. Catalog updates never write to the `pulls` table.
- Runtime requests go only to the repository's fixed HTTPS release assets, and contain no history, UID or identifier. Impact on saved pulls is computed on the PC.
- Only signed, supported-schema data is used. Any failure keeps the current or bundled data.
- Published asset names and envelope shapes that installed apps read do not change. A new field in `rules.json` needs an app release first.
- Unknown item names, rarities and kinds keep being shown from the raw record.
- Follow the conventions in `docs/game-catalog-updates.md` (banner windows, rerun channels, pity rules).

## Review focus

Reviewers, human or automated, should check:

- Forged, tampered or downgraded feeds and manifests are rejected; tests cover valid and tampered input.
- Offline, timeout and malformed responses preserve the current data.
- Duplicate or malformed entries are rejected without a partial install.
- `pulls` is never written by an update, and `pull_banners` is re-derived only in one transaction, after Apply.
- The impact shown on the review page is what Apply does; it is recomputed if the data changed.
- Nothing about the person's history appears in a request, header, log line or error message.
- Incomplete upstream data never erases known items.
- Published feeds stay readable by installed apps (legacy assets, shapes, rules field policy).

## Phase 1: signed feeds (done)

### Task 1: Schema, validation, signature verification

**Files:** `crates/wishtory-core/src/catalog.rs`, `catalog/catalog.json`, `catalog/catalog-public-key.txt`.

- [x] `Catalog::parse_and_validate`, `verify_envelope` with the pinned Ed25519 key, size and schema limits.
- [x] Tests for valid data, duplicates, invalid rarity or traits, unsupported schema, oversized input, valid and tampered signatures.

### Task 2: Cached store and background refresh

**Files:** `catalog.rs` (`Feed`, `FeedStore`), `crates/wishtory-core/src/ops.rs`, `crates/wishtory-bridge/src/api/core_api.rs`, `app/lib/state/refresh.dart`.

- [x] Versioned cache, newest valid copy wins, once-a-day gate, failed checks counted, monotonic versions.
- [x] Daily refresh after the app starts; the status string the UI knows (`updated`, `up-to-date`, `already-checking`).

### Task 3: Banner list and pity rules as feeds

**Files:** `catalog.rs` (`BannerSet`, `BannerStore`), `crates/wishtory-core/src/rules.rs`, `catalog/banners.json`, `catalog/rules.json`, `crates/wishtory-core/src/pity.rs`.

- [x] Banner windows and featured items as data; game rules (pity, odds, permanent pools, featured windows) as data.
- [x] `pull_banners` caches the banner each pull was made on, re-resolved when the list changes.
- [x] Item catalog used for Paimon import and export and for UI traits, with raw-record fallback.

### Task 4: Source review and publishing

**Files:** `scripts/update-catalog.mjs`, `scripts/track-banners.mjs`, `scripts/prepare-catalog.mjs`, `scripts/sign-catalog.mjs`, `scripts/fetch-published.sh`, `.github/workflows/catalog-proposal.yml`, `.github/workflows/catalog-release.yml`.

- [x] Weekly review PR from EnkaNetwork and Paimon.moe data; incomplete data fails closed and known rows are retained.
- [x] Versions set automatically (published plus one when content changed), signing with `CATALOG_SIGNING_KEY`, legacy asset copies kept.
- [x] Banner tracker drafts new Genshin events, reports differences, and corrects Paimon.moe's end time for the last phase of an update (HoYoverse's notices end it at `14:59` the day before the next update; Paimon.moe lists two as `17:59:59`).

### Task 5: Safety review

- [x] Host pinning, size and shape validation, secret handling, raw-data fallbacks.

## Phase 2: one catalog, change log and history impact (planned)

### Task 6: Manifest feed and change log generation

**Files:** `crates/wishtory-core/src/catalog.rs` (a `CatalogManifest` feed), a new `catalog/changes.json` (optional curated notes), `scripts/prepare-catalog.mjs` or a new `scripts/changelog.mjs`, `scripts/*.test.mjs`.

- [x] Define the manifest schema (`manifest_version`, `components`, `changes`, `generated_at`, `schema_version`) with strict validation and a size cap.
- [x] Generate `changes` by comparing the new feeds with the published ones; assign `major`, `minor` or `info` by the rules in the spec; merge notes from `catalog/changes.json`.
- [x] Tests: each severity rule, a banner window move, a removed banner, a rules change, an item rarity change, additions only, and no change.

### Task 7: Publishing order and the review PR

**Files:** `.github/workflows/catalog-release.yml`, `.github/workflows/catalog-proposal.yml`, `scripts/prepare-catalog.mjs`, `docs/game-catalog-updates.md`.

- [x] Upload component feeds first and the manifest last; sign the manifest with the same key; keep every existing asset.
- [x] Stamp the bundled manifest version in release builds like the other feeds.
- [x] Put the generated change list (with severities) in the review PR description.

### Task 8: Staging in the Rust core

**Files:** `catalog.rs` (`FeedStore` gains check-and-stage without install), `crates/wishtory-core/src/ops.rs`, `crates/wishtory-core/src/lib.rs`.

- [x] Download and verify the manifest and every component; keep the staged release on disk; re-verify on load; drop it when superseded.
- [x] Refuse a manifest whose components are missing, lower or mismatched; keep the current data and retry later.
- [x] Apply installs all components together and re-derives `pull_banners` in one transaction; a failure leaves the previous release in place.
- [x] Tests: tampered or downgraded manifests, partial publishes, crash safety, repeated checks, offline.

### Task 9: Impact analysis

**Files:** a new `crates/wishtory-core/src/impact.rs`, `pity.rs` (shared derivation), `db.rs` (read pulls).

- [x] Derive each saved pull's banner, 50/50 result, pity number and display facts under the current and the staged data; list pulls where they differ, with the reason and the change entry behind it.
- [x] Bound memory and size; page the result; report counts beyond the cap.
- [x] Tests: a moved window re-places specific pulls, a featured change flips a result, a rules change moves a pity number, an unchanged history reports nothing, and `pulls` is untouched.

### Task 10: Bridge API and generated code

**Files:** `crates/wishtory-bridge/src/api/core_api.rs`, `types.rs`, `convert.rs`, `app/lib/bridge/generated/**`, `app/lib/bridge/wishtory_api.dart`.

- [x] Expose catalog status, check, staged-release review (change log and impact pages), apply, dismiss and history.
- [x] Regenerate with `flutter_rust_bridge_codegen generate`; CI fails on any diff.

### Task 11: Settings card, notice and review page

**Files:** `app/lib/features/settings/settings_page.dart`, new files under `app/lib/features/settings/` (catalog card, review page), `app/lib/state/refresh.dart`, `app/lib/state/providers.dart`.

- [x] One **Game catalog** card with the single version and **Check for updates**; replace the separate catalog and banner buttons.
- [x] A startup notice and Settings badge when a release needs review.
- [x] A review page: change log by severity, affected pulls per account (before and after, why), **Apply** and **Not now**; a catalog history list.
- [ ] Tests for grouping, counts and notice states; accessible labels and keyboard use. (Written in `app/test/catalog_logic_test.dart`; not yet run: the Dart and Flutter checks could not run where this was built.)

### Task 12: Documentation and privacy

**Files:** `docs/game-catalog-updates.md`, this plan, the spec, `PRIVACY.md` (only if behaviour that it describes changes), `README.md` (one line).

- [x] Describe the manifest, severities, `catalog/changes.json`, publish order, and how to read the review page.

### Task 13: Final review

- [x] Check every invariant in the spec, and that no history leaves the PC.
- [ ] Run the verification commands below. (Rust, Node, bridge generation and the banner tracker ran clean; `flutter analyze` and `flutter test` still need a Flutter machine.)

## Verification

```text
cargo fmt --all --check
cargo test --locked --workspace
npm run catalog:test
flutter_rust_bridge_codegen generate   (no diff afterwards)
flutter analyze --no-fatal-infos
flutter test
```

Run what the environment allows and say which checks could not be run.
