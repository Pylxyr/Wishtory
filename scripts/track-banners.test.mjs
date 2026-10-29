import test from 'node:test';
import assert from 'node:assert/strict';
import { buildReport, insertRows, newFiveStars, parsePaimonCharacters, toWindow, trackBanners } from './track-banners.mjs';

const text = `export const banners = {
  characters: [
    {
      name: "Void Star's Advent",
      shortName: 'Skirk',
      start: '2026-09-23 06:00:00',
      end: '2026-10-13 17:59:59',
      featured: ['skirk', 'escoffier'],
      featuredRare: ['x'],
      version: '7.1',
    },
    // {
    //   name: 'Commented out',
    // },
    {
      name: 'Second Phase',
      start: '2026-10-13 18:00:00',
      end: '2026-11-03 14:59:00',
      featured: ['venti'],
      version: '7.1',
    },
  ],
  weapons: [
    { name: 'Not a character', start: '2026-01-01 00:00:00', end: '2026-01-02 00:00:00', featured: ['x'] },
  ],
};`;

test('reads the character list without running the file', () => {
  const rows = parsePaimonCharacters(text);
  assert.equal(rows.length, 2);
  assert.deepEqual(rows[0], { name: "Void Star's Advent", start: '2026-09-23 06:00:00', end: '2026-10-13 17:59:59', version: '7.1', featured: ['skirk', 'escoffier'] });
  assert.deepEqual(rows[1].featured, ['venti']);
  assert.throws(() => parsePaimonCharacters('export const banners = {}'));
});

test('turns Paimon.moe times into the list conventions', () => {
  const [first, second] = parsePaimonCharacters(text);
  assert.deepEqual(toWindow(first), { start: '2026-09-22 15:00:00', end: '2026-10-13 17:59:59' });
  assert.deepEqual(toWindow(second), { start: '2026-10-13 18:00:00', end: '2026-11-03 14:59:59' });
  assert.equal(toWindow({ start: '2026-10-13 09:00:00', end: '2026-10-14 17:59:59' }), null);
});

const items = [
  { game: 'hk4e', id: '1', kind: 'character', rarity: 5, name: 'Skirk', paimon_key: 'skirk' },
  { game: 'hk4e', id: '2', kind: 'character', rarity: 5, name: 'Escoffier', paimon_key: 'escoffier' },
  { game: 'hk4e', id: '3', kind: 'character', rarity: 5, name: 'Venti', paimon_key: 'venti' },
];
const ours = [{ id: 'hk4e-character-2026-09-23', game: 'hk4e', pool: 'character', name: "Void Star's Advent / La Chanson Cerise", start: '2026-09-22 15:00:00', end: '2026-10-13 17:59:59', featured_5: ['1', '2'] }];

test('drafts only events newer than the list and leaves matching ones alone', () => {
  const { added, problems } = trackBanners(parsePaimonCharacters(text), ours, items);
  assert.deepEqual(problems, []);
  assert.deepEqual(added, [{ id: 'hk4e-character-2026-10-13', game: 'hk4e', pool: 'character', name: 'Second Phase', start: '2026-10-13 18:00:00', end: '2026-11-03 14:59:59', featured_5: ['3'] }]);
});

test('reports a difference or an unknown character instead of editing', () => {
  const wrongEnd = [{ ...ours[0], end: '2026-10-13 17:59:00' }];
  assert.match(trackBanners(parsePaimonCharacters(text), wrongEnd, items).problems[0], /differs from Paimon\.moe \(end/);
  const noItems = items.filter((i) => i.id !== '2');
  assert.match(trackBanners(parsePaimonCharacters(text), ours, noItems).problems[0], /no entry for escoffier/);
});

test('inserts drafted rows after the last Genshin character row', () => {
  const file = '{\n  "banners": [\n    {"id": "hk4e-character-a", "game": "hk4e"},\n    {"id": "hk4e-weapon-a", "game": "hk4e"}\n  ]\n}';
  const out = insertRows(file, [{ id: 'hk4e-character-b', game: 'hk4e', pool: 'character', name: 'N', start: 's', end: 'e', featured_5: ['1'] }]);
  const lines = out.split('\n');
  assert.match(lines[3], /"id": "hk4e-character-b".*"featured_5": \["1"\]\},$/);
  assert.match(lines[4], /hk4e-weapon-a/);
});

test('lists only 5-star characters that are new', () => {
  const previous = [items[0]];
  assert.deepEqual(newFiveStars(previous, items), ['Escoffier (hk4e)', 'Venti (hk4e)']);
  assert.equal(buildReport({ added: [], problems: [], newFives: [] }), '');
  assert.match(buildReport({ added: [], problems: ['x'], newFives: ['Venti (hk4e)'] }), /Needs a look[\s\S]*New 5-star characters/);
});
