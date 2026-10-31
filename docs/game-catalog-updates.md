# Game catalog updates

Wishtory separates metadata updates from app releases. The desktop app requests only signed data assets from this repository's `catalog-v1` GitHub Release: the item catalog, the banner list and the pity rules (three independent feeds, each published as `<name>.envelope.json`). It never sends a UID, pull, game cache path, or other account data with that request.

## Sources and review

- [EnkaNetwork processed stores](https://github.com/EnkaNetwork/API-docs/tree/master/store) provide stable item IDs, rarity, English names, localization hashes, and game traits for Genshin Impact, Honkai: Star Rail, and Zenless Zone Zero. The catalog updater normalizes source-specific values into Wishtory's game-specific traits.
- [Paimon.moe's published item data](https://github.com/MadeBaruna/paimon-moe/tree/main/src/data) provides exact native backup keys for Genshin items. Wishtory uses those keys to import and export native backups without guessing item names.
- [HoYoverse's official Honkai: Star Rail announcement](https://www.hoyolab.com/article/43681649) and [Zenless Zone Zero update announcement](https://zenless.hoyoverse.com/en-us/news/166000) help verify newly introduced paths, attributes, and specialties when upstream data changes. They are references for checking game terminology; the updater reads the machine-readable EnkaNetwork and Paimon.moe sources above.

These projects and their data remain theirs; Wishtory is an independent, non-affiliated client. Catalog records are factual game metadata, and the source links above are provided for attribution and review.

The **Review game catalog updates** workflow runs each Monday and can also be run manually from GitHub Actions after a game patch. It builds a candidate, checks required metadata and minimum per-game counts, retains known rows omitted by an upstream refresh, and creates or updates a PR. It also tracks the banner schedule (see below). Review the diff before merging. The action never edits wish records, and it never edits the pity rules (`catalog/rules.json`, see below), which are changed by hand in a PR.

After merge, **Publish signed game catalog** sets the versions, signs the catalog, the banner list and the rules using the repository secret and updates the `catalog-v1` release assets (`catalog.envelope.json`, `banners.envelope.json`, `rules.envelope.json`). It only publishes a file whose content differs from what is published now. The app checks once per day when online and keeps its last valid cache when a feed is unavailable or invalid.

Two details keep the version numbers trustworthy:

- The workflow reads what is published through `scripts/fetch-published.sh`. Only "the release does not exist yet" counts as a first publish; any other failure (network, rate limit) stops the run, because publishing from a wrong baseline could produce a version lower than the live one, which every app ignores.
- The app prefers a cached feed over the copy bundled in its installer when the cache's version is not lower. The **Release** workflow therefore stamps the published versions into the bundled `catalog/*.json` before building (`prepare-catalog.mjs --stamp`), so a fresh install is never outranked by older cached content after an upgrade.

App versions up to 0.0.2 asked for `catalog.json` and `banners.json`, names the workflow never uploaded, so their update checks always ended in a 404. The workflow keeps a copy of both envelopes under those old names so those apps start receiving updates.

## Banner list

`catalog/banners.json` lists banner events: which 5★ items each one featured and when it ran. It is signed with the same key and published as the `banners.envelope.json` asset of the `catalog-v1` release. The app checks it once per day and when you press **Update banners**, and keeps the last valid copy when the feed is unavailable or invalid.

- `start` and `end` are inclusive **server-local** clock times (`YYYY-MM-DD HH:MM:SS`), the same clock as the `time` of each wish record, so they are compared with it directly. Genshin Impact Phase II starts at `18:00:00` server time and the previous phase ends at `17:59:59`. Phase II ends at `14:59:59` on the day before the next version, and that version's Phase I is listed from `15:00:00` the same day, so windows never overlap. The real start is the update time (11:00 UTC+8, a different clock time per server), and event wishes cannot be pulled before it. Zenless Zone Zero follows the same rule: Phase II starts at `12:00:00`, Phase I of a version is listed from `15:00:00` the day before it opens, and a phase ends at `11:59:59` (Phase I) or `14:59:59` the day before the next version (Phase II). Exclusive Rescreening, the selectable rerun channel, is listed as its own event beside the live Exclusive Channels, with its three selectable agents as `featured_5`. A phase with two concurrent banners (Genshin Character Event Wish and Wish-2, ZZZ's two Exclusive Channels) lists both banner titles in `name`, joined with ` / ` in `featured_5` order.
- `pool` is the banner key used by the pity rules: `character` or `weapon` for every game, plus `collab` and `collab_weapon` for Honkai: Star Rail.
- `featured_5` holds the game item IDs of every 5★ featured in that event. A Genshin character event with two concurrent banners lists both characters. `gacha_ids` holds the `gacha_id` values the game reports for the banner (Honkai: Star Rail and Zenless Zone Zero) and may be empty: the app then learns which `gacha_id` belongs to which banner from the 5★ pulls that a listed banner features. Where several banners run at once, list one event per phase with every featured 5★ of that phase.
- Add new events in a PR. You never bump a version: when the file is published, `scripts/prepare-catalog.mjs` sets `banners_version` (and `catalog_version`) to one above the published value, or keeps the file's own number if that is already higher. Clients ignore lower or equal versions, so every correction is published as a newer version. Every `featured_5` ID must exist in `catalog/catalog.json` with rarity 5; a test checks this.

### Tracking the banner schedule

The weekly workflow runs `npm run banners:track`. Paimon.moe publishes Genshin Impact's schedule as data, so for Genshin character events the script:

- **drafts** events that are newer than the list, with windows converted to the conventions above (the draft has only the first banner's title, so add the second before merging);
- **reports** existing events that differ from Paimon.moe, without changing them, because the list was checked by hand against HoYoverse's notices;
- **reports** 5★ characters that are new in the item catalog, for every game. Honkai: Star Rail and Zenless Zone Zero have no public schedule, so this is how their new characters get noticed.

Drafts go into the review PR and the report is added to its description. When the check changes no file but has something to review, it opens or updates one issue titled "Banner list needs review", and closes it when there is nothing left. Always check drafts against the official notice: Paimon.moe is a community source and can lag or be wrong.

## Zenless rerun channels

Exclusive Rescreening and W-Engine Reverberation are separate Signal Search channels with their own counters: cumulative over every period, independent of the Exclusive and W-Engine channels (HoYoverse's channel notices). Wishtory handles them as two more pity pools, `rescreening` and `reverberation`.

- **Fetching:** the history API takes them as `real_gacha_type` 102 (Rescreening) and 103 (Reverberation) together with `init_log_gacha_base_type`, and reports their records as the ordinary types 2 and 3. Wishtory therefore stores the channel it asked for, and keeps a rerun channel's records only if none of them also came back from an ordinary channel (a server that ignored the request would otherwise relabel the Exclusive history). A channel the server cannot answer for is skipped and mentioned after the refresh; the other channels are still saved.
- **Banner list:** the list's shape is fixed for installed apps, so the rerun periods are ordinary `character` / `weapon` entries whose **id ends in `-rescreening` or `-reverberation`**. That suffix is how a pull finds its entry, and how Exclusive and W-Engine channel pulls avoid matching one. Keep it when adding a period; the tracker script does not draft these (they are not in Paimon.moe's data).
- **Pity numbers:** `rules.json` uses 90 for Rescreening and 80 for Reverberation, inherited from the channels they mirror. HoYoverse does not publish a count for them, so treat both as working figures.
- **No win / loss tracking:** the first S-Rank of each period is the player's own pick (the Special Guarantee), so there is no coin flip to record.
- **UIGF:** the standard lists only channels 1, 2, 3 and 5. Wishtory exports a rerun pull as 2 / 3 and adds a `wishtory_gacha_type` field that it reads back on import; other tools ignore the field and see ordinary channels.
- A live fetch also corrects pulls that were imported as the ordinary channel (for example from another tool), because it reads them from the rerun channel.

## Pity rules

`catalog/rules.json` holds what used to be tables in the Rust code: each banner's hard and soft pity and 50/50 odds, the permanent pool of every game (ids and English names), permanent-pool items that were also featured for a window of dates, and Zenless Custom Search. It is signed and published like the other feeds (`rules.envelope.json`, version `rules_version`), so a game change reaches users as a data update instead of an app release.

- Edit it by hand in a PR. The app validates the whole file and ignores one that is malformed; unknown fields are rejected, so a new field needs an app release first.
- Soft pity values are community estimates; hard pity values are the in-game guarantees. The Chronicled Wish numbers follow secondary sources and are the least certain.
- A featured window (`from` / `to`, server-local dates, inclusive) makes a pull of that item inside the window count as featured, never as a lost 50/50.
- `custom_search.from` is the server-local time from which Custom Search applies; its `engines` map each W-Engine to its agent.
- Changing a banner's `key` is an app change: keys must match what `pity.rs` derives from the game's gacha types (a test checks this).

## One-time setup

1. In GitHub, open **Settings → Secrets and variables → Actions → New repository secret**.
2. Name the secret `CATALOG_SIGNING_KEY`.
3. Set its value to the complete PEM text from the ignored local file `catalog/catalog-signing-key.pem`.

The public half is checked in at `catalog/catalog-public-key.txt` and embedded into the app for signature verification. Never commit or paste the private key into an issue, PR, or chat. The private file is excluded by `.gitignore`.

## Rollback

If a catalog PR introduced bad metadata, restore the prior item data in a new PR. Merge it to publish a new signed asset: the version is raised automatically, because clients reject lower versions and a rollback must be published as a newer version containing the restored data.

The local scripts are `npm run catalog:test`, `npm run catalog:update`, `npm run banners:track` and `npm run catalog:sign`. `scripts/prepare-catalog.mjs` and `scripts/fetch-published.sh` are run by the workflows only. A local catalog update needs internet access to the upstream sources; the scheduled GitHub Action has that access.
