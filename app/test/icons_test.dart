import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/domain/icons.dart';

CatalogItem item(String game, String id, String kind) =>
    CatalogItem(game: game, id: id, name: 'Test', aliases: const [], rarity: 5, kind: kind, paimonKey: null);

void main() {
  final table = IconTable({}, {'10000002': 'Ayaka', '11509': 'Mistsplitter'}, {'1091': 'IconRoleCircle13'});

  test('Genshin characters prefer the front-facing portrait, then the side-facing one', () {
    expect(table.iconUrls(item('hk4e', '10000002', 'character')), [
      'https://enka.network/ui/UI_AvatarIcon_Ayaka.png',
      'https://enka.network/ui/UI_AvatarIcon_Side_Ayaka.png',
    ]);
  });

  test('Genshin weapons have one picture', () {
    expect(table.iconUrls(item('hk4e', '11509', 'weapon')), ['https://enka.network/ui/UI_EquipIcon_Mistsplitter.png']);
  });

  test('Star Rail and Zenless derive their pictures', () {
    expect(table.iconUrls(item('hkrpg', '1001', 'character')), ['https://enka.network/ui/hsr/SpriteOutput/AvatarRoundIcon/Avatar/1001.png']);
    expect(table.iconUrls(item('hkrpg', '23001', 'weapon')), ['https://enka.network/ui/hsr/SpriteOutput/LightConeMaxFigures/23001.png']);
    expect(table.iconUrls(item('nap', '1091', 'character')), ['https://enka.network/ui/zzz/IconRoleCircle13.png']);
  });

  test('unknown items and bad ids give no pictures', () {
    expect(table.iconUrls(null), isEmpty);
    expect(table.iconUrls(item('hk4e', '99999999', 'character')), isEmpty);
    expect(table.iconUrls(item('hk4e', 'abc', 'character')), isEmpty);
  });

  group('banner artwork', () {
    test('Genshin tries the wish-screen picture first, then the ordinary one', () {
      expect(table.bannerArtUrls(item('hk4e', '10000002', 'character')), [
        'https://enka.network/ui/UI_Gacha_AvatarImg_Ayaka.png',
        'https://enka.network/ui/UI_AvatarIcon_Ayaka.png',
      ]);
      expect(table.bannerArtUrls(item('hk4e', '11509', 'weapon')), [
        'https://enka.network/ui/UI_Gacha_EquipIcon_Mistsplitter.png',
        'https://enka.network/ui/UI_EquipIcon_Mistsplitter.png',
      ]);
    });

    test('Star Rail and Zenless have their own artwork', () {
      expect(table.bannerArtUrls(item('hkrpg', '1001', 'character')), [
        'https://enka.network/ui/hsr/SpriteOutput/AvatarDrawCard/1001.png',
        'https://enka.network/ui/hsr/SpriteOutput/AvatarRoundIcon/Avatar/1001.png',
      ]);
      expect(table.bannerArtUrls(item('hkrpg', '23001', 'weapon')), ['https://enka.network/ui/hsr/SpriteOutput/LightConeMaxFigures/23001.png']);
      // A Zenless agent's large portrait comes before the round icon; a W-Engine has one picture.
      expect(table.bannerArtUrls(item('nap', '1091', 'character')), [
        'https://enka.network/ui/zzz/IconRole13.png',
        'https://enka.network/ui/zzz/IconRoleCircle13.png',
      ]);
      expect(table.bannerArtUrls(item('nap', '14102', 'weapon')), ['https://enka.network/ui/zzz/Weapon_S_1021.png']);
    });

    test('unknown items and bad ids give nothing, so the card shows its gradient', () {
      expect(table.bannerArtUrls(null), isEmpty);
      expect(table.bannerArtUrls(item('hk4e', '99999999', 'character')), isEmpty);
      expect(table.bannerArtUrls(item('hk4e', 'abc', 'character')), isEmpty);
    });
  });

  group('Paimon.moe fallback for items Enka has no name for yet', () {
    CatalogItem keyed(String id, String kind, String? key) =>
        CatalogItem(game: 'hk4e', id: id, name: 'Test', aliases: const [], rarity: 5, kind: kind, paimonKey: key);
    const paimon = 'https://raw.githubusercontent.com/MadeBaruna/paimon-moe/main/static/images';

    test('a new character is shown from its full wish art, then its portrait', () {
      expect(table.bannerArtUrls(keyed('10000136', 'character', 'mitya')), [
        '$paimon/characters/full/mitya.png',
        '$paimon/characters/mitya.png',
      ]);
      expect(table.iconUrls(keyed('10000136', 'character', 'mitya')), ['$paimon/characters/mitya.png']);
    });

    test('a new weapon is shown from its picture', () {
      expect(table.bannerArtUrls(keyed('14525', 'weapon', 'starsealed_edict')), ['$paimon/weapons/starsealed_edict.png']);
      expect(table.iconUrls(keyed('14525', 'weapon', 'starsealed_edict')), ['$paimon/weapons/starsealed_edict.png']);
    });

    test('Enka stays first, and the full wish art comes before any small portrait', () {
      expect(table.bannerArtUrls(keyed('10000002', 'character', 'kamisato_ayaka')), [
        'https://enka.network/ui/UI_Gacha_AvatarImg_Ayaka.png',
        '$paimon/characters/full/kamisato_ayaka.png',
        'https://enka.network/ui/UI_AvatarIcon_Ayaka.png',
        '$paimon/characters/kamisato_ayaka.png',
      ]);
      expect(table.iconUrls(keyed('11509', 'weapon', 'mistsplitter_reforged')), [
        'https://enka.network/ui/UI_EquipIcon_Mistsplitter.png',
        '$paimon/weapons/mistsplitter_reforged.png',
      ]);
    });

    test('a key that is not plain lowercase words never reaches an address', () {
      for (final bad in ['../x', 'a/b', 'Mitya', 'mit ya', 'a?b=1', '']) {
        expect(table.bannerArtUrls(keyed('10000136', 'character', bad)), isEmpty, reason: bad);
        expect(table.iconUrls(keyed('10000136', 'character', bad)), isEmpty, reason: bad);
      }
    });

    test('other games are not given Paimon.moe pictures', () {
      final hsr = CatalogItem(game: 'hkrpg', id: '1001', name: 'Test', aliases: const [], rarity: 5, kind: 'character', paimonKey: 'march_7th');
      expect(table.iconUrls(hsr), ['https://enka.network/ui/hsr/SpriteOutput/AvatarRoundIcon/Avatar/1001.png']);
    });
  });

  group('farming material pictures', () {
    test('a series is shown by the picture of its lowest-tier item', () {
      expect(table.materialUrls('teachings_of_freedom'), [
        'https://raw.githubusercontent.com/MadeBaruna/paimon-moe/main/static/images/items/teachings_of_freedom.png',
      ]);
    });

    test('an id that is not plain lowercase words gives nothing', () {
      expect(table.materialUrls('../secret'), isEmpty);
      expect(table.materialUrls('Teachings'), isEmpty);
      expect(table.materialUrls(''), isEmpty);
    });
  });
}
