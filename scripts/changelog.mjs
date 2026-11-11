// Builds the change log and the manifest of a catalog release by comparing the feeds that are about
// to be published with the ones published now.
//
//   node scripts/changelog.mjs --new <dir> --published <dir> --out <manifest.json>
//        [--notes catalog/changes.json] [--summary <file.md>] [--force] [--stamp]
//
// <dir> for --new holds `catalog.json`, `banners.json` and `rules.json` (what will be published: the
// prepared file when it changed, otherwise the published one). <dir> for --published holds the
// `<name>.envelope.json` files that `scripts/fetch-published.sh` downloaded (any may be missing).
//
// Prints `publish` and writes <manifest.json> when a manifest has to be published: no manifest is
// published yet, or a component is newer than the one the published manifest names. Prints `skip`
// otherwise. With --stamp the manifest is written either way (as `prepare-catalog.mjs --stamp`
// does for the feeds), for release builds that bundle `catalog/manifest.json`.
//
// Severity rules (the spec's "The change log"), applied here and nowhere else:
//   major  can change how existing pulls are read: a banner removed, its dates, featured 5-stars or
//          pool changed, a pity or 50/50 rule, the permanent pool or a featured window changed, an
//          item removed or its rarity changed
//   minor  a narrow correction: a name, alias, trait, kind or Paimon key fix, `gacha_id`s added to a
//          banner, a banner's name or 4-star list, a banner rule's label
//   info   additions that cannot alter an existing pull: new items and new banners
import { existsSync } from 'node:fs';
import { readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export const SEVERITIES = ['major', 'minor', 'info'];
const RANK = { major: 0, minor: 1, info: 2 };
const GAMES = { hk4e: 'Genshin Impact', hkrpg: 'Honkai: Star Rail', nap: 'Zenless Zone Zero' };
// One entry per item or banner up to this many of a kind per game; beyond it they are grouped.
const GROUP_ABOVE = 40;
const MAX_IDS = 100;
const SUMMARY_MAX = 300;

const gameName = (game) => GAMES[game] ?? game;
const clip = (text) => (text.length <= SUMMARY_MAX ? text : `${text.slice(0, SUMMARY_MAX - 1)}…`);
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const sorted = (list) => [...list].sort();

function entry(scope, kind, game, ids, summary, severity, key = '') {
  const suffix = key ? `:${key}` : '';
  return { id: `${scope}:${kind}:${game}${suffix}`, scope, game, ids: ids.slice(0, MAX_IDS), summary: clip(summary), severity };
}

// Emits one entry per subject, or one grouped entry when there are many of them.
function emit(out, scope, kind, game, subjects, severity, one, many) {
  if (subjects.length === 0) return;
  if (subjects.length > GROUP_ABOVE) {
    out.push(entry(scope, kind, game, subjects.map((s) => s.id), many(subjects.length), severity));
    return;
  }
  for (const subject of subjects) out.push(entry(scope, kind, game, [subject.id], one(subject), severity, subject.id));
}

const byKey = (list, key) => new Map(list.map((x) => [x[key], x]));

export function diffItems(before, after) {
  const out = [];
  for (const game of Object.keys(GAMES)) {
    const oldItems = byKey(before.items.filter((i) => i.game === game), 'id');
    const newItems = byKey(after.items.filter((i) => i.game === game), 'id');
    const label = (i) => `“${i.name}”`;
    const added = [];
    const removed = [];
    const rarity = [];
    const names = [];
    const traits = [];
    for (const [id, item] of newItems) {
      const old = oldItems.get(id);
      if (!old) {
        added.push(item);
        continue;
      }
      if (old.rarity !== item.rarity) rarity.push({ ...item, was: old.rarity });
      if (old.name !== item.name || !same(sorted(old.aliases ?? []), sorted(item.aliases ?? []))) names.push({ ...item, was: old.name });
      if (old.kind !== item.kind || !same(old.traits ?? {}, item.traits ?? {}) || (old.paimon_key ?? null) !== (item.paimon_key ?? null)) {
        traits.push(item);
      }
    }
    for (const [id, item] of oldItems) if (!newItems.has(id)) removed.push(item);
    const g = gameName(game);
    emit(out, 'items', 'removed', game, removed, 'major',
      (i) => `${label(i)} is no longer in the ${g} item list.`,
      (n) => `${n} items are no longer in the ${g} item list.`);
    emit(out, 'items', 'rarity', game, rarity, 'major',
      (i) => `${label(i)} (${g}) is now a ${i.rarity}-star item instead of ${i.was}-star.`,
      (n) => `The star rating of ${n} ${g} items changed.`);
    emit(out, 'items', 'name', game, names, 'minor',
      (i) => `The name or aliases of ${label(i)} (${g}) were corrected${i.was !== i.name ? `; it was “${i.was}”` : ''}.`,
      (n) => `The names or aliases of ${n} ${g} items were corrected.`);
    emit(out, 'items', 'details', game, traits, 'minor',
      (i) => `The details of ${label(i)} (${g}) were corrected.`,
      (n) => `The details of ${n} ${g} items were corrected.`);
    emit(out, 'items', 'added', game, added, 'info',
      (i) => `New in ${g}: ${label(i)} (${i.rarity}-star).`,
      (n) => `${n} new items were added for ${g}.`);
  }
  return out;
}

const window = (b) => `${b.start} to ${b.end}`;

export function diffBanners(before, after) {
  const out = [];
  for (const game of Object.keys(GAMES)) {
    const oldList = byKey(before.banners.filter((b) => b.game === game), 'id');
    const newList = byKey(after.banners.filter((b) => b.game === game), 'id');
    const g = gameName(game);
    const kinds = { added: [], removed: [], window: [], featured: [], pool: [], gachaIds: [], gachaIdsAdded: [], featured4: [], name: [] };
    for (const [id, banner] of newList) {
      const old = oldList.get(id);
      if (!old) {
        kinds.added.push(banner);
        continue;
      }
      if (old.start !== banner.start || old.end !== banner.end) kinds.window.push({ ...banner, was: old });
      if (!same(sorted(old.featured_5 ?? []), sorted(banner.featured_5 ?? []))) kinds.featured.push({ ...banner, was: old });
      if (old.pool !== banner.pool) kinds.pool.push({ ...banner, was: old });
      const oldIds = old.gacha_ids ?? [];
      const newIds = banner.gacha_ids ?? [];
      if (!same(sorted(oldIds), sorted(newIds))) {
        // Adding `gacha_id`s only teaches the app more; removing or replacing one can move pulls.
        (oldIds.every((x) => newIds.includes(x)) ? kinds.gachaIdsAdded : kinds.gachaIds).push(banner);
      }
      if (!same(sorted(old.featured_4 ?? []), sorted(banner.featured_4 ?? []))) kinds.featured4.push(banner);
      if (old.name !== banner.name) kinds.name.push({ ...banner, was: old });
    }
    for (const [id, banner] of oldList) if (!newList.has(id)) kinds.removed.push(banner);
    const label = (b) => `“${b.name}”`;
    emit(out, 'banners', 'removed', game, kinds.removed, 'major',
      (b) => `The ${g} banner ${label(b)} (${window(b)}) was removed.`,
      (n) => `${n} ${g} banners were removed.`);
    emit(out, 'banners', 'window', game, kinds.window, 'major',
      (b) => `The dates of the ${g} banner ${label(b)} changed from ${window(b.was)} to ${window(b)}.`,
      (n) => `The dates of ${n} ${g} banners changed.`);
    emit(out, 'banners', 'featured', game, kinds.featured, 'major',
      (b) => `The featured 5-star items of the ${g} banner ${label(b)} changed.`,
      (n) => `The featured 5-star items of ${n} ${g} banners changed.`);
    emit(out, 'banners', 'pool', game, kinds.pool, 'major',
      (b) => `The ${g} banner ${label(b)} moved from the ${b.was.pool} pool to the ${b.pool} pool.`,
      (n) => `${n} ${g} banners moved to another pool.`);
    emit(out, 'banners', 'gacha-ids', game, kinds.gachaIds, 'major',
      (b) => `The game's banner ids of the ${g} banner ${label(b)} were replaced.`,
      (n) => `The game's banner ids of ${n} ${g} banners were replaced.`);
    emit(out, 'banners', 'gacha-ids-added', game, kinds.gachaIdsAdded, 'minor',
      (b) => `The ${g} banner ${label(b)} can now be recognised by the game's own banner id.`,
      (n) => `${n} ${g} banners can now be recognised by the game's own banner id.`);
    emit(out, 'banners', 'featured-4', game, kinds.featured4, 'minor',
      (b) => `The featured 4-star items of the ${g} banner ${label(b)} changed.`,
      (n) => `The featured 4-star items of ${n} ${g} banners changed.`);
    emit(out, 'banners', 'name', game, kinds.name, 'minor',
      (b) => `The ${g} banner “${b.was.name}” is now called ${label(b)}.`,
      (n) => `${n} ${g} banners were renamed.`);
    emit(out, 'banners', 'added', game, kinds.added, 'info',
      (b) => `New ${g} banner: ${label(b)} (${window(b)}).`,
      (n) => `${n} new ${g} banners were added.`);
  }
  return out;
}

const windowKey = (w) => `${w.id}|${w.from}|${w.to}`;

export function diffRules(before, after) {
  const out = [];
  for (const game of Object.keys(GAMES)) {
    const old = before.games?.[game];
    const now = after.games?.[game];
    if (!old || !now) continue;
    const g = gameName(game);
    const oldRules = byKey(old.banners, 'key');
    const newRules = byKey(now.banners, 'key');
    for (const [key, rule] of newRules) {
      const was = oldRules.get(key);
      if (!was) {
        out.push(entry('rules', 'banner-added', game, [key], `${g} now has pity rules for the ${rule.label} banner.`, 'major', key));
        continue;
      }
      const limits = ['hard', 'soft', 'odds'].filter((f) => (was[f] ?? null) !== (rule[f] ?? null));
      if (limits.length > 0) {
        const parts = limits.map((f) => `${{ hard: 'hard pity', soft: 'soft pity', odds: 'the featured chance' }[f]} ${was[f] ?? 'none'} → ${rule[f] ?? 'none'}`);
        out.push(entry('rules', 'limits', game, [key], `The ${g} ${rule.label} rules changed: ${parts.join(', ')}.`, 'major', key));
      }
      if (was.label !== rule.label) {
        out.push(entry('rules', 'label', game, [key], `The ${g} banner rules “${was.label}” are now called “${rule.label}”.`, 'minor', key));
      }
    }
    for (const [key, rule] of oldRules) {
      if (!newRules.has(key)) out.push(entry('rules', 'banner-removed', game, [key], `${g} no longer has pity rules for the ${rule.label} banner.`, 'major', key));
    }
    for (const pool of ['character', 'weapon']) {
      const oldIds = new Set(old.standard?.[pool]?.ids ?? []);
      const newIds = new Set(now.standard?.[pool]?.ids ?? []);
      const oldNames = new Set(old.standard?.[pool]?.names ?? []);
      const newNames = new Set(now.standard?.[pool]?.names ?? []);
      const added = [...newIds].filter((x) => !oldIds.has(x));
      const removed = [...oldIds].filter((x) => !newIds.has(x));
      const namesChanged = !same(sorted(oldNames), sorted(newNames));
      if (added.length || removed.length || namesChanged) {
        const detail = [added.length ? `${added.length} added` : '', removed.length ? `${removed.length} removed` : '']
          .filter(Boolean).join(', ') || 'names corrected';
        out.push(entry('rules', `standard-${pool}`, game, [...added, ...removed], `The ${g} permanent ${pool} pool changed (${detail}).`, 'major'));
      }
    }
    const oldWindows = new Map((old.featured_windows ?? []).map((w) => [windowKey(w), w]));
    const newWindows = new Map((now.featured_windows ?? []).map((w) => [windowKey(w), w]));
    const windowsAdded = [...newWindows.keys()].filter((k) => !oldWindows.has(k));
    const windowsRemoved = [...oldWindows.keys()].filter((k) => !newWindows.has(k));
    if (windowsAdded.length || windowsRemoved.length) {
      const ids = [...new Set([...windowsAdded.map((k) => newWindows.get(k).id), ...windowsRemoved.map((k) => oldWindows.get(k).id)])];
      out.push(entry('rules', 'featured-windows', game, ids,
        `The list of permanent-pool items that were also featured for a while changed for ${g} (${windowsAdded.length} added or changed, ${windowsRemoved.length} removed or changed).`, 'major'));
    }
    if (!same(old.custom_search ?? null, now.custom_search ?? null)) {
      out.push(entry('rules', 'custom-search', game, [], `The Custom Search rules of ${g} changed.`, 'major'));
    }
  }
  return out;
}

/** Entries for what changed between two sets of feeds. A feed that is missing on either side is skipped. */
export function diffFeeds(before, after) {
  const changes = [];
  if (before?.catalog && after?.catalog) changes.push(...diffItems(before.catalog, after.catalog));
  if (before?.banners && after?.banners) changes.push(...diffBanners(before.banners, after.banners));
  if (before?.rules && after?.rules) changes.push(...diffRules(before.rules, after.rules));
  return sortChanges(changes);
}

export function sortChanges(changes) {
  return [...changes].sort((a, b) => RANK[a.severity] - RANK[b.severity] || a.id.localeCompare(b.id));
}

/**
 * `catalog/changes.json` is optional: `{ "schema_version": 1, "notes": [{ "id", "note"?, "severity"? }] }`.
 * A note is attached to the generated entry with that id, and a severity replaces the generated one.
 * Notes for ids that are not in this release are ignored (they may describe an earlier one).
 */
export function applyNotes(changes, file) {
  if (!file) return { changes, unused: [] };
  if (file.schema_version !== 1 || !Array.isArray(file.notes)) throw new Error('catalog/changes.json: unsupported shape.');
  const known = new Map(changes.map((c) => [c.id, c]));
  const unused = [];
  for (const note of file.notes) {
    if (typeof note?.id !== 'string') throw new Error('catalog/changes.json: every note needs an id.');
    if (note.severity !== undefined && !SEVERITIES.includes(note.severity)) throw new Error(`catalog/changes.json: unknown severity for ${note.id}.`);
    if (note.note !== undefined && (typeof note.note !== 'string' || note.note.length > 500)) throw new Error(`catalog/changes.json: note for ${note.id} must be text of at most 500 characters.`);
    const target = known.get(note.id);
    if (!target) {
      unused.push(note.id);
      continue;
    }
    if (note.severity) target.severity = note.severity;
    if (note.note) target.note = note.note;
  }
  return { changes: sortChanges(changes), unused };
}

const COMPONENTS = { catalog: 'catalog_version', banners: 'banners_version', rules: 'rules_version' };

export function componentVersions(feeds) {
  return Object.fromEntries(Object.entries(COMPONENTS).map(([name, key]) => [name, feeds[name][key]]));
}

/**
 * Decides whether a manifest has to be published and builds it.
 * `published` is the published manifest document (or null), `publishedFeeds` the published
 * component documents, `newFeeds` what will be published. The manifest names the exact component
 * versions, so the app can tell a complete release from a half published one.
 */
export function buildManifest({ published, publishedFeeds = {}, newFeeds, notes = null, now = new Date(), floor = 1, force = false }) {
  const components = componentVersions(newFeeds);
  const needed = !published
    || force
    || Object.keys(COMPONENTS).some((name) => components[name] !== published.components?.[name]);
  let changes = diffFeeds(publishedFeeds, newFeeds);
  // Components published earlier than the previous manifest named (for example a run that stopped
  // before its manifest was uploaded) changed things nobody was told about.
  if (published) {
    for (const [name, key] of Object.entries(COMPONENTS)) {
      const live = publishedFeeds[name]?.[key];
      if (live !== undefined && live > (published.components?.[name] ?? 0)) {
        const scope = { catalog: 'items', banners: 'banners', rules: 'rules' }[name];
        for (const game of Object.keys(GAMES)) {
          changes.push(entry(scope, 'unlisted', game, [], `Earlier ${gameName(game)} ${name === 'catalog' ? 'item' : name === 'banners' ? 'banner' : 'rule'} changes were published without a detailed list.`, 'minor'));
        }
      }
    }
  }
  const { changes: finalChanges, unused } = applyNotes(changes, notes);
  const version = Math.max(floor, (published?.manifest_version ?? 0) + 1);
  const doc = {
    schema_version: 1,
    manifest_version: needed ? version : (published?.manifest_version ?? floor),
    generated_at: now.toISOString(),
    components,
    changes: needed ? finalChanges : [],
  };
  return { publish: needed, doc, unused };
}

/** The review PR description: what the people who update will be told. */
export function summarize(changes) {
  if (changes.length === 0) return 'No change log entries (the feeds are unchanged, or this is the first manifest).\n';
  const lines = [];
  for (const severity of SEVERITIES) {
    const group = changes.filter((c) => c.severity === severity);
    if (group.length === 0) continue;
    lines.push(`**${severity[0].toUpperCase()}${severity.slice(1)}** (${group.length})`, '');
    for (const c of group) lines.push(`- ${c.summary}${c.note ? ` _${c.note}_` : ''}`);
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

export async function readEnvelopePayload(path) {
  const envelope = JSON.parse(await readFile(path, 'utf8'));
  return JSON.parse(Buffer.from(envelope.payload, 'base64').toString('utf8'));
}

async function readIfPresent(path, read) {
  return existsSync(path) ? read(path) : null;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const args = process.argv.slice(2);
  const option = (name) => {
    const at = args.indexOf(`--${name}`);
    return at >= 0 ? args[at + 1] : undefined;
  };
  const newDir = option('new');
  const publishedDir = option('published');
  const out = option('out');
  if (!newDir || !publishedDir || !out) {
    throw new Error('Usage: changelog.mjs --new <dir> --published <dir> --out <manifest.json> [--notes <changes.json>] [--summary <file.md>] [--force] [--stamp]');
  }
  const readJson = async (path) => JSON.parse(await readFile(path, 'utf8'));
  const newFeeds = {};
  const publishedFeeds = {};
  for (const name of Object.keys(COMPONENTS)) {
    newFeeds[name] = await readJson(join(newDir, `${name}.json`));
    const pub = await readIfPresent(join(publishedDir, `${name}.envelope.json`), readEnvelopePayload);
    if (pub) publishedFeeds[name] = pub;
  }
  const published = await readIfPresent(join(publishedDir, 'manifest.envelope.json'), readEnvelopePayload);
  const notesPath = option('notes');
  const notes = notesPath ? await readIfPresent(notesPath, readJson) : null;
  const floorFile = await readIfPresent(join(newDir, 'manifest.json'), readJson);
  const result = buildManifest({
    published,
    publishedFeeds,
    newFeeds,
    notes,
    floor: floorFile?.manifest_version ?? 1,
    force: args.includes('--force'),
  });
  for (const id of result.unused) console.error(`catalog/changes.json: note for ${id} matches nothing in this release.`);
  if (result.publish || args.includes('--stamp')) {
    const doc = args.includes('--stamp') ? { ...result.doc, changes: [] } : result.doc;
    await writeFile(out, `${JSON.stringify(doc)}\n`);
  }
  const summaryPath = option('summary');
  if (summaryPath) await writeFile(summaryPath, summarize(result.publish ? result.doc.changes : []));
  console.log(result.publish ? 'publish' : 'skip');
}
