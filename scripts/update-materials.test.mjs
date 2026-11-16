import test from 'node:test';
import assert from 'node:assert/strict';
import { buildMaterials, dayNumbers, sanityCheck, SERIES_OVERRIDES } from './update-materials.mjs';

const catalog = [
  { game: 'hk4e', kind: 'character', id: '1', name: 'Venti', rarity: 5, paimon_key: 'venti' },
  { game: 'hk4e', kind: 'character', id: '2', name: 'Barbara', rarity: 4, paimon_key: 'barbara' },
  { game: 'hk4e', kind: 'weapon', id: '3', name: 'Skyward Harp', rarity: 5, paimon_key: 'skyward_harp' },
  { game: 'hk4e', kind: 'weapon', id: '4', name: 'Starsealed Edict', rarity: 5, paimon_key: 'starsealed_edict' },
  { game: 'hk4e', kind: 'weapon', id: '5', name: 'Old Stick', rarity: 3, paimon_key: 'old_stick' },
  { game: 'hkrpg', kind: 'character', id: '9', name: 'Venti', rarity: 5, paimon_key: 'venti' },
];
const ballad = { id: 'teachings_of_ballad', name: 'Teachings of Ballad', day: ['wednesday', 'saturday'] };
const source = {
  characters: {
    venti: { id: 'venti', material: { book: [ballad] } },
    barbara: { id: 'barbara', material: { book: [ballad] } },
    aloy: { id: 'aloy', material: { book: [ballad] } },
    nobook: { id: 'nobook', material: {} },
  },
  itemGroup: { boreal_wolfs_milk_tooth: { name: 'Boreal Wolf', items: [{ name: "Boreal Wolf's Milk Tooth" }] } },
  weaponList: {
    skyward_harp: { id: 'skyward_harp', rarity: 5, ascension: [{ items: [{ item: { id: 'boreal_wolfs_cracked_tooth', name: 'Cracked Tooth', day: ['tuesday', 'friday'], parent: 'boreal_wolfs_milk_tooth' } }] }] },
    starsealed_edict: { id: 'starsealed_edict', rarity: 5, ascension: [{ items: [{ item: { id: 'the_frost_emperors_revival', name: 'Revival' } }] }] },
    old_stick: { id: 'old_stick', rarity: 3, ascension: [] },
    mystery: { id: 'mystery', rarity: 5, ascension: [{ items: [{ item: { id: 'unknown_thing', name: 'Unknown' } }] }] },
  },
};

test('weekday names become numbers, Monday = 1', () => {
  assert.deepEqual(dayNumbers(['thursday', 'Monday', 'monday', 'someday']), [1, 4]);
  assert.deepEqual(dayNumbers(undefined), []);
});

test('talent books group characters by book and use the catalog names', () => {
  const { materials, problems } = buildMaterials(source, catalog);
  assert.equal(materials.books.length, 1);
  const book = materials.books[0];
  assert.equal(book.name, 'Ballad');
  assert.deepEqual(book.days, [3, 6]);
  assert.deepEqual(book.users, [{ name: 'Venti', rarity: 5 }, { name: 'Barbara', rarity: 4 }]);
  assert.ok(problems.includes('character aloy: not in the catalog'));
  assert.ok(problems.includes('character nobook: not in the catalog'));
});

test('a game other than Genshin never leaks in', () => {
  const { materials } = buildMaterials(source, catalog);
  assert.equal(materials.books[0].users.filter((u) => u.name === 'Venti').length, 1);
});

test('weapon series come from item days or from an override, and 3-star weapons are left out', () => {
  const { materials, problems } = buildMaterials(source, catalog);
  assert.deepEqual(materials.weapons.map((s) => s.name).sort(), ['Boreal Wolf', 'Frost Emperor']);
  const wolf = materials.weapons.find((s) => s.name === 'Boreal Wolf');
  assert.deepEqual(wolf.days, [2, 5]);
  assert.equal(wolf.title, "Boreal Wolf's Milk Tooth");
  assert.deepEqual(wolf.users, [{ name: 'Skyward Harp', rarity: 5 }]);
  const frost = materials.weapons.find((s) => s.name === 'Frost Emperor');
  assert.deepEqual(frost.days, [3, 6]);
  assert.deepEqual(frost.users.map((u) => u.name), ['Starsealed Edict']);
  assert.ok(!JSON.stringify(materials).includes('Old Stick'));
  assert.ok(problems.includes('weapon mystery: not in the catalog'));
});

test('every override has days that are real weekdays and no Sunday', () => {
  for (const [id, series] of Object.entries(SERIES_OVERRIDES)) {
    assert.ok(series.days.length === 2, id);
    assert.ok(series.days.every((d) => d >= 1 && d <= 6), id);
    assert.ok(series.name && series.title, id);
  }
});

test('series are sorted by their first day, then name', () => {
  const many = { ...source, characters: { ...source.characters } };
  const { materials } = buildMaterials(many, catalog);
  const firstDays = materials.weapons.map((s) => s.days[0]);
  assert.deepEqual(firstDays, [...firstDays].sort((a, b) => a - b));
});

test('a broken or shrunken result is refused', () => {
  const series = (n) => Array.from({ length: n }, (_, i) => ({ users: [{ name: `x${i}` }] }));
  assert.throws(() => sanityCheck({ books: series(3), weapons: series(3) }, null), /too few/i);
  const good = { books: series(12), weapons: series(12) };
  sanityCheck(good, null);
  assert.throws(() => sanityCheck({ books: series(10), weapons: series(10) }, { books: series(30), weapons: series(30) }), /fewer/i);
  sanityCheck(good, { books: series(12), weapons: series(12) });
});
