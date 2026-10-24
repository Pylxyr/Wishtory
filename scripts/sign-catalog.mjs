import { createPrivateKey, createPublicKey, sign } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';

const inputPath = process.argv[2] ?? 'catalog/catalog.json';
const outputPath = process.argv[3] ?? 'catalog/catalog.envelope.json';
const pem = process.env.CATALOG_SIGNING_KEY ?? (process.env.CATALOG_SIGNING_KEY_FILE
  ? await readFile(process.env.CATALOG_SIGNING_KEY_FILE, 'utf8')
  : null);
if (!pem) throw new Error('Set CATALOG_SIGNING_KEY or CATALOG_SIGNING_KEY_FILE.');

const privateKey = createPrivateKey(pem);
const publicKey = createPublicKey(privateKey);
const rawPublicKey = publicKey.export({ format: 'der', type: 'spki' }).subarray(-32).toString('base64');
const pinned = (await readFile('catalog/catalog-public-key.txt', 'utf8')).trim();
if (rawPublicKey !== pinned) throw new Error('Signing key does not match the public key pinned by Wishtory.');

const payload = await readFile(inputPath);
const parsed = JSON.parse(payload);
if (parsed.schema_version !== 1 || !Number.isSafeInteger(parsed.catalog_version) || !Array.isArray(parsed.items)) {
  throw new Error('Catalog schema validation failed.');
}
const signature = sign(null, payload, privateKey).toString('base64');
await writeFile(outputPath, `${JSON.stringify({ payload: payload.toString('base64'), signature })}\n`);
