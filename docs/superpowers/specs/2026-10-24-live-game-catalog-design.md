# Live game catalog design

## Intent

Let Wishtory recognize newly released characters and weapons without requiring a full app installer for every game patch. Keep pull history local and preserve readable records when metadata is missing.

## Goals

- Maintain one versioned, data-only catalog for Genshin Impact, Honkai: Star Rail, and Zenless Zone Zero.
- Check a Wishtory-controlled HTTPS catalog feed in the background at most once per day; send no account or pull data.
- Verify catalog signatures and schema, limit response sizes, install updates atomically, and keep the bundled/last-known-good catalog on any failure.
- Use the catalog for traits and Paimon.moe item keys; keep raw pull names and rarities as the display/export fallback.
- Add a scheduled source-sync that validates upstream community datasets and opens a reviewable catalog update rather than trusting third-party data directly at runtime.

## Non-goals

- Automatically change pity rules or banner type interpretation from downloaded data. Mechanics changes remain tested app releases.
- Silently guess Paimon.moe keys or rarity for unknown native-backup rows.
- Download images or executable code in the metadata feed.

## Approaches

- Bundle-only generated data: simple and offline, but each item update needs an app release.
- Fetch third-party catalogs directly in the app: fast to build, but app behavior depends on upstream availability, schemas, and security.
- Wishtory-owned signed catalog with a scheduled upstream review workflow: independent data releases, offline fallback, and controlled ingestion. Selected by the user’s request to update smoothly without compromising local data.

## Design

Use EnkaNetwork's public processed stores as update candidates for all three games and Paimon.moe's published item keys for exact native-backup compatibility. A scheduled GitHub Action runs a deterministic generator, rejects incomplete or malformed data, retains previously known entries, and opens a PR for catalog changes. After a catalog PR is merged, a separate workflow signs and publishes one JSON envelope as the rolling data asset. The app downloads only Wishtory's asset, verifies its pinned Ed25519 signature and schema, and atomically caches the envelope under its catalog version. Startup uses the highest valid cached catalog immediately, then checks the feed asynchronously once daily. Unknown pulls continue to display the original API name/rank/type. Catalog lookups never alter pull identity or mechanics.

The signing private key belongs only in the repository's GitHub Actions secret; the verification public key is stored in the repository and embedded in the app. The local generated private-key file is ignored by Git. Until the repository secret is configured and the first candidate PR is reviewed and merged, the bundled catalog and legacy item mappings remain usable.

## Constraints and validation

- Existing SQLite records, import/export formats, and mechanics remain compatible.
- No third-party catalog request occurs at runtime; only a versioned Wishtory feed is contacted.
- Catalog schema is versioned; unknown schema versions, invalid signatures, oversized responses, duplicate IDs, invalid games/traits, and stale versions are rejected.
- Rust tests cover validation, signature rejection, offline cache behavior, cache-write recovery, and new Paimon mappings; generator tests cover all three upstream shapes, known-key preservation, and malformed/incomplete data.

## Remaining implementation calls

- Keep the first release data limited to fields already useful in the UI and Paimon conversion: game, stable item ID, English name/aliases, rarity, item type, and game-specific trait.
- Use HTTPS on GitHub Releases as the Wishtory-controlled asset host; pin owner/repository in app code.
- Keep updates quiet and non-blocking, with no account identifiers in request URLs, headers, or telemetry.
