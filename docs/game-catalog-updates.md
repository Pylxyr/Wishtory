# Game catalog updates

Wishtory separates metadata updates from app releases. The desktop app requests only a signed catalog asset from this repository's `catalog-v1` GitHub Release. It never sends a UID, pull, game cache path, or other account data with that request.

## Sources and review

- [EnkaNetwork processed stores](https://github.com/EnkaNetwork/API-docs/tree/master/store) provide stable item IDs, rarity, English names, localization hashes, and game traits for Genshin Impact, Honkai: Star Rail, and Zenless Zone Zero. The catalog updater normalizes source-specific values into Wishtory's game-specific traits.
- [Paimon.moe's published item data](https://github.com/MadeBaruna/paimon-moe/tree/main/src/data) provides exact native backup keys for Genshin items. Wishtory uses those keys to import and export native backups without guessing item names.
- [HoYoverse's official Honkai: Star Rail announcement](https://www.hoyolab.com/article/43681649) and [Zenless Zone Zero update announcement](https://zenless.hoyoverse.com/en-us/news/166000) help verify newly introduced paths, attributes, and specialties when upstream data changes. They are references for checking game terminology; the updater reads the machine-readable EnkaNetwork and Paimon.moe sources above.

These projects and their data remain theirs; Wishtory is an independent, non-affiliated client. Catalog records are factual game metadata, and the source links above are provided for attribution and review.

The **Review game catalog updates** workflow runs each Monday and can also be run manually from GitHub Actions after a game patch. It builds a candidate, checks required metadata and minimum per-game counts, retains known rows omitted by an upstream refresh, and creates or updates a PR. Review the diff before merging. The action never edits wish records or pity rules.

After merge, **Publish signed game catalog** signs the catalog and the banner list using the repository secret and updates the `catalog-v1` release assets. The app checks once per day when online and keeps its last valid cache when the feed is unavailable or invalid.

## Banner list

`catalog/banners.json` lists banner events: which 5★ items each one featured and when it ran. It is signed with the same key and published as the `banners.json` asset of the `catalog-v1` release. The app checks it once per day and when you press **Update banners**, and keeps the last valid copy when the feed is unavailable or invalid.

- `start` and `end` are inclusive **server-local** clock times (`YYYY-MM-DD HH:MM:SS`), the same clock as the `time` of each wish record, so they are compared with it directly. Genshin Impact Phase II starts at `18:00:00` server time and the previous phase ends at `17:59:59`. Phase II ends at `14:59:59` on the day before the next version, and that version's Phase I is listed from `15:00:00` the same day, so windows never overlap. The real start is the update time (11:00 UTC+8, a different clock time per server), and event wishes cannot be pulled before it.
- `pool` is the banner key used by the pity rules: `character` or `weapon` for every game, plus `collab` and `collab_weapon` for Honkai: Star Rail.
- `featured_5` holds the game item IDs of every 5★ featured in that event. A Genshin character event with two concurrent banners lists both characters. `gacha_ids` holds the `gacha_id` values the game reports for the banner (Honkai: Star Rail and Zenless Zone Zero) and may be empty: the app then learns which `gacha_id` belongs to which banner from the 5★ pulls that a listed banner features. Where several banners run at once, list one event per phase with every featured 5★ of that phase.
- Add new events in a PR and raise `banners_version` above the published value. Clients ignore lower or equal versions, so a correction is published as a newer version. Every `featured_5` ID must exist in `catalog/catalog.json` with rarity 5; a test checks this.

## One-time setup

1. In GitHub, open **Settings → Secrets and variables → Actions → New repository secret**.
2. Name the secret `CATALOG_SIGNING_KEY`.
3. Set its value to the complete PEM text from the ignored local file `catalog/catalog-signing-key.pem`.

The public half is checked in at `catalog/catalog-public-key.txt` and embedded into the app for signature verification. Never commit or paste the private key into an issue, PR, or chat. The private file is excluded by `.gitignore`.

## Rollback

If a catalog PR introduced bad metadata, restore the prior item data in a new PR and increment `catalog_version` above the currently published value. Merge it to publish a new signed asset; clients reject lower versions so a rollback must be published as a newer version containing the restored data.

The local scripts are `npm run catalog:test`, `npm run catalog:update`, and `npm run catalog:sign`. A local catalog update needs internet access to the upstream sources; the scheduled GitHub Action has that access.
