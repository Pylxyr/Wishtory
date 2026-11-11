# Game catalog: signed updates, change log and history impact

This is the design reference for how Wishtory's game data is published, verified, installed and explained to the person using the app. Contributors and reviewers (human or automated) should follow it, and update it in the same pull request as any change to the behaviour it describes.

The matching task list is `docs/superpowers/plans/2026-10-24-live-game-catalog.md`. Operating instructions for maintainers are in `docs/game-catalog-updates.md`.

## Status

| Part | State |
| :--- | :--- |
| Three signed feeds (item catalog, banner list, pity rules), bundled copy, cache, daily check | Implemented |
| Weekly source review (Enka, Paimon.moe) and signed publishing | Implemented |
| One catalog version, a change log, and a preview of what an update does to saved history | Implemented (see "Catalog revisions") |

## Intent

Let Wishtory recognise new characters and weapons, and correct banner dates or rules, without an installer for every game patch. Pull history stays on the person's PC, and an update must never change how their history is read without telling them first.

## Goals

- Data-only updates: no code, no images, no executable content in a feed.
- Authenticity: every feed is Ed25519-signed with a key whose public half is pinned in the app.
- Safety: size limits, strict schemas, monotonic versions, atomic installs, and the last valid copy kept on any failure.
- Privacy: the update check is a plain request for fixed public files. It carries no UID, pull, game path or other account data, and nothing about the person's history is ever sent.
- Honesty about impact (planned): when an update changes how saved pulls are interpreted, the person sees which pulls and why, before anything changes.

## Non-goals

- Editing, deleting or re-timing raw pull records. Catalog data only changes how records are read.
- Remote code or remote changes to the app's mechanics. Game rules that change are data in `rules.json`; changes to how the app derives results are app releases.
- Third-party requests at run time. Only the repository's own release assets are contacted.

## Current design (implemented)

### Feeds

All feeds are assets of the repository's `catalog-v1` GitHub Release, each as `<asset>.envelope.json` (a payload plus its signature).

| Feed | Source file | Version field | Rust type | Holds |
| :--- | :--- | :--- | :--- | :--- |
| Items | `catalog/catalog.json` | `catalog_version` | `Catalog` (`FeedStore<Catalog>`) | Items per game: id, name, rarity, kind, traits, Paimon key |
| Banners | `catalog/banners.json` | `banners_version` | `BannerSet` (`BannerStore`) | Banner events: window, featured 5-star ids, optional gacha ids |
| Rules | `catalog/rules.json` | `rules_version` | `RuleSet` (`FeedStore<RuleSet>`) | Hard and soft pity, 50/50 odds, permanent pools, featured windows, Zenless Custom Search |

The code lives in `crates/wishtory-core/src/catalog.rs` (the `Feed` trait, `FeedStore`, `Catalog`, `BannerSet`, `BannerStore`) and `rules.rs`. `Feed` is the one implementation of "bundled copy plus signed, versioned, cached updates".

### Rules every feed follows

- `verify_envelope` checks the signature against the pinned key before anything is parsed. The document is then validated against its schema and size limit.
- A copy is installed only if its version is higher than the current one. Verified envelopes are cached as `<version>.json`; the newest valid cache wins unless the bundled copy is newer.
- The app asks at most once a day, or when the person presses an update button. A failed or offline check counts as a check, a failure keeps the current copy, and requests have fixed hosts and short timeouts.
- Releases stamp the published versions into the bundled files (`scripts/prepare-catalog.mjs --stamp`), so a fresh install is never outranked by older cached content.
- Installed apps are old apps too: published asset names, envelope shapes and the banner list's shape do not change in ways older apps cannot read. `rules.json` rejects unknown fields, so a new rules field needs an app release first. Legacy copies (`catalog.json`, `banners.json`) are kept for 0.0.2 apps.

### Publishing

`catalog-proposal.yml` runs weekly: it builds a candidate from EnkaNetwork and Paimon.moe data, drafts banner events (`scripts/track-banners.mjs`) and opens a review PR (or the "Banner list needs review" issue). `catalog-release.yml` runs after a merge: it fetches what is published (`scripts/fetch-published.sh`), sets versions with `scripts/prepare-catalog.mjs` (published plus one, only for files whose content changed), signs with the `CATALOG_SIGNING_KEY` secret and uploads the assets. The details and conventions (banner windows, rerun channels, pity rules) are in `docs/game-catalog-updates.md`.

### How catalog data touches saved history

Raw pulls (the `pulls` table) are never rewritten by a catalog update. Everything below is derived and can change when the data changes:

| Derived fact | Derived from | Where |
| :--- | :--- | :--- |
| Which banner a pull was made on | Pull time, `gacha_type`, `gacha_id`, banner list | `pity::resolve_banner(s)`, cached in the `pull_banners` table |
| Won, lost or guaranteed for a 5-star (50/50, 75/25) | Banner list (`featured_5`), rules (permanent pools, featured windows) | `pity::compute_with_banners`, `listed_as_featured` |
| Pity numbers, averages, hard and soft pity | Pull order and each pull's pool (`pull_pities` ignores the rules), rules only for the hard and soft limits shown beside them | `pity::compute_with_banners`, `pull_pities` |
| Name, rarity, kind and traits shown for a pull | Item catalog, with the raw record as fallback | Catalog lookups, `pity::display_rank` |

Before catalog revisions, a newer banner list was installed at once and `pull_banners` was cleared and re-resolved, so the person was told what changed in the list but not what it did to their pulls. `ops::refresh_banners`, `ops::refresh_catalog` and `BannerUpdate` are gone: every update now goes through a release.

## Catalog revisions (implemented)

### One catalog, one version

The person sees one **Game catalog** with one version number and one **Check for updates** action. Underneath it stays three signed feeds, so installed apps keep working.

A new signed feed, the **manifest** (`manifest.envelope.json`, `manifest_version`), describes a catalog release:

- `manifest_version`: the single, monotonic, user-visible catalog version.
- `components`: the exact versions the release is made of (`catalog`, `banners`, `rules`).
- `changes`: the change log for this release and the releases since the previous manifest (see below).
- `generated_at`, `schema_version`.

Older apps ignore the manifest. Publishing uploads the three component feeds first and the manifest last, so a manifest never points at a missing component. The app treats a manifest whose components are not all available and valid as "not ready yet" and keeps what it has.

### The change log

Each change is one entry with a stable `id`, a `scope` (`items`, `banners`, `rules`), a `game`, the ids it concerns, a short plain-language `summary`, and a `severity`:

| Severity | Meaning | Examples |
| :--- | :--- | :--- |
| `major` | Can change how many existing pulls are read, or changes the rules the numbers rest on | A banner window or featured character corrected, a banner removed, pity or 50/50 rules changed, an item's rarity changed |
| `minor` | A small correction with a narrow effect | A name or alias fix, a trait fix, a `gacha_id` added to an existing banner |
| `info` | Additions that cannot alter an existing pull | New characters, weapons or banners |

`scripts/changelog.mjs` generates the entries by comparing the new feeds with what is published, and assigns severity by fixed rules. A maintainer can add notes or raise a severity in `catalog/changes.json`, which is part of the review PR; the review PR description lists the generated entries so the reviewer sees what users will be told.

The fixed rules, as implemented in `scripts/changelog.mjs`:

| Severity | Entries |
| :--- | :--- |
| `major` | Banner removed, dates changed, featured 5-stars changed, pool changed, `gacha_id`s replaced or removed; item removed or its rarity changed; any change to a banner rule's hard pity, soft pity or odds; a banner rule added or removed; the permanent pool changed; the featured windows changed; Custom Search changed |
| `minor` | Item name, alias, kind, trait or Paimon key corrected; `gacha_id`s only added to a banner; a banner's name or 4-star list changed; a banner rule's label changed; changes published earlier without a list ("unlisted") |
| `info` | New items; new banners |

Entry ids are stable (`<scope>:<kind>:<game>[:<id>]`). Up to 40 items or banners of one kind per game get one entry each; beyond that they are grouped into one entry whose `ids` holds the first 100. `catalog/changes.json` is `{ "schema_version": 1, "notes": [{ "id", "note"?, "severity"? }] }`; a note applies to the generated entry with that id and a note whose id is not in the release is ignored. The manifest is published only when a component is newer than the one the published manifest names (or none is published yet). Components that were published without a manifest are reported as `unlisted` minor entries. At most 2,000 entries, 2 MiB.

### Staging, impact and applying

The app handles a release in three states:

1. **Current**: what the app uses now.
2. **Staged**: a release whose manifest and components have all been downloaded and verified, and kept on disk, but not used. A staged release is re-verified when loaded and dropped if a newer release replaces it.
3. **Applied**: the staged release becomes current in one step, and saved banner assignments are re-derived.

When a release is staged, the app computes the **impact** on the person's own history, on their PC (never on a server): for every saved pull, it derives the facts in the table above under the current data and under the staged data, and lists each pull where something differs, with the reason and the change entry that caused it. Per pull it records: the pull (time, item, rarity), the banner before and after, the 50/50 result before and after, the pity number before and after, and the display name or rarity if those change.

| Impact | What happens |
| :--- | :--- |
| No pull is affected | Applied automatically (a setting can turn this off), and recorded in the catalog history with its change log |
| At least one pull is affected, any severity | Not applied. The person is notified and can review the affected pulls, then **Apply** or **Not now** |

### What the person sees

- A **Game catalog** card in Settings: installed version, last check, **Check for updates**, and a clear state when an update is waiting.
- A notice when a staged release needs review: a Settings badge and a message at startup. It says the version, the number of changes by severity, and how many pulls and accounts are affected.
- A **review page**: the change log grouped by severity, then the affected pulls per account (time, item, banner before and after, result before and after, why). Nothing changes until **Apply** is pressed.
- A **catalog history** of applied releases with their change logs.

Notifications are in-app only. Wishtory has no background service, so a check happens while the app is open.

## Invariants (reviewers: check these)

1. A catalog update never writes to `pulls`. Only derived data (`pull_banners`) is re-derived, in one transaction, and only after the release is applied.
2. Impact is computed locally. Nothing about the person's history is in a request, a header, a log line or a bug report.
3. A feed or manifest is used only after its signature, schema, size and version checks pass. A failure at any point keeps the current data.
4. Installing a release is all or nothing across its components. A crash or a failed re-derivation leaves the previous release in place.
5. Versions only go up. Corrections and rollbacks are published as newer versions.
6. Published asset names and envelope shapes that installed apps read do not change.
7. The review page shows exactly what Apply will do. If the data changes between review and Apply (a newer release arrives), the impact is recomputed before anything is applied.
8. Every user-visible string avoids blame and jargon, and nothing relies on colour alone.

## Decisions

| Question | Decision | Why |
| :--- | :--- | :--- |
| One merged feed or a manifest over the three? | Manifest over three feeds | Older apps keep reading the feeds they know; no envelope shape changes |
| Who classifies severity? | Generated by fixed rules, optionally annotated in `catalog/changes.json` | Consistent and reviewable, with a human override |
| Auto-apply when nothing is affected? | Yes, with a setting | Quiet by default, and honest when it matters |
| Notify on any affected pull, or only major? | Any severity | The person decides what is worth applying |
| Where to keep staged releases and history? | Files in the app data folder next to the feed caches | No database migration for data that is only a cache of signed files |
| Listing size | All affected pulls, paged in the UI, with a hard cap and a count beyond it | Keeps memory and the screen bounded |
| Which copy of each feed is in use? | The version the current manifest names, exactly. If any part cannot be loaded, the whole bundled release is used | A release is only ever used whole; a stray newer cache can never be half adopted |
| Does a pulled pity number ever change with a release? | It is derived and compared like the other facts, but no release can move it today: `pull_pities` depends on pull order and pool, not on the rules | The code, not the first draft of this spec, is the truth. The tests show a rules change flipping a 50/50 result and leaving the pity number alone |
| What counts as "displayed item facts"? | The name and rarity shown for the pull: the catalog's, or the saved record's own when the item is not in the catalog | Kind and traits do not change how a pull is read |
| Component newer than the manifest names | "Not ready": nothing is staged, the next check tries again | The manifest for the newer release has not been published yet |
| Check flow | A request names one fixed public asset and nothing else (`Source::get(asset, max_bytes)`); a failed attempt counts as a check | No history can reach a request, and an offline start is not retried on every launch |
| Stored marker for derived banners | `derived_state` table (`CREATE TABLE IF NOT EXISTS`, no migration) holds the banner list version `pull_banners` was derived from | Written in the same transaction as the re-derivation; at start a mismatch clears the cache, which repairs a crash between the release becoming current and the database commit |
| Commit order | Component caches, then the manifest cache (this is what makes the release current), then the database commit; a database failure removes the manifest cache and restores the previous release in memory | The manifest is the single commit point, so a crash leaves the old release or the new one |
| "Not now" | The release stays staged, the start-up notice stops for that version, the Settings badge stays | The person can still review it any time |
| History kept | The last 20 applied releases, each with its 100 most important entries and the total | Bounded file size |
| Catalog request and history | Stage, apply and history are files `catalog-staged/`, `catalog-history.json`, `catalog-dismissed`, `manifest-last-check`, `manifest-cache/` in the app data folder | No migration |

## Testing

- Rust: impact derivation (a moved window re-places specific pulls, a featured change flips a 50/50 result, a rules change flips a 50/50 result and moves no pity number), staging and apply atomicity, stale or lower versions, a manifest whose components are missing or mismatched, tampered signatures, no change to `pulls`, and a request that carries no history.
- Node: change-log generation and severity rules, and publishing order.
- Dart: the review page's grouping and counts, and the notice states.
- Everything in CI (`cargo fmt --all --check`, `cargo test --locked --workspace`, `npm run catalog:test`, regenerated bridge code with no diff, `flutter analyze`, `flutter test`).
