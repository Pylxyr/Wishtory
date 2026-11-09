import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

import '../bridge/generated/api/types.dart';
import 'game_info.dart';

/// Item icons come from Enka.Network's public image host (credited in About). Star Rail paths
/// derive from the catalog id; Genshin and Zenless need the lookup tables (5★ only). Anything
/// missing gives no address and the UI draws a star glyph instead. Pictures are saved on disk by
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
        if (key == null) return const [];
        return weapon
            ? ['$_enka/ui/UI_EquipIcon_$key.png']
            : ['$_enka/ui/UI_AvatarIcon_$key.png', '$_enka/ui/UI_AvatarIcon_Side_$key.png'];
      case 'nap':
        final key = _zenless[item.id];
        return key == null ? const [] : ['$_enka/ui/zzz/$key.png'];
    }
    return const [];
  }
}

/// Catalog lookups by (game, item name), built once per catalog.
class CatalogIndex {
  CatalogIndex(GameCatalog? catalog) {
    for (final item in catalog?.items ?? const <CatalogItem>[]) {
      _byName['${item.game}|${item.name.toLowerCase()}'] = item;
      for (final alias in item.aliases) {
        _byName.putIfAbsent('${item.game}|${alias.toLowerCase()}', () => item);
      }
    }
  }

  final Map<String, CatalogItem> _byName = {};

  CatalogItem? find(GameId game, String name) => _byName['${gameKey(game)}|${name.toLowerCase()}'];
}
