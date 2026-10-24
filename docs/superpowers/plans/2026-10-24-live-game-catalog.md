# Live game catalog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver independently updateable, signed game metadata with offline fallback and a validated source-update workflow.

**Architecture:** Embed a bundled catalog and expose a Rust-managed catalog store to imports, exports, and the UI. Fetch a Wishtory-hosted signed JSON catalog in the background no more than once daily, verify and atomically replace the cached data, and open PRs from a scheduled upstream-data generator.

**Tech Stack:** Rust/Tauri, `ring` Ed25519 verification, Svelte 5, Node 22 scripts, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-24-live-game-catalog-design.md`

## Global Constraints

- Existing SQLite records and UIGF/SRGF/Paimon formats remain compatible.
- Runtime requests go only to a Wishtory-controlled HTTPS asset; they contain no user history or identifiers.
- Only signed, supported-schema catalogs are installed; failure always preserves the last-known-good or bundled catalog.
- Unknown pull name, rarity, and type continue to be displayed from the raw record.
- Banner mechanics are not remotely changed.

## Review Focus

- Invalid or forged signatures must never replace the catalog; test valid and tampered assets.
- Offline, timeout, and malformed feeds must preserve current/bundled metadata; test update outcomes.
- Duplicate or malformed catalog entries must be rejected without partial application; test validation.
- New Paimon item mappings must not guess a key or rarity; test native import/export lookups.
- Incomplete upstream data must not erase existing catalog items; test generator merge behavior.

---

### Task 1: Catalog schema, validation, signature verification

**Files:** Create `src-tauri/src/catalog.rs`, `catalog/catalog.json`; modify `src-tauri/Cargo.toml`, `Cargo.lock`, `src-tauri/src/main.rs`.

**Interfaces:** `Catalog::bundled()`, `Catalog::parse_and_validate(bytes)`, `Catalog::verify_signature(bytes, signature)`, plus serializable `CatalogItem` and `Catalog`.

- [x] Add tests for valid multi-game data, duplicate keys, invalid rarity/trait, unsupported schema, oversized content, valid Ed25519 signatures, and modified bytes.
- [x] Run the focused Rust tests and observe failures for missing catalog implementation.
- [x] Implement strict catalog parsing and pinned-key signature verification using the `ring` crate.
- [x] Seed a small bundled catalog and retain the existing offline item maps as fallback; full initial source refresh runs in the network-enabled scheduled workflow.
- [x] Run focused tests and `cargo test --locked --manifest-path src-tauri/Cargo.toml`.

### Task 2: Cached store and background refresh

**Files:** Modify `src-tauri/src/catalog.rs`, `src-tauri/src/main.rs`, `src-tauri/src/commands.rs`, `src/lib/api.ts`, `src/App.svelte`; tests in `catalog.rs`.

**Interfaces:** Managed `CatalogStore`, `get_catalog`, `refresh_catalog` commands; frontend returns only catalog version/items and a non-sensitive update status.

- [x] Add tests for current cache preference, invalid cache fallback, daily check gating, invalid-feed preservation, and cache-write recovery.
- [x] Implement per-app-data versioned cache with an 8 MiB payload cap, HTTPS-only fixed release endpoint, short timeout, signature/schema checks, and monotonic versions.
- [x] Initialize from the highest valid cache or embedded catalog; trigger the daily refresh asynchronously after app mount.
- [x] Verify tests and run frontend checks/build.

### Task 3: Use catalog in Paimon import/export and UI traits

**Files:** Modify `src-tauri/src/uigf.rs`, `src-tauri/src/commands.rs`, `src/lib/elements.ts`, `src/lib/types.ts`, `src/lib/state.svelte.ts`, `src/pages/Dashboard.svelte`, `src/lib/components/PityRing.svelte`.

**Interfaces:** Catalog-aware import/export helpers and a frontend catalog store; exact existing commands/formats remain intact.

- [x] Add tests proving newly cataloged Paimon keys import/export and existing unknown-item behavior remains explicit.
- [x] Pass managed catalog to Paimon import/export; retain test-only bundled wrappers.
- [x] Use current catalog traits for Genshin/HSR/ZZZ five-star styling, with neutral fallback for unknown names.
- [x] Run full Rust tests, Svelte check, and production build.

### Task 4: Automated source review and catalog publishing

**Files:** Create `scripts/update-catalog.mjs`, `scripts/sign-catalog.mjs`, fixture tests, `.github/workflows/catalog-proposal.yml`, `.github/workflows/catalog-release.yml`; modify `package.json`, README.

**Interfaces:** Deterministic script downloads public Enka stores, normalizes recognized three-game item fields and localization hashes into the Wishtory schema, validates non-regression, and writes a candidate. Signing workflow reads `CATALOG_SIGNING_KEY` only from Actions secrets.

- [x] Add fixtures/tests for Genshin, HSR, and ZZZ upstream shapes; missing traits and incomplete feeds fail closed, and absent upstream records do not delete known rows.
- [x] Implement candidate generation and secure signing without logging secrets.
- [x] Add weekly scheduled PR workflow and post-merge catalog asset publication workflow with least-privilege permissions.
- [x] Document one-time secret setup, provenance, manual review, and catalog rollback.
- [x] Run script tests and CI-equivalent checks.

### Task 5: Final safety review

- [x] Review network host pinning, command registration, catalog size/shape validation, secret handling, and raw-data fallbacks.
- [x] Run `npm run check`, `npm run build`, `cargo test --locked --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] Report the one-time GitHub secret setup requirement and limits of local network access for live-source validation.

## Verification note

The fixture tests pass, but the local environment could not resolve `raw.githubusercontent.com`, so the live-source generator could not be dry-run locally. The scheduled GitHub Action has network access and will be the first full upstream integration run. The full-catalog candidate must still be reviewed and merged, and GitHub Actions needs the `CATALOG_SIGNING_KEY` secret before it can publish the signed feed.
