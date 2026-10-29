import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { prepare, readEnvelope, versionKey } from './prepare-catalog.mjs';

const banners = (version, name = 'A') => ({ schema_version: 1, banners_version: version, generated_at: 'old', banners: [{ id: 'x', name }] });
const items = (version, name = 'A') => ({ schema_version: 1, catalog_version: version, generated_at: 'old', items: [{ id: '1', name }] });
const rules = (version, hard = 90) => ({ schema_version: 1, rules_version: version, generated_at: 'old', games: { hk4e: { banners: [{ key: 'character', hard }] } } });
const now = new Date('2026-01-01T00:00:00Z');

test('recognises every kind and rejects anything else', () => {
  assert.equal(versionKey(banners(1)), 'banners_version');
  assert.equal(versionKey(items(1)), 'catalog_version');
  assert.equal(versionKey(rules(1)), 'rules_version');
  assert.equal(versionKey(JSON.parse(readFileSync(new URL('../catalog/rules.json', import.meta.url), 'utf8'))), 'rules_version');
  assert.throws(() => versionKey({ schema_version: 1 }));
});

test('publishes the file as it is when nothing is published yet', () => {
  const doc = banners(5);
  assert.deepEqual(prepare(doc, null, { now }), { publish: true, doc });
});

test('skips when only the version or timestamp differ', () => {
  const result = prepare(banners(9), { ...banners(4), generated_at: 'other' }, { now });
  assert.equal(result.publish, false);
});

test('a changed file gets one more than the published version, whatever its own number says', () => {
  const result = prepare(banners(3, 'B'), banners(7, 'A'), { now });
  assert.equal(result.publish, true);
  assert.equal(result.doc.banners_version, 8);
  assert.equal(result.doc.generated_at, now.toISOString());
});

test('keeps the file version when it is already ahead of the published one', () => {
  assert.equal(prepare(items(12, 'B'), items(7, 'A'), { now }).doc.catalog_version, 12);
});

test('force publishes unchanged content with a new version', () => {
  const result = prepare(items(7), items(7), { now, force: true });
  assert.equal(result.publish, true);
  assert.equal(result.doc.catalog_version, 8);
});

test('refuses to compare different kinds of catalog', () => {
  assert.throws(() => prepare(banners(1), items(1)));
});

test('reads a signed envelope payload', () => {
  const doc = banners(2);
  const envelope = JSON.stringify({ payload: Buffer.from(JSON.stringify(doc)).toString('base64'), signature: 'x' });
  assert.deepEqual(readEnvelope(envelope), doc);
});

test('a changed rule set is versioned like the other feeds', () => {
  const result = prepare(rules(1, 80), rules(4, 90), { now });
  assert.equal(result.publish, true);
  assert.equal(result.doc.rules_version, 5);
  assert.equal(prepare(rules(9, 90), rules(4, 90), { now }).publish, false);
});

function cli(args, files) {
  const dir = mkdtempSync(join(tmpdir(), 'prepare-'));
  const paths = {};
  for (const [name, content] of Object.entries(files)) {
    paths[name] = join(dir, name);
    writeFileSync(paths[name], content);
  }
  const run = spawnSync(process.execPath, [fileURLToPath(new URL('./prepare-catalog.mjs', import.meta.url)), ...args(paths)], { encoding: 'utf8' });
  return { run, read: (name) => readFileSync(join(dir, name), 'utf8') };
}

test('--stamp writes the published version into an unchanged bundled file', () => {
  const envelope = JSON.stringify({ payload: Buffer.from(JSON.stringify(items(7))).toString('base64'), signature: 'x' });
  const { run, read } = cli((p) => [p['in.json'], p['in.json'], p['pub.json'], '--stamp'], {
    'in.json': JSON.stringify(items(2)),
    'pub.json': envelope,
  });
  assert.equal(run.status, 0, run.stderr);
  assert.equal(run.stdout.trim(), 'skip');
  assert.equal(JSON.parse(read('in.json')).catalog_version, 7);
});

test('--stamp gives a not yet published change the version CI will publish', () => {
  const envelope = JSON.stringify({ payload: Buffer.from(JSON.stringify(items(7, 'A'))).toString('base64'), signature: 'x' });
  const { run, read } = cli((p) => [p['in.json'], p['in.json'], p['pub.json'], '--stamp'], {
    'in.json': JSON.stringify(items(2, 'B')),
    'pub.json': envelope,
  });
  assert.equal(run.stdout.trim(), 'publish');
  assert.equal(JSON.parse(read('in.json')).catalog_version, 8);
});

test('without --stamp an unchanged file is not written', () => {
  const envelope = JSON.stringify({ payload: Buffer.from(JSON.stringify(items(7))).toString('base64'), signature: 'x' });
  const { run, read } = cli((p) => [p['in.json'], p['out.json'], p['pub.json']], {
    'in.json': JSON.stringify(items(2)),
    'out.json': 'untouched',
    'pub.json': envelope,
  });
  assert.equal(run.stdout.trim(), 'skip');
  assert.equal(read('out.json'), 'untouched');
});
