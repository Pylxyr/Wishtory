// Builds app/assets/data/materials.json: which talent books and weapon materials are farmable on which
// weekday in Genshin Impact, and which characters and weapons use them.
//
//   node scripts/update-materials.mjs [--catalog catalog/catalog.json] [--out app/assets/data/materials.json]
//
// Paimon.moe publishes the farm days with every item and lists each character's and weapon's materials.
// Its weapon series for the newer regions (Fontaine to Snezhnaya) carry no days, so those come from
// SERIES_OVERRIDES below, checked against the in-game domain schedule.
// Items are matched to the catalog through `paimon_key` and written by catalog name, which is how the app
// finds their pictures. Anything that cannot be matched is reported, never guessed.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';

const RAW = 'https://raw.githubusercontent.com/MadeBaruna/paimon-moe/main/src/data';
// Dependencies first: a module may only refer to what an earlier one exports.
const SOURCE_FILES = ['elements', 'weapons', 'itemList', 'itemGroup', 'characters', 'weaponList'];
// Far larger than the real files (about 1 MB in all); a runaway download is refused, not parsed.
const MAX_SOURCE_BYTES = 4 * 1024 * 1024;
const MAX_DEPTH = 64;

const DAY_NUMBERS = { monday: 1, tuesday: 2, wednesday: 3, thursday: 4, friday: 5, saturday: 6, sunday: 7 };
const MON_THU = [1, 4];
const TUE_FRI = [2, 5];
const WED_SAT = [3, 6];

/** Weekday names ("monday") to numbers, Monday = 1 ... Sunday = 7, sorted and without repeats. */
export function dayNumbers(names) {
  const out = new Set();
  for (const name of names ?? []) {
    const n = DAY_NUMBERS[String(name).toLowerCase()];
    if (n) out.add(n);
  }
  return [...out].sort((a, b) => a - b);
}

/**
 * Weapon series Paimon.moe has no farm days for, keyed by the id of the series' first item (the one every
 * weapon of the series lists first). Sunday is never listed: every domain opens fully on Sundays.
 */
export const SERIES_OVERRIDES = {
  // Fontaine: Echoes of the Deep Tides
  fragment_of_an_ancient_chord: { name: 'Ancient Chord', title: 'Fragment of an Ancient Chord', days: MON_THU },
  dross_of_pure_sacred_dewdrop: { name: 'Sacred Dewdrop', title: 'Dross of Pure Sacred Dewdrop', days: TUE_FRI },
  broken_goblet_of_the_pristine_sea: { name: 'Pristine Sea', title: 'Broken Goblet of the Pristine Sea', days: WED_SAT },
  // Natlan: Ancient Watchtower
  blazing_sacrificial_hearts_terror: { name: 'Blazing Sacrificial Heart', title: "Blazing Sacrificial Heart's Terror", days: MON_THU },
  delirious_decadence_of_the_sacred_lord: { name: 'Delirious Sacred Lord', title: 'Delirious Decadence of the Sacred Lord', days: TUE_FRI },
  night_winds_mystic_consideration: { name: "Night-Wind's Mystic", title: "Night-Wind's Mystic Consideration", days: WED_SAT },
  // Nod-Krai: Lost Mooncourt
  artful_device_fragment: { name: 'Artful Device', title: 'Artful Device Fragment', days: MON_THU },
  ember_of_long_night_flint: { name: 'Long Night Flint', title: 'Ember of Long Night Flint', days: TUE_FRI },
  sundered_glory_of_the_far_north_scions: { name: 'Far-North Scions', title: 'Sundered Glory of the Far-North Scions', days: WED_SAT },
  // Snezhnaya: Scars of Cursed Obsession
  rise_of_the_pale_star_army: { name: 'Pale Star Army', title: 'Rise of the Pale Star Army', days: MON_THU },
  measured_pour_of_the_cellared_spiritual_nectar: { name: 'Cellared Spiritual Nectar', title: 'Measured Pour of the Cellared Spiritual Nectar', days: TUE_FRI },
  the_frost_emperors_revival: { name: 'Frost Emperor', title: "The Frost Emperor's Revival", days: WED_SAT },
};

const byRarityThenName = (a, b) => b.rarity - a.rarity || a.name.localeCompare(b.name);

/**
 * @param source { characters, weaponList, itemGroup } the Paimon.moe data modules
 * @param catalogItems the Genshin items of catalog.json
 * @returns { materials, problems } the JSON to write and what could not be matched
 */
export function buildMaterials(source, catalogItems) {
  const problems = [];
  const genshin = catalogItems.filter((i) => i.game === 'hk4e');
  const byKey = new Map(genshin.filter((i) => i.paimon_key).map((i) => [`${i.kind}|${i.paimon_key}`, i]));

  // --- talent books: one series per book, users are the characters that level their talents with it.
  const books = new Map();
  for (const character of Object.values(source.characters)) {
    const item = byKey.get(`character|${character.id}`);
    const book = character.material?.book?.[0];
    if (!item) {
      problems.push(`character ${character.id}: not in the catalog`);
      continue;
    }
    const days = dayNumbers(book?.day);
    if (!book || days.length === 0) {
      problems.push(`character ${character.id}: no talent book days`);
      continue;
    }
    if (!books.has(book.id)) {
      books.set(book.id, { id: book.id, name: String(book.name).replace(/^Teachings of /, ''), title: book.name, days, users: [] });
    }
    books.get(book.id).users.push({ name: item.name, rarity: item.rarity });
  }

  // --- weapon materials: a series is named by the first item that has days (or an override).
  const weapons = new Map();
  const seriesOf = (weapon) => {
    for (const step of weapon.ascension ?? []) {
      for (const { item } of step.items ?? []) {
        if (!item?.id) continue;
        const override = SERIES_OVERRIDES[item.id];
        if (override) return { id: item.id, ...override };
        const days = dayNumbers(item.day);
        if (days.length === 0) continue;
        const root = item.parent ?? item.id;
        const group = source.itemGroup[root];
        return { id: root, name: group?.name ?? String(item.name), title: group?.items?.[0]?.name ?? String(item.name), days };
      }
    }
    return null;
  };
  const addWeapon = (series, item) => {
    if (!weapons.has(series.id)) weapons.set(series.id, { id: series.id, name: series.name, title: series.title, days: series.days, users: [] });
    const users = weapons.get(series.id).users;
    if (!users.some((u) => u.name === item.name)) users.push({ name: item.name, rarity: item.rarity });
  };
  for (const weapon of Object.values(source.weaponList)) {
    if (weapon.rarity < 4) continue;
    const item = byKey.get(`weapon|${weapon.id}`);
    if (!item) {
      problems.push(`weapon ${weapon.id}: not in the catalog`);
      continue;
    }
    const series = seriesOf(weapon);
    if (!series) {
      problems.push(`weapon ${weapon.id}: no series with farm days`);
      continue;
    }
    addWeapon(series, item);
  }

  const finish = (map) =>
    [...map.values()]
      .map((s) => ({ ...s, users: s.users.sort(byRarityThenName) }))
      .sort((a, b) => a.days[0] - b.days[0] || a.name.localeCompare(b.name));
  return { materials: { schema: 1, source: 'Paimon.moe', books: finish(books), weapons: finish(weapons) }, problems };
}

/**
 * Reads one of Paimon.moe's data modules WITHOUT running it. These files hold only data:
 * `import { a } from './a';` lines and `export const name = <literal>;` where the literal is made of objects,
 * arrays, strings, numbers, true/false/null and references such as `weapons.claymore` to something an earlier
 * module exported. Anything else (calls, functions, templates, spreads, computed keys, ...) is an error, so a
 * changed or tampered upstream file can alter the data but can never run code on this machine.
 * `scope` holds what earlier modules exported, by name. Returns what this module exports.
 */
export function parseDataModule(source, scope = {}) {
  if (typeof source !== 'string' || source.length > MAX_SOURCE_BYTES) throw new Error('The module is missing or too large.');
  let i = 0;
  const known = new Map(Object.entries(scope));
  const exports = {};
  const fail = (what) => {
    const line = source.slice(0, i).split('\n').length;
    throw new Error(`${what} (line ${line})`);
  };
  const skip = () => {
    for (;;) {
      const c = source[i];
      if (c === ' ' || c === '\t' || c === '\r' || c === '\n' || c === '\uFEFF') i += 1;
      else if (c === '/' && source[i + 1] === '/') {
        while (i < source.length && source[i] !== '\n') i += 1;
      } else if (c === '/' && source[i + 1] === '*') {
        const end = source.indexOf('*/', i + 2);
        if (end < 0) fail('Unterminated comment');
        i = end + 2;
      } else return;
    }
  };
  const take = (text) => {
    skip();
    if (!source.startsWith(text, i)) fail(`Expected "${text}"`);
    i += text.length;
  };
  const peek = (text) => {
    skip();
    return source.startsWith(text, i);
  };
  const identifier = () => {
    skip();
    const match = /^[A-Za-z_$][A-Za-z0-9_$]*/.exec(source.slice(i, i + 200));
    if (!match) fail('Expected a name');
    i += match[0].length;
    return match[0];
  };
  const string = () => {
    const quote = source[i];
    i += 1;
    let out = '';
    for (;;) {
      const c = source[i];
      if (c === undefined || c === '\n') fail('Unterminated string');
      i += 1;
      if (c === quote) return out;
      if (c !== '\\') {
        out += c;
        continue;
      }
      const e = source[i];
      i += 1;
      if (e === 'n') out += '\n';
      else if (e === 't') out += '\t';
      else if (e === 'r') out += '\r';
      else if (e === 'u' || e === 'x') {
        const digits = e === 'u' ? 4 : 2;
        const hex = source.slice(i, i + digits);
        if (!new RegExp(`^[0-9a-fA-F]{${digits}}$`).test(hex)) fail('Bad escape');
        out += String.fromCharCode(parseInt(hex, 16));
        i += digits;
      } else if (e === '\n' || e === undefined) fail('Bad escape');
      else out += e;
    }
  };
  const safeKey = (key) => {
    if (key === '__proto__') fail('A "__proto__" key is not allowed');
    return key;
  };
  const reference = () => {
    const name = identifier();
    if (!known.has(name)) fail(`"${name}" is not something an earlier module exports`);
    let value = known.get(name);
    for (;;) {
      let key;
      if (source[i] === '.') {
        i += 1;
        key = identifier();
      } else if (source[i] === '[' && (source[i + 1] === "'" || source[i + 1] === '"')) {
        i += 1;
        key = string();
        take(']');
      } else return value;
      if (value === null || typeof value !== 'object' || !Object.hasOwn(value, key)) fail(`"${name}" has no "${key}"`);
      value = value[key];
    }
  };
  const literal = (depth) => {
    if (depth > MAX_DEPTH) fail('Nested too deeply');
    skip();
    const c = source[i];
    if (c === '{') {
      i += 1;
      const object = {};
      for (;;) {
        skip();
        if (source[i] === '}') {
          i += 1;
          return object;
        }
        let key;
        if (source[i] === "'" || source[i] === '"') key = string();
        else if (/[0-9]/.test(source[i] ?? '')) key = String(number());
        else key = identifier();
        take(':');
        object[safeKey(key)] = literal(depth + 1);
        skip();
        if (source[i] === ',') i += 1;
        else if (source[i] !== '}') fail('Expected "," or "}"');
      }
    }
    if (c === '[') {
      i += 1;
      const list = [];
      for (;;) {
        skip();
        if (source[i] === ']') {
          i += 1;
          return list;
        }
        list.push(literal(depth + 1));
        skip();
        if (source[i] === ',') i += 1;
        else if (source[i] !== ']') fail('Expected "," or "]"');
      }
    }
    if (c === "'" || c === '"') return string();
    if (c === '-' || /[0-9]/.test(c ?? '')) return number();
    const word = /^[A-Za-z_$][A-Za-z0-9_$]*/.exec(source.slice(i, i + 200))?.[0];
    if (word === 'true' || word === 'false' || word === 'null') {
      i += word.length;
      return word === 'null' ? null : word === 'true';
    }
    if (word) return reference();
    return fail('Unexpected character');
  };
  const number = () => {
    const match = /^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(source.slice(i, i + 64));
    if (!match) fail('Bad number');
    i += match[0].length;
    return Number(match[0]);
  };

  for (;;) {
    skip();
    if (i >= source.length) return exports;
    if (peek('import')) {
      take('import');
      take('{');
      const names = [];
      for (;;) {
        names.push(identifier());
        if (peek(',')) take(',');
        if (peek('}')) break;
      }
      take('}');
      take('from');
      skip();
      if (source[i] !== "'" && source[i] !== '"') fail('Expected a module path');
      string();
      if (peek(';')) take(';');
      for (const name of names) if (!known.has(name)) fail(`Cannot import "${name}"`);
    } else if (peek('export')) {
      take('export');
      take('const');
      const name = identifier();
      take('=');
      const value = literal(0);
      if (peek(';')) take(';');
      exports[safeKey(name)] = value;
      known.set(name, value);
    } else fail('Only imports and "export const" are allowed');
  }
}

/** Downloads Paimon.moe's data modules and reads them as data (see parseDataModule). */
async function loadSource() {
  const scope = {};
  for (const name of SOURCE_FILES) {
    const response = await fetch(`${RAW}/${name}.js`, { headers: { 'user-agent': 'Wishtory catalog maintenance' } });
    if (!response.ok) throw new Error(`${name}.js: HTTP ${response.status}`);
    const text = await response.text();
    Object.assign(scope, parseDataModule(text, scope));
  }
  return { characters: scope.characters, weaponList: scope.weaponList, itemGroup: scope.itemGroup };
}

/** Refuses a result that lost most of what it had: a broken source must not empty the app's data. */
export function sanityCheck(materials, previous) {
  const count = (m) => m.books.reduce((n, s) => n + s.users.length, 0) + m.weapons.reduce((n, s) => n + s.users.length, 0);
  if (materials.books.length < 10 || materials.weapons.length < 10) throw new Error('Too few material series; the source looks broken.');
  if (previous && count(materials) < count(previous) * 0.9) throw new Error('The new data lists far fewer characters and weapons than before; not writing it.');
}

async function main() {
  const args = process.argv.slice(2);
  const arg = (flag, fallback) => (args.includes(flag) ? args[args.indexOf(flag) + 1] : fallback);
  const catalogPath = arg('--catalog', 'catalog/catalog.json');
  const outPath = arg('--out', 'app/assets/data/materials.json');
  const catalog = JSON.parse(await readFile(catalogPath, 'utf8'));
  const { materials, problems } = buildMaterials(await loadSource(), catalog.items);
  let previous = null;
  try {
    previous = JSON.parse(await readFile(outPath, 'utf8'));
  } catch {
    // First run.
  }
  sanityCheck(materials, previous);
  for (const problem of problems) console.log(`note: ${problem}`);
  const text = `${JSON.stringify(materials, null, 1)}\n`;
  if (previous && JSON.stringify(previous) === JSON.stringify(materials)) {
    console.log('Materials are current.');
    return;
  }
  await mkdir(dirname(outPath), { recursive: true });
  await writeFile(outPath, text);
  console.log(`Materials updated: ${materials.books.length} talent book series, ${materials.weapons.length} weapon series.`);
}

if (import.meta.main) await main();
