import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const BASE = 'https://raw.githubusercontent.com/EnkaNetwork/API-docs/master/store';
const PAIMON = 'https://api.github.com/repos/MadeBaruna/paimon-moe/contents';
const MINIMUM_ITEMS_PER_GAME = { hk4e: 250, hkrpg: 100, nap: 60 };
const URLS = {
  giAvatars: `${BASE}/gi/avatars.json`, giWeapons: `${BASE}/gi/weapons.json`, giLocs: `${BASE}/gi/locs.json`,
  hsrAvatars: `${BASE}/hsr/avatars.json`, hsrWeapons: `${BASE}/hsr/weapons.json`, hsrLocs: `${BASE}/hsr/hsr.json`,
  zzzAvatars: `${BASE}/zzz/avatars.json`, zzzWeapons: `${BASE}/zzz/weapons.json`, zzzLocs: `${BASE}/zzz/locs.json`
};

const normalize = (s) => String(s ?? '').normalize('NFKD').replace(/[’']/g, '').replace(/[^a-z\d]+/gi, ' ').trim().toLowerCase();
const slug = (s) => normalize(s).replace(/\s+/g, '_');
const mapElement = (value, game) => {
  const values = Array.isArray(value) ? value : [value];
  if (!values.some(v => v && String(v).toLowerCase() !== 'none')) return undefined;
  const games = {
    hk4e: { Ice: 'cryo', Fire: 'pyro', Wind: 'anemo', Lightning: 'electro', Electric: 'electro', Electro: 'electro', Water: 'hydro', Rock: 'geo', Grass: 'dendro' },
    hkrpg: { Ice: 'ice', Fire: 'fire', Wind: 'wind', Lightning: 'lightning', Thunder: 'lightning', Physical: 'physical', Imaginary: 'imaginary', Quantum: 'quantum' },
    nap: { Ice: 'ice', Fire: 'fire', Elec: 'electric', Electric: 'electric', Thunder: 'electric', Ether: 'ether', Physical: 'physical', Physics: 'physical', Wind: 'wind', Lumen: 'lumen' }
  };
  const mappings = games[game] ?? {};
  const mapped = values.filter(v => v && String(v).toLowerCase() !== 'none').map(v => mappings[v] ?? String(v).toLowerCase());
  const accepted = new Set(Object.values(mappings));
  return mapped.find(v => accepted.has(v)) ?? mapped[0];
};
const mapPath = (value) => ({ Warrior: 'destruction', Rogue: 'hunt', Mage: 'erudition', Shaman: 'harmony', Warlock: 'nihility', Knight: 'preservation', Priest: 'abundance', Memory: 'remembrance', Elation: 'elation' }[value]);
const mapSpecialty = (value) => ({ Attack: 'attack', Stun: 'stun', Support: 'support', Anomaly: 'anomaly', Defense: 'defense', Rupture: 'rupture', Armorer: 'armorer' }[value]);

export function buildItems(source, oldItems = [], minimumCounts = MINIMUM_ITEMS_PER_GAME) {
  const oldById = new Map(oldItems.map(item => [`${item.game}:${item.id}`, item]));
  const items = [];
  const add = (item) => {
    const old = oldById.get(`${item.game}:${item.id}`);
    if (old?.paimon_key && !item.paimon_key) item.paimon_key = old.paimon_key;
    items.push({ aliases: [], traits: {}, paimon_key: null, ...item });
  };

  const giNames = source.giLocs.en;
  const charSlugs = new Set(source.paimonCharacters);
  const weaponMap = new Map(Object.values(source.paimonWeapons).map(w => [normalize(w.name), w.id]));
  for (const [id, row] of Object.entries(source.giAvatars)) {
    const rarity = row.QualityType === 'QUALITY_ORANGE' ? 5 : row.QualityType === 'QUALITY_PURPLE' ? 4 : 0;
    if (!rarity) continue;
    const baseName = giNames[String(row.NameTextMapHash)];
    if (!baseName) continue;
    const element = mapElement(row.Element, 'hk4e');
    if (!element) continue;
    const name = baseName === 'Traveler' && element ? `Traveler (${element[0].toUpperCase()}${element.slice(1)})` : baseName;
    const key = baseName === 'Traveler' ? `traveler_${element}` : slug(baseName);
    add({ game: 'hk4e', id, name, rarity, kind: 'character', traits: element ? { element } : {}, paimon_key: charSlugs.has(key) ? key : null });
  }
  for (const [id, row] of Object.entries(source.giWeapons)) {
    if (![3, 4, 5].includes(row.Rarity)) continue;
    const name = giNames[String(row.NameTextMapHash)];
    if (!name) continue;
    add({ game: 'hk4e', id, name, rarity: row.Rarity, kind: 'weapon', paimon_key: weaponMap.get(normalize(name)) ?? null });
  }

  const hsrLocs = source.hsrLocs.en;
  for (const [id, row] of Object.entries(source.hsrAvatars)) {
    if (![4, 5].includes(row.Rarity)) continue;
    const name = hsrLocs[row.AvatarName?.Hash];
    if (!name) continue;
    const traits = { element: mapElement(row.Element, 'hkrpg') };
    const path = mapPath(row.AvatarBaseType); if (path) traits.path = path;
    add({ game: 'hkrpg', id, name, rarity: row.Rarity, kind: 'character', traits });
  }
  for (const [id, row] of Object.entries(source.hsrWeapons)) {
    if (![3, 4, 5].includes(row.Rarity)) continue;
    const name = hsrLocs[row.EquipmentName?.Hash];
    if (!name) continue;
    add({ game: 'hkrpg', id, name, rarity: row.Rarity, kind: 'light_cone' });
  }

  const zzzLocs = source.zzzLocs.en;
  for (const [id, row] of Object.entries(source.zzzAvatars)) {
    if (![3, 4].includes(row.Rarity)) continue;
    const name = zzzLocs[row.Name];
    if (!name) continue;
    const element = mapElement(row.ElementTypes, 'nap');
    const specialty = mapSpecialty(row.ProfessionType);
    add({ game: 'nap', id, name, rarity: row.Rarity + 1, kind: 'character', traits: { ...(element ? { element } : {}), ...(specialty ? { specialty } : {}) } });
  }
  for (const [id, row] of Object.entries(source.zzzWeapons)) {
    if (![2, 3, 4].includes(row.Rarity)) continue;
    const name = zzzLocs[row.ItemName];
    if (!name) continue;
    add({ game: 'nap', id, name, rarity: row.Rarity + 1, kind: 'weapon', traits: { ...(mapSpecialty(row.ProfessionType) ? { specialty: mapSpecialty(row.ProfessionType) } : {}) } });
  }

  const seen = new Set();
  for (const item of items) {
    const key = `${item.game}:${item.id}`;
    if (!item.name || !item.id || seen.has(key)) throw new Error(`Invalid or duplicate catalog item: ${key}`);
    seen.add(key);
    const allowed = {
      hk4e: { element: ['anemo', 'cryo', 'dendro', 'electro', 'geo', 'hydro', 'pyro'] },
      hkrpg: { element: ['physical', 'fire', 'ice', 'lightning', 'wind', 'quantum', 'imaginary'], path: ['destruction', 'hunt', 'erudition', 'harmony', 'nihility', 'preservation', 'abundance', 'remembrance', 'elation'] },
      nap: { element: ['physical', 'ice', 'fire', 'electric', 'ether', 'wind', 'lumen'], specialty: ['attack', 'stun', 'support', 'anomaly', 'defense', 'rupture', 'armorer'] }
    }[item.game];
    for (const [key, value] of Object.entries(item.traits)) {
      if (!allowed[key]?.includes(value)) throw new Error(`Unsupported ${item.game} trait ${key}=${value}`);
    }
    if (item.game === 'hk4e' && item.kind === 'character' && !item.traits.element) throw new Error(`Genshin character missing element: ${item.name}`);
    if (item.game === 'hkrpg' && item.kind === 'character' && (!item.traits.element || !item.traits.path)) throw new Error(`Star Rail character missing traits: ${item.name}`);
    if (item.game === 'nap' && item.kind === 'character' && (!item.traits.element || !item.traits.specialty)) throw new Error(`Zenless agent missing traits: ${item.name}`);
  }
  const counts = Object.groupBy ? Object.groupBy(items, item => item.game) : items.reduce((o, item) => ((o[item.game] ??= []).push(item), o), {});
  for (const game of ['hk4e', 'hkrpg', 'nap']) {
    if ((counts[game]?.length ?? 0) < minimumCounts[game]) throw new Error(`Upstream data is incomplete for ${game}`);
  }
  for (const old of oldItems) if (!seen.has(`${old.game}:${old.id}`)) items.push(old);
  return items.sort((a, b) => a.game.localeCompare(b.game) || a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name) || a.id.localeCompare(b.id));
}

async function getJSON(url) {
  const response = await fetch(url, { headers: { 'user-agent': 'Wishtory catalog maintenance' } });
  if (!response.ok) throw new Error(`Upstream request failed (${response.status}): ${url}`);
  return response.json();
}

export async function loadSources() {
  const keys = Object.keys(URLS);
  const values = await Promise.all(keys.map(k => getJSON(URLS[k])));
  const out = Object.fromEntries(keys.map((k, i) => [k, values[i]]));
  const [characters, weapons] = await Promise.all([
    getJSON(`${PAIMON}/src/data/characterData`),
    getJSON('https://raw.githubusercontent.com/MadeBaruna/paimon-moe/main/src/data/weapons/en.json')
  ]);
  out.paimonCharacters = characters.filter(x => x.type === 'file' && x.name.endsWith('.json')).map(x => x.name.slice(0, -5));
  out.paimonWeapons = weapons;
  return out;
}

export async function updateCatalog(path = 'catalog/catalog.json', sources = null) {
  const old = JSON.parse(await readFile(path, 'utf8'));
  const items = buildItems(sources ?? await loadSources(), old.items);
  const oldComparable = JSON.stringify(old.items);
  if (oldComparable === JSON.stringify(items)) return false;
  const next = { schema_version: 1, catalog_version: old.catalog_version + 1, generated_at: new Date().toISOString(), items };
  await writeFile(path, `${JSON.stringify(next)}\n`);
  return true;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const changed = await updateCatalog();
  console.log(changed ? 'Catalog data changed.' : 'Catalog is current.');
}
