import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { applyNotes, buildManifest, diffBanners, diffFeeds, diffItems, diffRules, summarize } from './changelog.mjs';

const item = (id, over = {}) => ({ game: 'hk4e', id, name: `Item ${id}`, aliases: [], rarity: 5, kind: 'character', traits: {}, paimon_key: null, ...over });
const catalog = (items, version = 1) => ({ schema_version: 1, catalog_version: version, generated_at: 'x', items });
const banner = (id, over = {}) => ({
  id, game: 'hk4e', pool: 'character', name: `Banner ${id}`, gacha_ids: [], start: '2026-01-01 18:00:00', end: '2026-01-21 14:59:59', featured_5: ['a'], featured_4: [], ...over,
});
const banners = (list, version = 1) => ({ schema_version: 1, banners_version: version, generated_at: 'x', banners: list });
const rule = (over = {}) => ({ key: 'character', label: 'Character Event', hard: 90, soft: 74, odds: 50, ...over });
const gameRules = (over = {}) => ({ banners: [rule()], standard: { character: { ids: ['x'], names: ['X'] }, weapon: { ids: [], names: [] } }, featured_windows: [], ...over });
const rules = (over = {}, version = 1) => ({
  schema_version: 1, rules_version: version, generated_at: 'x', games: { hk4e: gameRules(over), hkrpg: gameRules(), nap: gameRules() },
});
const one = (changes) => {
  assert.equal(changes.length, 1, JSON.stringify(changes));
  return changes[0];
};

test('new items and new banners are info', () => {
  const items = diffItems(catalog([item('a')]), catalog([item('a'), item('b')]));
  assert.equal(one(items).severity, 'info');
  assert.equal(one(items).scope, 'items');
  assert.deepEqual(one(items).ids, ['b']);
  const added = diffBanners(banners([banner('b1')]), banners([banner('b1'), banner('b2')]));
  assert.equal(one(added).severity, 'info');
  assert.deepEqual(one(added).ids, ['b2']);
});

test('an item rarity change is major and a name or trait fix is minor', () => {
  assert.equal(one(diffItems(catalog([item('a')]), catalog([item('a', { rarity: 4 })]))).severity, 'major');
  assert.equal(one(diffItems(catalog([item('a')]), catalog([item('a', { name: 'Renamed' })]))).severity, 'minor');
  assert.equal(one(diffItems(catalog([item('a')]), catalog([item('a', { aliases: ['Other'] })]))).severity, 'minor');
  assert.equal(one(diffItems(catalog([item('a')]), catalog([item('a', { traits: { element: 'Pyro' } })]))).severity, 'minor');
  assert.equal(one(diffItems(catalog([item('a')]), catalog([]))).severity, 'major');
});

test('a banner window move, a featured change, a pool change and a removal are major', () => {
  const base = banners([banner('b1')]);
  const moved = one(diffBanners(base, banners([banner('b1', { start: '2026-01-02 18:00:00' })])));
  assert.equal(moved.severity, 'major');
  assert.match(moved.summary, /dates/);
  assert.deepEqual(moved.ids, ['b1']);
  assert.equal(one(diffBanners(base, banners([banner('b1', { featured_5: ['z'] })]))).severity, 'major');
  assert.equal(one(diffBanners(base, banners([banner('b1', { pool: 'weapon' })]))).severity, 'major');
  const gone = diffBanners(base, banners([banner('b9', { game: 'hkrpg' })]));
  assert.ok(gone.some((c) => c.severity === 'major' && c.game === 'hk4e' && /removed/.test(c.summary)));
  assert.ok(gone.some((c) => c.severity === 'info' && c.game === 'hkrpg'));
});

test('adding a gacha_id to a banner is minor, replacing one is major', () => {
  const before = banners([banner('b1', { gacha_ids: ['1'] })]);
  assert.equal(one(diffBanners(before, banners([banner('b1', { gacha_ids: ['1', '2'] })]))).severity, 'minor');
  assert.equal(one(diffBanners(before, banners([banner('b1', { gacha_ids: ['2'] })]))).severity, 'major');
  assert.equal(one(diffBanners(before, banners([banner('b1', { gacha_ids: ['1'], featured_4: ['q'] })]))).severity, 'minor');
  assert.equal(one(diffBanners(before, banners([banner('b1', { gacha_ids: ['1'], name: 'New name' })]))).severity, 'minor');
});

test('pity and 50/50 rule changes, the permanent pool and featured windows are major', () => {
  const base = rules();
  const hard = one(diffRules(base, rules({ banners: [rule({ hard: 80 })] })));
  assert.equal(hard.severity, 'major');
  assert.match(hard.summary, /hard pity 90 → 80/);
  assert.deepEqual(hard.ids, ['character']);
  assert.equal(one(diffRules(base, rules({ banners: [rule({ odds: 75 })] }))).severity, 'major');
  assert.equal(one(diffRules(base, rules({ standard: { character: { ids: ['x', 'y'], names: ['X', 'Y'] }, weapon: { ids: [], names: [] } } }))).severity, 'major');
  const windows = one(diffRules(base, rules({ featured_windows: [{ id: 'x', name: 'X', from: '2026-01-01', to: '2026-01-20' }] })));
  assert.equal(windows.severity, 'major');
  assert.deepEqual(windows.ids, ['x']);
  assert.equal(one(diffRules(base, rules({ custom_search: { from: '2026-01-01 00:00:00', engines: [] } }))).severity, 'major');
  assert.equal(one(diffRules(base, rules({ banners: [rule({ label: 'Character' })] }))).severity, 'minor');
});

test('identical feeds produce no change log', () => {
  const feeds = { catalog: catalog([item('a')]), banners: banners([banner('b1')]), rules: rules() };
  assert.deepEqual(diffFeeds(feeds, JSON.parse(JSON.stringify(feeds))), []);
  // Version and timestamp are not content.
  assert.deepEqual(diffFeeds(feeds, { ...feeds, catalog: { ...feeds.catalog, catalog_version: 9, generated_at: 'later' } }), []);
});

test('large changes are grouped and the ids list stays bounded', () => {
  const many = Array.from({ length: 150 }, (_, i) => item(`n${i}`));
  const out = diffItems(catalog([]), catalog(many));
  assert.equal(out.length, 1);
  assert.equal(out[0].ids.length, 100);
  assert.match(out[0].summary, /150 new items/);
});

test('entries are ordered major, minor, info and have unique ids', () => {
  const before = { catalog: catalog([item('a')]), banners: banners([banner('b1')]), rules: rules() };
  const after = {
    catalog: catalog([item('a', { name: 'Fixed' }), item('b')]),
    banners: banners([banner('b1', { end: '2026-01-22 14:59:59' })]),
    rules: rules({ banners: [rule({ hard: 80 })] }),
  };
  const changes = diffFeeds(before, after);
  assert.deepEqual(changes.map((c) => c.severity), ['major', 'major', 'minor', 'info']);
  assert.equal(new Set(changes.map((c) => c.id)).size, changes.length);
});

test('curated notes attach text and can override a severity; unknown ids are reported, not fatal', () => {
  const changes = diffItems(catalog([item('a')]), catalog([item('a', { name: 'Fixed' })]));
  const id = changes[0].id;
  const result = applyNotes(changes, { schema_version: 1, notes: [{ id, note: 'Spelling only.', severity: 'info' }, { id: 'items:gone:hk4e', note: 'x' }] });
  assert.equal(result.changes[0].severity, 'info');
  assert.equal(result.changes[0].note, 'Spelling only.');
  assert.deepEqual(result.unused, ['items:gone:hk4e']);
  assert.throws(() => applyNotes(changes, { schema_version: 1, notes: [{ id, severity: 'urgent' }] }));
  assert.throws(() => applyNotes(changes, { schema_version: 2, notes: [] }));
  assert.deepEqual(applyNotes(changes, null).changes, changes);
});

const feedsAt = (c, b, r) => ({ catalog: catalog([item('a')], c), banners: banners([banner('b1')], b), rules: rules({}, r) });

test('the first manifest is published and names the component versions', () => {
  const result = buildManifest({ published: null, publishedFeeds: feedsAt(3, 4, 5), newFeeds: feedsAt(3, 4, 5), now: new Date('2026-01-01T00:00:00Z') });
  assert.equal(result.publish, true);
  assert.equal(result.doc.manifest_version, 1);
  assert.deepEqual(result.doc.components, { catalog: 3, banners: 4, rules: 5 });
  assert.deepEqual(result.doc.changes, []);
});

test('a manifest is only published when a component is newer than the one it names', () => {
  const published = { manifest_version: 7, components: { catalog: 3, banners: 4, rules: 5 }, changes: [] };
  const same = buildManifest({ published, publishedFeeds: feedsAt(3, 4, 5), newFeeds: feedsAt(3, 4, 5) });
  assert.equal(same.publish, false);
  assert.equal(same.doc.manifest_version, 7);
  const next = buildManifest({
    published,
    publishedFeeds: feedsAt(3, 4, 5),
    newFeeds: { ...feedsAt(3, 5, 5), banners: banners([banner('b1', { end: '2026-02-01 00:00:00' })], 5) },
  });
  assert.equal(next.publish, true);
  assert.equal(next.doc.manifest_version, 8);
  assert.deepEqual(next.doc.components, { catalog: 3, banners: 5, rules: 5 });
  assert.equal(next.doc.changes[0].severity, 'major');
});

test('the version never goes down, even when the file in the repository is behind', () => {
  const published = { manifest_version: 40, components: { catalog: 1, banners: 1, rules: 1 }, changes: [] };
  const result = buildManifest({ published, publishedFeeds: feedsAt(1, 1, 1), newFeeds: feedsAt(1, 2, 1), floor: 2 });
  assert.equal(result.doc.manifest_version, 41);
  assert.equal(buildManifest({ published, publishedFeeds: feedsAt(1, 1, 1), newFeeds: feedsAt(1, 2, 1), floor: 90 }).doc.manifest_version, 90);
});

test('components published without a manifest are reported as unlisted changes', () => {
  const published = { manifest_version: 2, components: { catalog: 3, banners: 4, rules: 5 }, changes: [] };
  const result = buildManifest({ published, publishedFeeds: feedsAt(3, 6, 5), newFeeds: feedsAt(3, 6, 5) });
  assert.equal(result.publish, true);
  assert.deepEqual(result.doc.components, { catalog: 3, banners: 6, rules: 5 });
  assert.ok(result.doc.changes.some((c) => c.scope === 'banners' && /without a detailed list/.test(c.summary)));
});

test('force publishes a manifest for unchanged components with the next version', () => {
  const published = { manifest_version: 2, components: { catalog: 3, banners: 4, rules: 5 }, changes: [] };
  const result = buildManifest({ published, publishedFeeds: feedsAt(3, 4, 5), newFeeds: feedsAt(3, 4, 5), force: true });
  assert.equal(result.publish, true);
  assert.equal(result.doc.manifest_version, 3);
});

test('the review summary lists entries by severity', () => {
  assert.match(summarize([]), /No change log entries/);
  const text = summarize(diffFeeds(
    { catalog: catalog([item('a')]), banners: banners([banner('b1')]) },
    { catalog: catalog([item('a'), item('b')]), banners: banners([banner('b1', { end: '2026-01-22 14:59:59' })]) },
  ));
  assert.ok(text.indexOf('**Major**') < text.indexOf('**Info**'));
  assert.match(text, /- The dates of/);
});

function envelope(doc) {
  return JSON.stringify({ payload: Buffer.from(JSON.stringify(doc)).toString('base64'), signature: 'x' });
}

test('the command line reads the new and published feeds and writes the manifest', () => {
  const dir = mkdtempSync(join(tmpdir(), 'changelog-'));
  const newDir = join(dir, 'new');
  const pubDir = join(dir, 'pub');
  mkdirSync(newDir);
  mkdirSync(pubDir);
  const before = feedsAt(3, 4, 5);
  const after = { ...before, banners: banners([banner('b1', { end: '2026-02-01 00:00:00' })], 5) };
  for (const name of ['catalog', 'banners', 'rules']) {
    writeFileSync(join(newDir, `${name}.json`), JSON.stringify(after[name]));
    writeFileSync(join(pubDir, `${name}.envelope.json`), envelope(before[name]));
  }
  writeFileSync(join(pubDir, 'manifest.envelope.json'), envelope({ manifest_version: 6, components: { catalog: 3, banners: 4, rules: 5 }, changes: [] }));
  const out = join(dir, 'manifest.json');
  const summary = join(dir, 'summary.md');
  const run = spawnSync(process.execPath, [fileURLToPath(new URL('./changelog.mjs', import.meta.url)), '--new', newDir, '--published', pubDir, '--out', out, '--summary', summary], { encoding: 'utf8' });
  assert.equal(run.status, 0, run.stderr);
  assert.equal(run.stdout.trim(), 'publish');
  const manifest = JSON.parse(readFileSync(out, 'utf8'));
  assert.equal(manifest.manifest_version, 7);
  assert.equal(manifest.changes[0].severity, 'major');
  assert.match(readFileSync(summary, 'utf8'), /\*\*Major\*\*/);
  // --stamp bundles the version and components without the change log.
  const stamped = spawnSync(process.execPath, [fileURLToPath(new URL('./changelog.mjs', import.meta.url)), '--new', newDir, '--published', pubDir, '--out', out, '--stamp'], { encoding: 'utf8' });
  assert.equal(stamped.status, 0, stamped.stderr);
  assert.deepEqual(JSON.parse(readFileSync(out, 'utf8')).changes, []);
});

test('publishing uploads every component before the manifest', () => {
  const workflow = readFileSync(new URL('../.github/workflows/catalog-release.yml', import.meta.url), 'utf8');
  const components = workflow.indexOf('Publish component feeds');
  const manifest = workflow.indexOf('Publish manifest');
  assert.ok(components > 0 && manifest > components, 'the manifest step must come after the component step');
  const uploadsManifest = workflow.slice(components, manifest);
  assert.ok(!/manifest\.envelope\.json/.test(uploadsManifest.replace(/#.*$/gm, '')), 'the component step must not upload the manifest');
});
