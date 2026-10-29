// Decides whether a catalog file needs publishing and sets its version, so nobody bumps versions by hand.
//
//   node scripts/prepare-catalog.mjs <feed.json> <out.json> [published.envelope.json] [--force] [--stamp]
//
// Prints `publish` and writes <out.json> when the content differs from what is published now (the
// version becomes one more than the published one, if the file's own version is not already higher),
// or `skip` when nothing changed. Works for the item catalog (catalog_version), the banner list
// (banners_version) and the pity rules (rules_version).
//
// With --stamp <out.json> is written either way: the document as it is (or will be) published, so
// a release build can bundle files whose version numbers match the signed feed. The app prefers a
// cached feed over the bundled one when its version is not lower, so a bundled file that kept an
// old version number would lose to older cached content after an upgrade.
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function versionKey(doc) {
  if (Number.isSafeInteger(doc.catalog_version) && Array.isArray(doc.items)) return 'catalog_version';
  if (Number.isSafeInteger(doc.banners_version) && Array.isArray(doc.banners)) return 'banners_version';
  if (Number.isSafeInteger(doc.rules_version) && doc.games && typeof doc.games === 'object') return 'rules_version';
  throw new Error('This is not an item catalog, a banner list or a rule set.');
}

// Everything except the version and the timestamp, which are the only things this script sets.
const content = (doc, key) => JSON.stringify({ ...doc, [key]: 0, generated_at: '' });

export function prepare(doc, published, { now = new Date(), force = false } = {}) {
  const key = versionKey(doc);
  if (!published) return { publish: true, doc };
  if (versionKey(published) !== key) throw new Error('The published file is a different kind of catalog.');
  if (!force && content(doc, key) === content(published, key)) return { publish: false, doc: published };
  return {
    publish: true,
    doc: { ...doc, [key]: Math.max(doc[key], published[key] + 1), generated_at: now.toISOString() },
  };
}

export function readEnvelope(text) {
  const envelope = JSON.parse(text);
  return JSON.parse(Buffer.from(envelope.payload, 'base64').toString('utf8'));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const args = process.argv.slice(2);
  const force = args.includes('--force');
  const stamp = args.includes('--stamp');
  const [input, output, envelopePath] = args.filter((a) => !a.startsWith('--'));
  if (!input || !output) throw new Error('Usage: prepare-catalog.mjs <feed.json> <out.json> [published.envelope.json] [--force] [--stamp]');
  const doc = JSON.parse(await readFile(input, 'utf8'));
  const published = envelopePath ? readEnvelope(await readFile(envelopePath, 'utf8')) : null;
  const result = prepare(doc, published, { force });
  if (result.publish || stamp) await writeFile(output, `${JSON.stringify(result.doc)}\n`);
  console.log(result.publish ? 'publish' : 'skip');
}
