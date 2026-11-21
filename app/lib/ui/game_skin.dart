import 'package:flutter/material.dart';

import '../bridge/wishtory_api.dart';

/// How the open game cuts its interface, carried in the theme next to the colours so every widget can ask for it:
///   Genshin Impact     soft and rounded, pill buttons, sparkles and diamonds.
///   Honkai: Star Rail  bevelled corners (top left and bottom right), rail-line ornaments, spaced capitals.
///   Zenless Zone Zero  bevelled the other way (top right and bottom left), slanted blocks, heavy italics.
/// With no game (the start-up error screen, tests) everything keeps the original square-ish look.
@immutable
class GameSkin extends ThemeExtension<GameSkin> {
  const GameSkin(this.game);

  final GameId? game;

  bool get isGenshin => game == GameId.genshin;
  bool get isStarRail => game == GameId.starRail;
  bool get isZenless => game == GameId.zenless;

  /// A rectangle's outline in this game's cut. [r] is the size of the corner treatment: a radius for the rounded
  /// look, the size of the cut for the bevelled ones.
  OutlinedBorder shape({double r = 12, BorderSide side = BorderSide.none}) => switch (game) {
        null => RoundedRectangleBorder(borderRadius: BorderRadius.circular(r), side: side),
        GameId.genshin => RoundedRectangleBorder(borderRadius: BorderRadius.circular(r * 1.4), side: side),
        GameId.starRail => BeveledRectangleBorder(
            borderRadius: BorderRadius.only(topLeft: Radius.circular(r * 1.2), bottomRight: Radius.circular(r * 1.2)),
            side: side,
          ),
        GameId.zenless => BeveledRectangleBorder(
            borderRadius: BorderRadius.only(topRight: Radius.circular(r), bottomLeft: Radius.circular(r)),
            side: side,
          ),
      };

  /// The outline of something small and wide (a chip, a button): a pill for Genshin Impact, cut for the others.
  OutlinedBorder pill({BorderSide side = BorderSide.none}) => switch (game) {
        null || GameId.genshin => StadiumBorder(side: side),
        GameId.starRail => BeveledRectangleBorder(borderRadius: const BorderRadius.only(topLeft: Radius.circular(8), bottomRight: Radius.circular(8)), side: side),
        GameId.zenless => BeveledRectangleBorder(borderRadius: const BorderRadius.only(topRight: Radius.circular(8), bottomLeft: Radius.circular(8)), side: side),
      };

  /// The sidebar's mark: a sparkle, the Astral Express, a television.
  IconData get brandIcon => switch (game) {
        null || GameId.genshin => Icons.auto_awesome,
        GameId.starRail => Icons.train,
        GameId.zenless => Icons.live_tv,
      };

  /// The app's name as the sidebar writes it.
  String get wordmark => game == null || isGenshin ? 'Wishtory' : 'WISHTORY';

  /// Wraps [base] in this game's lettering.
  TextStyle wordmarkStyle(TextStyle base) => switch (game) {
        null || GameId.genshin => base,
        GameId.starRail => base.copyWith(fontWeight: FontWeight.w500, letterSpacing: 2.2, fontSize: (base.fontSize ?? 17) - 2),
        GameId.zenless => base.copyWith(fontWeight: FontWeight.w800, fontStyle: FontStyle.italic, letterSpacing: 0.4),
      };

  /// A page title ("Genshin Impact") in this game's lettering.
  String title(String name) => game == null || isGenshin ? name : name.toUpperCase();

  TextStyle titleStyle(TextStyle base) => switch (game) {
        null || GameId.genshin => base,
        GameId.starRail => base.copyWith(fontWeight: FontWeight.w500, letterSpacing: 1.6, fontSize: (base.fontSize ?? 22) - 3),
        GameId.zenless => base.copyWith(fontWeight: FontWeight.w800, fontStyle: FontStyle.italic, letterSpacing: 0.2),
      };

  /// A small ornament that leads a section heading, or null for the original plain look.
  Widget? headerMark(Color color) => switch (game) {
        null => null,
        GameId.genshin => ExcludeSemantics(
            child: Transform.rotate(
              angle: 0.7853981633974483,
              child: SizedBox(width: 7, height: 7, child: DecoratedBox(decoration: BoxDecoration(color: color))),
            ),
          ),
        GameId.starRail => ExcludeSemantics(child: SizedBox(width: 14, height: 2, child: DecoratedBox(decoration: BoxDecoration(color: color)))),
        GameId.zenless => ExcludeSemantics(
            child: Transform(
              transform: Matrix4.skewX(-0.4),
              child: SizedBox(width: 7, height: 14, child: DecoratedBox(decoration: BoxDecoration(color: color))),
            ),
          ),
      };

  @override
  GameSkin copyWith({GameId? game}) => GameSkin(game ?? this.game);

  @override
  GameSkin lerp(ThemeExtension<GameSkin>? other, double t) => other is GameSkin && t >= 0.5 ? other : this;
}

extension GameSkinOnContext on BuildContext {
  /// The open game's skin, or the original look when the theme has none.
  GameSkin get skin => Theme.of(this).extension<GameSkin>() ?? const GameSkin(null);
}
