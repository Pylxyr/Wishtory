import '../bridge/generated/api/types.dart';

/// Words that differ per game, in one place so every screen says "S-rank" for Zenless and "5★"
/// elsewhere. (Ported from the old `GAMES` table.)
class GameInfo {
  const GameInfo({
    required this.name,
    required this.short,
    required this.abbr,
    required this.howTo,
    required this.historyPage,
    required this.r5,
    required this.r4,
    required this.five,
    required this.fives,
    required this.currency,
    required this.pullCost,
  });

  final String name, short, abbr, howTo, historyPage;

  /// "5★" / "S-rank".
  final String r5, r4;

  /// "five-star" / "S-rank pull" and their plurals.
  final String five, fives;
  final String currency;

  /// Premium currency per pull, for comparisons only.
  final int pullCost;

  static const _all = <GameId, GameInfo>{
    GameId.genshin: GameInfo(
      name: 'Genshin Impact',
      short: 'Genshin',
      abbr: 'GI',
      howTo: 'Open the Paimon menu, then Wish, then History, and flick through a banner tab.',
      historyPage: 'Wish → History',
      r5: '5★',
      r4: '4★',
      five: 'five-star',
      fives: 'five-stars',
      currency: 'Primogems',
      pullCost: 160,
    ),
    GameId.starRail: GameInfo(
      name: 'Honkai: Star Rail',
      short: 'Star Rail',
      abbr: 'HSR',
      howTo: 'Open Warp → Records (Warp History) and let the list load.',
      historyPage: 'Warp → Records',
      r5: '5★',
      r4: '4★',
      five: 'five-star',
      fives: 'five-stars',
      currency: 'Stellar Jade',
      pullCost: 160,
    ),
    GameId.zenless: GameInfo(
      name: 'Zenless Zone Zero',
      short: 'Zenless Zone Zero',
      abbr: 'ZZZ',
      howTo: 'Open Signal Search → History and let the list load.',
      historyPage: 'Signal Search → History',
      r5: 'S-rank',
      r4: 'A-rank',
      five: 'S-rank',
      fives: 'S-rank pulls',
      currency: 'Polychromes',
      pullCost: 160,
    ),
  };

  static GameInfo of(GameId game) => _all[game]!;
}

/// The short id the backend stores for a game ("hk4e", "hkrpg", "nap").
String gameKey(GameId game) => switch (game) {
      GameId.genshin => 'hk4e',
      GameId.starRail => 'hkrpg',
      GameId.zenless => 'nap',
    };

GameId? gameFromKey(String? key) => switch (key) {
      'hk4e' => GameId.genshin,
      'hkrpg' => GameId.starRail,
      'nap' => GameId.zenless,
      _ => null,
    };

/// "5★" or "S-rank": the rarity of a pull as the game words it.
String rankLabel(GameId game, int rank) {
  final g = GameInfo.of(game);
  if (rank == 5) return g.r5;
  if (rank == 4) return g.r4;
  return game == GameId.zenless ? 'B-rank' : '3★';
}

/// "UTC+8" for an account's server offset.
String utcLabel(int hours) => 'UTC${hours >= 0 ? '+' : ''}$hours';

/// 12345 → "12,345" (whole numbers need no locale package).
String groupDigits(int n) {
  final s = n.abs().toString();
  final b = StringBuffer(n < 0 ? '-' : '');
  for (var i = 0; i < s.length; i++) {
    if (i > 0 && (s.length - i) % 3 == 0) b.write(',');
    b.write(s[i]);
  }
  return b.toString();
}

String plural(int n, String word, [String? many]) => '${groupDigits(n)} ${n == 1 ? word : (many ?? '${word}s')}';

/// "just now", "5 min ago", "2 h ago", "3 d ago".
String ago(DateTime then, [DateTime? now]) {
  final s = (now ?? DateTime.now()).difference(then).inSeconds;
  if (s < 45) return 'just now';
  final m = (s / 60).round();
  if (m < 60) return '$m min ago';
  final h = (m / 60).round();
  if (h < 48) return '$h h ago';
  return '${(h / 24).round()} d ago';
}
