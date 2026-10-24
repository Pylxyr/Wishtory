# Wishtory repository guidance

## Project context

Wishtory tracks history for HoYoverse gacha games and is implemented with Rust, Svelte, and Tauri. Treat this as orientation; inspect the actual repository to confirm supported games, data sources, storage model, and import/sync behavior.

## Workflow

- Start by inspecting `git status --short`, Cargo workspace/manifests/lockfiles, frontend manifests/lockfile, database/schema migrations, data-source adapters, Tauri config/capabilities, and CI. Preserve all existing work.
- Trace a record from source/import/sync through parsing, normalization, deduplication, persistence, ordering, and UI display before editing.
- Preserve the distinction between raw source data and normalized internal records if the architecture supports it. Keep per-game/per-source parsing rules isolated instead of spreading game-specific conditionals across shared logic.
- Verify timestamp units and semantics, timezone conversion, sort order, pagination/cursors, record IDs, nullable fields, duplicate detection, and history-window boundaries from source code and fixtures. Do not infer them from UI labels alone.
- Make repeated imports/syncs idempotent where intended. Avoid duplicate records, silently discarded rows, unstable ordering, mixed-game data, and destructive resets. Keep any schema migration backward compatible or provide an explicit migration and recovery path.
- Treat import/export and backup/restore as data-integrity boundaries. Validate input, handle malformed or partial files, avoid partial destructive writes, and report counts/errors. Never delete a user's history as an automatic “repair.”
- Add fixture-based tests for valid records, missing/unknown fields, duplicate imports, malformed payloads, pagination boundaries, timestamp/timezone boundaries, and migration/round-trip behavior when relevant.
- Never place cookies, session URLs, API keys, auth tokens, or private user identifiers in logs or committed fixtures. Sanitize real exports before using them in tests.
- Inspect Tauri command input validation and capabilities when touching filesystem, network, or import/export behavior; keep permissions narrow.
- Detect the actual package manager and supported scripts from the repository; do not invent commands or claim unrun checks passed.

## Completion report

State the observed root cause, any data-integrity implications, migration/rollback behavior, tests and commands actually run, and known limitations. Be explicit when live source APIs or game-version behavior could not be confirmed.
