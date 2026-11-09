# Wishtory 0.3 — UI rewrite: decisions and design

Status: design agreed before implementation. Anything marked **(open)** is a judgement call that
is cheap to change.

## 1. Platform decision

**Flutter (desktop) + the existing Rust code, bridged in-process with `flutter_rust_bridge` (FRB). Tauri is removed.**

Why the shell had to change: Tauri's frontend slot is a system webview. Flutter and Compose
Multiplatform are not webview frameworks, so the only way to keep Tauri would be to ship a
canvas-rendered web build inside a webview, which is worse than the current Svelte app on every
axis that matters here (text selection, accessibility, memory, startup). The Rust code, however,
is almost free of Tauri: only `commands.rs` (State / AppHandle), one `spawn_blocking` call in
`db.rs` and the `#[tauri::command]` attributes touched it. That code is now `crates/wishtory-core`.

Why Flutter over Compose Multiplatform:

| | Flutter | Compose Desktop |
|---|---|---|
| Rust integration | FRB: typed, async, streams, in-process | JNI/Panama or a sidecar process |
| Download size | ~20-30 MB | JVM runtime, 80-150 MB |
| 50k-row history list | slivers + `itemExtent`, trivial | LazyColumn works, heavier |
| Linux (GTK, Wine users) | first-class target | works, weaker accessibility |
| Theming | Material 3 + `ThemeExtension` | Material 3 |
| Packaging | `flutter build`, same CI shape as today | jpackage |

Costs accepted: Dart is new to the repo; the old `localStorage` state (aliases, last-refresh
times, theme) does not carry over (it lives in the old webview profile and cannot be read safely).
The **database and catalog caches do carry over**: `Core` keeps using
`app.wishtory.desktop` as its data folder, so history, cached feeds and Game locations survive.

State management: **Riverpod** (`Notifier` / `AsyncNotifier`, no code generation). It gives
cancellation-safe async loading (the old Svelte code hand-rolls a `stale` flag in every page),
testable providers and no build step. Preferences: `shared_preferences`. Dialogs: `file_selector`.
Window: the title and the minimum size (960x640) are set in the platform runners under `app/windows` and `app/linux`; no plugin is needed.

## 2. Who uses this and what they want

A player opens Wishtory right after pulling, or while deciding whether to pull. Ranked by how often:

1. **"Where am I?"** Current pity on each banner, soft/hard pity distance, and whether the next
   5★ is guaranteed. Must be answerable in under two seconds with no clicks.
2. **"Is the data fresh?"** One-click refresh from the game cache, with a clear result and a
   recoverable error when the link has expired (the most common failure).
3. **"What did I get, and how lucky was I?"** Chronological 5★ and 4★ list with the pity each cost
   and the 50/50 result.
4. **"Find a specific pull."** Search by name, filter by banner/rarity/date.
5. **Occasional:** import/export (moving trackers), second account, catalog updates, Linux game folder.

Design consequences: the Overview is a status board, not a report; History is a fast, dense,
keyboard-friendly tool; everything rare lives behind Settings or progressive disclosure.

## 3. Information architecture

```
+ Navigation rail ----------+ Page bar (game pages only) ---------------------------------+
| Wishtory                  | Genshin Impact  [Account v]   [Overview|History]  [Refresh v]|
| -- Games --               | Refreshed 2 hours ago                                        |
|  Genshin                  +--------------------------------------------------------------+
|  Star Rail                |                                                              |
|  Zenless Zone Zero        |   page content (max width 1200, centred)                     |
| -- Your data --           |                                                              |
|  Accounts                 |                                                              |
|  Import / Export          +--------------------------------------------------------------+
|                           | Status bar: privacy note, progress, catalog version          |
| ------------------------- +--------------------------------------------------------------+
|  Settings (dot = update)
|  Help
|  About
+---------------------------+
```

* **The games are destinations.** Choosing a game in the rail (or with `Alt+1/2/3`) always opens it:
  on Overview or History it switches the game and keeps the page; from Settings, About, Accounts, Help or
  Import / Export it also leaves that page, for whichever of Overview and History was open last. Nothing
  about a game choice is ever silent.
* **Overview and History are two views of one game**, so they are tabs in the page bar, not rail items.
  The rail only holds places: the games, the data that belongs to no single game, and the app pages.
* **Game and account are global context, not page settings.** The game is chosen in the rail, the account
  in the page bar, and every game page reads them. Import / Export has its own game picker, because its
  game-specific formats depend on it.
* **Responsive**: >=1100 px expanded rail with labels; below that an icon rail with tooltips. The page bar
  puts everything on one row from about 900 px of content width and wraps below that. The window minimum
  is 960x640, so a bottom bar is not needed. Cards reflow from 3 to 2 to 1 columns, and the History detail
  panel is a side panel from 1280 px and a dialog below. Nothing relies on hover.
* **Restored on launch**: last page, game and account.
* **Shortcuts**: `Alt+1/2/3` open a game, `Ctrl+R` refresh, `Ctrl+,` Settings, `F1` Help. Modifier keys are
  required on purpose: bare digits or `/` would steal keystrokes from text fields.

## 4. Viewing 5★ and 4★ pulls — the hybrid

No single presentation serves all four jobs, so there are three coordinated views:

1. **Overview → banner cards** (glance). One card per banner: big current pity number, a segmented
   bar to hard pity with the soft-pity zone marked, and a guarantee badge. Colour zones:
   normal → soft pity → hard-pity-close, each also differing in label text, never colour alone.
2. **Overview → selected banner detail** (recent luck). A *pity bar chart* of the last 5★ pulls,
   one bar per 5★, bar length = pity, bar tint = result (won 50/50, lost, guaranteed, unknown),
   icon + name + date under each bar, plus the average pity line. 4★ shown as a compact secondary row.
   This is the "was I lucky" view and replaces a plain list because lengths compare instantly.
3. **History → "Rare pulls" mode** (full chronological record). A timeline list of every 5★ (4★
   togglable), newest first, **one row per pull**: item icon, name, pity as a number *and* a bar
   scaled to the banner's hard pity, result chip (Won 50/50 / Lost 50/50 / Guaranteed), banner
   event name and date. Month headers group the rows. This is the "career" view and the one people
   screenshot.

History's other mode, **"All pulls" table**, is the dense spreadsheet-like view for search and audit
(§5). A segmented control switches between them and the choice is remembered.

At-a-glance priority for a pull row, in order: **item name → pity → result → banner → date**.
Date is last because the list is already chronological. Item icons are decorative-plus-recognition:
they load lazily from the existing Enka host and fall back to a rarity-coloured star glyph, so a
missing icon never looks like an error and works offline.

## 5. History

A dedicated page, because it is the only place with unbounded data.

* **Controls (one toolbar, wraps on narrow windows):** mode switch (Rare pulls / All pulls),
  banner chips (from the backend's per-game filter list), rarity (All pulls: segmented All / 5★ / 4★, worded
  per game as "S-rank/A-rank" for Zenless; Rare pulls: an "Include 4★" switch), search (`/`), date range, **Group by** (None / Banner
  event / Month), density (compact 32 px / comfortable 44 px), column picker, "Copy CSV", "Clear filters".
* **Filtering is client-side** over the already-loaded account list (the backend returns the whole
  history in one call; 50k rows is fine). Search is a case-insensitive substring match on item name.
* **Table mode:** sortable columns (time, item, rarity, pity), sticky header, `SliverFixedExtentList`
  so only visible rows are built. Sort and filter state does not reload data.
* **Progressive disclosure:** selecting a row opens a side detail panel (full time with zone,
  pool, banner event, gacha type, record id, copy button). Hidden by default; a row is never
  taller because of detail.
* **Result count** is a live region ("312 of 1,904 pulls").
* **Filter empty state** names the active filters and offers "Clear filters"; **no-data empty state**
  teaches the two-step Refresh flow.
* **(open)** Group-by-banner-event uses `banner_name`, which is `null` when the signed banner list
  cannot place a pull; those fall into an "Unplaced" group at the end.

## 6. Overview

Top to bottom: **status strip** (pulls, 5★ count, average 5★ pity, latest 5★, worth in
premium currency — "comparison value, not spend", as before) → **Live now** (running banners with
countdown on the account's server clock, hidden when none) → **banner cards grid** → **selected
banner detail** (§4.2) → collapsed "Refresh not working? Paste a link". Without an account: a
two-step onboarding empty state instead of zeros.

## 7. Refresh, errors, progress

* **Refresh** is a split button: *Refresh* (stops at the first already-saved page), *Full
  re-sync*, *Paste a link…*. Disabled while running.
* While running: a determinate-looking, but honest, progress panel fed by the Rust `Stream<String>`
  (one line per banner, same replace-by-prefix rule as before). A **result banner** stays in the
  status bar until dismissed: "Downloaded 312 pulls for UID …: 12 new, 300 already saved".
* **Errors never lose the user.** The backend's friendly message is classified (port of
  `errors.ts`) into a recovery card: title, why, numbered steps naming the exact in-game page,
  *Try again*, link to the matching Help topic, and on Linux "Game locations" shortcut. Rate-limit
  errors show a countdown on the retry button. Everything else is a toast; errors persist until dismissed.
* **Loading:** skeleton blocks (never spinners over empty space) and previous numbers stay on screen
  during a reload of the *same* account; another account never shows the previous account's numbers.

## 8. Other screens

* **Accounts:** card per account (game, UID, nickname, alias, pull count, last refreshed on this
  PC, timezone), rename (alias is local), remove with confirmation that states exactly what is
  deleted and that the game's own data is untouched. "Clear all history" lives here, behind a typed confirmation.
* **Import / Export:** two cards. Import: pick file → summary (format, accounts, added, skipped).
  Export: format list worded for humans ("UIGF v4.2 — recommended", "UIGF v3.0 for older Genshin
  trackers", "SRGF v1.0", "Paimon.moe backup"), scope (this account / all), then save dialog. Result
  is a summary, never silent success.
* **Settings:** Appearance (System / Dark / Light, history density), Game locations (Linux only; shows
  detected prefixes), Updates (app: "Check for updates" + View release; Catalog: version, date,
  "Check now", what changed), Diagnostics (opt-in in-memory log; page appears only when enabled).
* **Help:** searchable accordion, deep-linkable from error cards.
* **About:** version, licence, disclaimers, grouped credits (open links through the backend's allow-list).
* **First run:** a three-step tutorial dialog, dismissible, reopenable from Help.

## 9. Visual design system

Carried over from the old app because it was a good system, re-expressed as Flutter tokens:

* **Surfaces:** graphite (`#0D1015` background, `#151A22` panel) dark; cool grey-white light.
  One accent (`#5AA2FF` dark / `#1F5FD0` light) for **all three games**, so the app has one identity
  and games are told apart by name and abbreviation, not by three competing palettes.
* **Rarity colours are reserved:** gold = 5★, purple = 4★, never reused for anything else.
  Light-theme gold/purple are darkened to hold 4.5:1 on white.
* **Type:** Google Sans (UI/display), JetBrains Mono (UIDs, pity numbers, tabular figures),
  both bundled as assets (no runtime font download; the app stays local-first). Scale 12/14/16/22/32.
* **Density:** 8-pt grid; cards 12 px radius, hairline borders instead of shadows; compact
  tables by default.
* **Motion:** short (120-200 ms) fades, bar growth on first paint; respects the OS "reduce motion"
  setting (`MediaQuery.disableAnimations`).
* **Accessibility:** every rarity/result is conveyed by text or icon as well as colour; full keyboard
  operation with visible focus rings; `Semantics` labels on charts ("Banner Character Event: 62 of 90
  pulls, soft pity reached"); text scales with OS setting; contrast ≥ 4.5:1 for text and ≥ 3:1 for
  control borders (tokens carry the same values the old CSS audited).
* **Copy:** game-specific wording comes from one `GameInfo` table (S-rank vs 5★), as before.

## 10. Architecture

```
crates/wishtory-core/     Rust, no UI: ops, db, pity, fetch, uigf, catalog   (was src-tauri/src)
crates/wishtory-bridge/   FRB API: plain DTOs + one `Core` handle + progress stream
app/                      Flutter
  lib/
    bridge/               generated FRB code + thin `WishtoryApi` facade (the only importer of it)
    domain/               GameInfo, error classification, planning (pullsToGuarantee, live banners)
    state/                Riverpod providers: context (game/account/page), pity, history, refresh, prefs
    features/{overview,history,accounts,import_export,settings,help,about}/
    ui/                   design tokens (ThemeExtension), shared widgets (PityBar, RarityChip, EmptyState…)
  assets/{fonts,data}/    bundled fonts; icon + element tables generated from the old elements.ts
```

Rules: widgets never call the bridge; they watch providers. Providers depend on a `WishtoryApi`
interface, so tests substitute a fake with no native library. Pity maths stays in Rust (one source of truth,
already tested); Dart only formats and plans.

## 11. Build and release shape

* `cargo build -p wishtory-bridge` produces the native library FRB loads; `flutter build windows|linux`
  bundles it. CI runs the Rust gates (`cargo fmt --all --check`, `cargo test --locked --workspace`),
  `flutter analyze` + `flutter test`, a release build on both systems, and a check that the generated
  bridge code is current.
* Releases carry an Inno Setup installer and a zip for Windows (`packaging/windows`). Linux has no release
  download: `packaging/linux/install.sh` builds from source, installs under `~/.local` and cleans up, and
  the PKGBUILDs in `packaging/arch/` do the same through pacman. CI still builds the Linux app on every change.

## 12. Implementation notes (what changed while building)

* The backend now sends a typed `ErrorCode` instead of messages being matched by text, so error
  recovery cards (`domain/recovery.dart`) no longer depend on wording.
* `recent_fives` in the pity engine holds *every* 5★ of a banner, not only recent ones, so History's
  "Rare pulls" timeline joins its win/loss/guaranteed result from it instead of recomputing it in Dart.
* Per-account labels, last-refresh times, theme and density live in `shared_preferences`. The old
  webview's `localStorage` is not migrated; the database and signed-feed caches are.
* Item pictures still load from Enka.Network with no on-disk cache (Flutter's in-memory image cache
  only). A disk cache is a possible follow-up. **(open)**
