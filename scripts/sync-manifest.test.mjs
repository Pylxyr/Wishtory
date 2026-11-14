import test from 'node:test';
import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { syncComponents, syncManifest } from './sync-manifest.mjs';

const manifest = { schema_version: 1, manifest_version: 3, generated_at: 'then', components: { catalog: 4, banners: 5, rules: 1 }, changes: [{ id: 'a' }] };
const feeds = (catalog, banners, rules) => ({
  catalog: { catalog_version: catalog },
  banners: { banners_version: banners },
  rules: { rules_version: rules }
});

test('sets only the component versions', () => {
  const next = syncComponents(manifest, feeds(5, 5, 2));
  assert.deepEqual(next.components, { catalog: 5, banners: 5, rules: 2 });
  assert.equal(next.manifest_version, 3);
  assert.equal(next.generated_at, 'then');
  assert.deepEqual(next.changes, [{ id: 'a' }]);
});

test('refuses a feed without a valid version', () => {
  assert.throws(() => syncComponents(manifest, feeds(undefined, 5, 1)), /catalog\.json/);
  assert.throws(() => syncComponents(manifest, feeds(4, 0, 1)), /banners\.json/);
});

test('the checked-in manifest already matches the checked-in feeds', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'sync-'));
  cpSync(new URL('../catalog', import.meta.url), dir, { recursive: true });
  assert.equal(await syncManifest(dir), false);
});

test('rewrites the manifest file when a feed moved on, and then reports nothing to do', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'sync-'));
  cpSync(new URL('../catalog', import.meta.url), dir, { recursive: true });
  const path = join(dir, 'catalog.json');
  const catalog = JSON.parse(readFileSync(path, 'utf8'));
  catalog.catalog_version += 1;
  writeFileSync(path, JSON.stringify(catalog));
  assert.equal(await syncManifest(dir), true);
  assert.equal(JSON.parse(readFileSync(join(dir, 'manifest.json'), 'utf8')).components.catalog, catalog.catalog_version);
  assert.equal(await syncManifest(dir), false);
});
