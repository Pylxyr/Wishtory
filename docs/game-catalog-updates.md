# Game catalog updates

Wishtory separates metadata updates from app releases. The desktop app requests only a signed catalog asset from this repository's `catalog-v1` GitHub Release. It never sends a UID, pull, game cache path, or other account data with that request.

## Sources and review

- [EnkaNetwork processed stores](https://github.com/EnkaNetwork/API-docs/tree/master/store) provide stable item IDs, rarity, English names, localization hashes, and game traits for Genshin Impact, Honkai: Star Rail, and Zenless Zone Zero.
- [Paimon.moe's published item data](https://github.com/MadeBaruna/paimon-moe/tree/main/src/data) provides exact native backup keys for Genshin items.

The **Review game catalog updates** workflow runs each Monday and can also be run manually from GitHub Actions after a game patch. It builds a candidate, checks required metadata and minimum per-game counts, retains known rows omitted by an upstream refresh, and creates or updates a PR. Review the diff before merging. The action never edits wish records or pity rules.

After merge, **Publish signed game catalog** signs the catalog using the repository secret and updates the `catalog-v1` release asset. The app checks once per day when online and keeps its last valid cache when the feed is unavailable or invalid.

## One-time setup

1. In GitHub, open **Settings → Secrets and variables → Actions → New repository secret**.
2. Name the secret `CATALOG_SIGNING_KEY`.
3. Set its value to the complete PEM text from the ignored local file `catalog/catalog-signing-key.pem`.

The public half is checked in at `catalog/catalog-public-key.txt` and embedded into the app for signature verification. Never commit or paste the private key into an issue, PR, or chat. The private file is excluded by `.gitignore`.

## Rollback

If a catalog PR introduced bad metadata, restore the prior item data in a new PR and increment `catalog_version` above the currently published value. Merge it to publish a new signed asset; clients reject lower versions so a rollback must be published as a newer version containing the restored data.

The local scripts are `npm run catalog:test`, `npm run catalog:update`, and `npm run catalog:sign`. A local catalog update needs internet access to the upstream sources; the scheduled GitHub Action has that access.
