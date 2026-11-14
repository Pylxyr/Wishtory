// Keeps catalog/manifest.json in step with the bundled feeds.
//
//   node scripts/sync-manifest.mjs [catalog-dir]
//
// The checked-in manifest names the version of each bundled feed (`components`), and a test in
// wishtory-core requires them to match catalog.json, banners.json and rules.json. The catalog update
// raises `catalog_version` by itself, so the weekly workflow runs this after it: without it every
// weekly pull request would fail that test. Only `components` is touched; the manifest version,
// its date and its change list stay as they are (the release workflow sets those).
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const FEEDS = { catalog: 'catalog_version', banners: 'banners_version', rules: 'rules_version' };

/** `manifest` with `components` set to the versions of `feeds` ({ catalog, banners, rules } documents). */
export function syncComponents(manifest, feeds) {
  const components = { ...manifest.components };
  for (const [name, key] of Object.entries(FEEDS)) {
    const version = feeds[name]?.[key];
    if (!Number.isInteger(version) || version < 1) throw new Error(`${name}.json has no valid ${key}.`);
    components[name] = version;
  }
  return { ...manifest, components };
}

export async function syncManifest(dir = 'catalog') {
  const read = async (name) => JSON.parse(await readFile(join(dir, `${name}.json`), 'utf8'));
  const feeds = { catalog: await read('catalog'), banners: await read('banners'), rules: await read('rules') };
  const manifest = await read('manifest');
  const next = syncComponents(manifest, feeds);
  if (JSON.stringify(next) === JSON.stringify(manifest)) return false;
  await writeFile(join(dir, 'manifest.json'), `${JSON.stringify(next)}\n`);
  return true;
}

if (import.meta.main) {
  console.log((await syncManifest(process.argv[2])) ? 'Manifest components updated.' : 'Manifest is current.');
}
