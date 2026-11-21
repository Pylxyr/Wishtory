import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/ui/game_skin.dart';
import 'package:wishtory/ui/tokens.dart';

void main() {
  test('each game cuts its shapes its own way; with no game the original rounded look stays', () {
    expect(const GameSkin(null).shape(), isA<RoundedRectangleBorder>());
    expect(const GameSkin(GameId.genshin).shape(), isA<RoundedRectangleBorder>());
    expect(const GameSkin(GameId.starRail).shape(), isA<BeveledRectangleBorder>());
    expect(const GameSkin(GameId.zenless).shape(), isA<BeveledRectangleBorder>());
    // Star Rail and Zenless Zone Zero are bevelled on opposite corners.
    expect(const GameSkin(GameId.starRail).shape(), isNot(equals(const GameSkin(GameId.zenless).shape())));
  });

  test('the theme carries the open game\'s skin, and none for the start-up screen', () {
    expect(buildTheme(Brightness.dark).extension<GameSkin>()?.game, isNull);
    for (final g in GameId.values) {
      for (final b in Brightness.values) {
        expect(buildTheme(b, game: g).extension<GameSkin>()?.game, g, reason: '$g $b');
      }
    }
  });
}
