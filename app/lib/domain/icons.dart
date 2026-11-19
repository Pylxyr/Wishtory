import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

import '../bridge/generated/api/types.dart';
import 'game_info.dart';

/// Item icons come from Enka.Network's public image host (credited in About). Star Rail paths
/// derive from the catalog id; Genshin and Zenless need the lookup tables (5★ only). Genshin items the
/// table does not know yet (a new character or weapon is usually missing for a while) fall back to the
/// pictures in Paimon.moe's open-source repository, which the catalog's `paimon_key` points at. Anything
/// still missing gives no address and the UI draws a star glyph instead. Pictures are saved on disk by
/// `IconCache` after the first download.
class IconTable {
  IconTable(this._elements, this._genshin, this._zenless);

  static const _enka = 'https://enka.network';

  final Map<String, String> _elements;
  final Map<String, String> _genshin;
  final Map<String, String> _zenless;

  static Future<IconTable> load() async {
    final raw = jsonDecode(await rootBundle.loadString('assets/data/icons.json')) as Map<String, dynamic>;
    Map<String, String> table(String key) => (raw[key] as Map<String, dynamic>).map((k, v) => MapEntry(k, v as String));
    return IconTable(table('element_by_name'), table('genshin_icons'), table('zenless_icons'));
  }

  /// The element of a Genshin/Star Rail character by English name, or "neutral".
  String elementOf(String name) => _elements[name.trim().toLowerCase()] ?? 'neutral';

  /// The picture addresses to try, best first. Genshin characters use the front-facing portrait and
  /// fall back to the side-facing one; everything else has a single picture.
  List<String> iconUrls(CatalogItem? item) {
    if (item == null || !RegExp(r'^\d+$').hasMatch(item.id)) return const [];
    final weapon = item.kind != 'character';
    switch (item.game) {
      case 'hkrpg':
        return ['$_enka/ui/hsr/SpriteOutput/${weapon ? 'LightConeMaxFigures' : 'AvatarRoundIcon/Avatar'}/${item.id}.png'];
      case 'hk4e':
        final key = _genshin[item.id];
        final paimon = _paimonKey(item);
        return [
          if (key != null) ...(weapon ? ['$_enka/ui/UI_EquipIcon_$key.png'] : ['$_enka/ui/UI_AvatarIcon_$key.png', '$_enka/ui/UI_AvatarIcon_Side_$key.png']),
          if (paimon != null) '$_paimon/${weapon ? 'weapons' : 'characters'}/$paimon.png',
        ];
      case 'nap':
        final key = _zenless[item.id];
        return key == null ? const [] : ['$_enka/ui/zzz/$key.png'];
    }
    return const [];
  }
}

/// Paimon.moe's open-source repository (MIT, credited in About) serves pictures for Genshin items, named by the
/// catalog's `paimon_key`. Only used when Enka has no address for an item, or as the next try after it.
const _paimon = 'https://raw.githubusercontent.com/MadeBaruna/paimon-moe/main/static/images';

/// Item keys are lowercase words joined by underscores; anything else is never put into an address.
final _paimonKeyPattern = RegExp(r'^[a-z0-9_]+$');

String? _paimonKey(CatalogItem? item) {
  final key = item?.paimonKey;
  return key != null && _paimonKeyPattern.hasMatch(key) ? key : null;
}

extension BannerArt on IconTable {
  /// The picture addresses to try for a banner's background, best first: the wish-screen artwork of the
  /// featured item where a host has it, then its ordinary picture (so an unknown artwork name still
  /// shows something). Used with `IconRequest(urls, art: true)`.
  List<String> bannerArtUrls(CatalogItem? item) {
    if (item == null || !RegExp(r'^\d+$').hasMatch(item.id)) return const [];
    final weapon = item.kind != 'character';
    const enka = IconTable._enka;
    switch (item.game) {
      case 'hk4e':
        final key = _genshin[item.id];
        final paimon = _paimonKey(item);
        if (weapon) {
          return [
            if (key != null) ...['$enka/ui/UI_Gacha_EquipIcon_$key.png', '$enka/ui/UI_EquipIcon_$key.png'],
            if (paimon != null) '$_paimon/weapons/$paimon.png',
          ];
        }
        // The full wish-screen picture of a character comes before any small portrait, from either host.
        return [
          if (key != null) '$enka/ui/UI_Gacha_AvatarImg_$key.png',
          if (paimon != null) '$_paimon/characters/full/$paimon.png',
          if (key != null) '$enka/ui/UI_AvatarIcon_$key.png',
          if (paimon != null) '$_paimon/characters/$paimon.png',
        ];
      case 'hkrpg':
        return weapon
            ? ['$enka/ui/hsr/SpriteOutput/LightConeMaxFigures/${item.id}.png']
            : ['$enka/ui/hsr/SpriteOutput/AvatarDrawCard/${item.id}.png', '$enka/ui/hsr/SpriteOutput/AvatarRoundIcon/Avatar/${item.id}.png'];
      case 'nap':
        final key = _zenless[item.id];
        if (key == null) return const [];
        // Enka names an agent's large portrait like its round icon without "Circle" (IconRoleCircle13 and
        // IconRole13): the portrait first, the round icon only if that picture is missing.
        final portrait = weapon ? null : key.replaceFirst('IconRoleCircle', 'IconRole');
        return [
          if (portrait != null && portrait != key) '$enka/ui/zzz/$portrait.png',
          '$enka/ui/zzz/$key.png',
        ];
    }
    return const [];
  }
}

extension MaterialPictures on IconTable {
  /// The picture of a farming series, such as "Teachings of Freedom", by the series id in `materials.json`
  /// (the key of its lowest-tier item). Paimon.moe's repository has one for every series.
  List<String> materialUrls(String seriesId) => _paimonKeyPattern.hasMatch(seriesId) ? ['$_paimon/items/$seriesId.png'] : const [];
}

/// Catalog lookups by (game, item name), built once per catalog.
class CatalogIndex {
  CatalogIndex(GameCatalog? catalog) {
    for (final item in catalog?.items ?? const <CatalogItem>[]) {
      // The first item with a name wins, like the app's own name lookup in the Rust core.
      _byName.putIfAbsent('${item.game}|${item.name.toLowerCase()}', () => item);
      for (final alias in item.aliases) {
        _byName.putIfAbsent('${item.game}|${alias.toLowerCase()}', () => item);
      }
    }
  }

  final Map<String, CatalogItem> _byName = {};

  CatalogItem? find(GameId game, String name) => _byName['${gameKey(game)}|${name.toLowerCase()}'];
}
