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
}
