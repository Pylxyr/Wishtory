import test from 'node:test';
import assert from 'node:assert/strict';
import { buildItems } from './update-catalog.mjs';

const sources = {
  giLocs: { en: { 1: 'Kamisato Ayaka', 2: 'Cool Steel' } },
  giAvatars: { 10000002: { QualityType: 'QUALITY_ORANGE', NameTextMapHash: 1, Element: 'Ice' } },
  giWeapons: { 11101: { Rarity: 3, NameTextMapHash: 2 } },
  paimonCharacters: ['kamisato_ayaka'],
  paimonWeapons: { cool_steel: { id: 'cool_steel', name: 'Cool Steel' } },
  hsrLocs: { en: { '123': 'March 7th', '456': 'Passkey' } },
  hsrAvatars: { 1001: { Rarity: 4, AvatarName: { Hash: '123' }, Element: 'Ice', AvatarBaseType: 'Knight' } },
  hsrWeapons: { 2001: { Rarity: 3, EquipmentName: { Hash: '456' } } },
  zzzLocs: { en: { Agent_Anby: 'Anby', WEngine: 'Vortex' } },
  zzzAvatars: { 1011: { Rarity: 4, Name: 'Agent_Anby', ElementTypes: ['Elec'], ProfessionType: 'Stun' } },
  zzzWeapons: { 2001: { Rarity: 4, ItemName: 'WEngine', ProfessionType: 'Attack' } }
};
const fixtureMinimum = { hk4e: 1, hkrpg: 1, nap: 1 };

test('normalizes all three games and preserves exact Paimon keys', () => {
  const items = buildItems(sources, [], fixtureMinimum);
  assert.equal(items.length, 6);
  assert.deepEqual(items.find(x => x.id === '10000002').traits, { element: 'cryo' });
  assert.equal(items.find(x => x.id === '10000002').paimon_key, 'kamisato_ayaka');
  assert.equal(items.find(x => x.id === '11101').paimon_key, 'cool_steel');
  assert.deepEqual(items.find(x => x.game === 'hkrpg' && x.kind === 'character').traits, { element: 'ice', path: 'preservation' });
  assert.deepEqual(items.find(x => x.game === 'nap' && x.kind === 'character').traits, { element: 'electric', specialty: 'stun' });
  assert.equal(items.find(x => x.game === 'nap' && x.kind === 'character').rarity, 5);
});

test('retains known items when an upstream dataset drops an old record', () => {
  const retired = { game: 'hkrpg', id: '9999', name: 'Retired Light Cone', aliases: [], rarity: 4, kind: 'light_cone', traits: {}, paimon_key: null };
  const items = buildItems(sources, [retired], fixtureMinimum);
  assert.ok(items.some(x => x.id === '9999' && x.name === 'Retired Light Cone'));
});

test('ignores non-playable Genshin avatar rows with no element sentinel', () => {
  const withPlaceholder = {
    ...sources,
    giAvatars: {
      ...sources.giAvatars,
      10009999: { QualityType: 'QUALITY_ORANGE', NameTextMapHash: 1, Element: 'none' }
    }
  };
  const items = buildItems(withPlaceholder, [], fixtureMinimum);
  assert.deepEqual(items.filter(item => item.game === 'hk4e' && item.kind === 'character').map(item => item.id), ['10000002']);
});

test('normalizes the new Star Rail Elation path', () => {
  const withElation = {
    ...sources,
    hsrLocs: { en: { ...sources.hsrLocs.en, '789': 'Sparxie' } },
    hsrAvatars: {
      ...sources.hsrAvatars,
      1501: { Rarity: 5, AvatarName: { Hash: '789' }, Element: 'Fire', AvatarBaseType: 'Elation' }
    }
  };
  const sparxie = buildItems(withElation, [], fixtureMinimum).find(item => item.name === 'Sparxie');
  assert.deepEqual(sparxie.traits, { element: 'fire', path: 'elation' });
});

test('normalizes the Zenless Physics element spelling', () => {
  const withPhysics = {
    ...sources,
    zzzLocs: { en: { ...sources.zzzLocs.en, Agent_Trigger: 'Trigger' } },
    zzzAvatars: {
      ...sources.zzzAvatars,
      1151: { Rarity: 4, Name: 'Agent_Trigger', ElementTypes: ['Physics'], ProfessionType: 'Attack' }
    }
  };
  const trigger = buildItems(withPhysics, [], fixtureMinimum).find(item => item.name === 'Trigger');
  assert.equal(trigger.traits.element, 'physical');
});

test('keeps the standard element for Zenless variant attributes and accepts newer elements', () => {
  const withNewElements = {
    ...sources,
    zzzLocs: { en: { ...sources.zzzLocs.en, Agent_Miyabi: 'Miyabi', Agent_Velina: 'Velina', Agent_Remielle: 'Remielle' } },
    zzzAvatars: {
      ...sources.zzzAvatars,
      1091: { Rarity: 4, Name: 'Agent_Miyabi', ElementTypes: ['FireFrost', 'Ice'], ProfessionType: 'Anomaly' },
      1201: { Rarity: 4, Name: 'Agent_Velina', ElementTypes: ['Wind'], ProfessionType: 'Anomaly' },
      1581: { Rarity: 4, Name: 'Agent_Remielle', ElementTypes: ['Lumen'], ProfessionType: 'Anomaly' }
    }
  };
  const items = buildItems(withNewElements, [], fixtureMinimum);
  assert.equal(items.find(item => item.name === 'Miyabi').traits.element, 'ice');
  assert.equal(items.find(item => item.name === 'Velina').traits.element, 'wind');
  assert.equal(items.find(item => item.name === 'Remielle').traits.element, 'lumen');
});

test('normalizes the new Zenless Armorer specialty', () => {
  const withArmorer = {
    ...sources,
    zzzLocs: { en: { ...sources.zzzLocs.en, Agent_Claret: 'Claret' } },
    zzzAvatars: {
      ...sources.zzzAvatars,
      1611: { Rarity: 4, Name: 'Agent_Claret', ElementTypes: ['Elec'], ProfessionType: 'Armorer' }
    }
  };
  const claret = buildItems(withArmorer, [], fixtureMinimum).find(item => item.name === 'Claret');
  assert.equal(claret.traits.specialty, 'armorer');
});

test('rejects incomplete or malformed upstream data instead of publishing it', () => {
  assert.throws(() => buildItems({ ...sources, zzzAvatars: {}, zzzWeapons: {} }, [], fixtureMinimum), /incomplete for nap/);
  const bad = { ...sources, hsrAvatars: { ...sources.hsrAvatars, 1001: { ...sources.hsrAvatars['1001'], AvatarBaseType: 'UnknownPath' } } };
  assert.throws(() => buildItems(bad, [], fixtureMinimum), /missing traits/);
});
