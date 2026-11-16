import '../bridge/wishtory_api.dart';
import 'game_info.dart';

/// What the player says about the very first 5★ of a banner's history.
///
/// A history that starts mid-way cannot show whether that 5★ was a 50/50 win or a guarantee carried over from a loss
/// before it (the game only keeps about six months). Whichever it was, the 5★ after it is a 50/50, so only the label
/// and the win rate depend on the answer.
enum FirstFiveAnswer {
  win,
  guaranteed,
  unsure;

  static FirstFiveAnswer? parse(String? name) => FirstFiveAnswer.values.asNameMap()[name];
}

/// The settings key of one banner of one account.
String firstFiveKey(GameId game, String uid, String banner) => '${gameKey(game)}|$uid|$banner';

/// A stored answer is "name|time|answer": it belongs to that exact 5★, so older history imported later (a new first
/// 5★) asks the question again instead of reusing an answer about a different pull.
String firstFiveValue(FiveStar first, FirstFiveAnswer answer) => '${first.name}|${first.time}|${answer.name}';

/// The answer stored for [first], or null when none is stored or it was given for another pull.
FirstFiveAnswer? firstFiveAnswerFor(String? stored, FiveStar first) {
  if (stored == null) return null;
  final cut = stored.lastIndexOf('|');
  if (cut < 0) return null;
  if (stored.substring(0, cut) != '${first.name}|${first.time}') return null;
  return FirstFiveAnswer.parse(stored.substring(cut + 1));
}

/// The first 5★ of a banner whose result the history cannot tell: it was the featured item (not a permanent-pool
/// loss) on a banner that tracks a 50/50. Null when there is nothing to ask.
FiveStar? firstFiveToAsk(BannerPity p) {
  if (p.guaranteeOdds == null || p.recentFives.isEmpty) return null;
  final first = p.recentFives.first;
  return first.offBanner == false ? first : null;
}

/// [p] with the answer applied to its first 5★. "Not sure" and no answer leave it as the core returned it (unknown).
BannerPity applyFirstFiveAnswer(BannerPity p, FirstFiveAnswer? answer) {
  final first = firstFiveToAsk(p);
  if (first == null || answer == null || answer == FirstFiveAnswer.unsure) return p;
  final outcome = answer == FirstFiveAnswer.win ? PullOutcome.win : PullOutcome.guaranteed;
  return BannerPity(
    banner: p.banner,
    label: p.label,
    pity: p.pity,
    hardPity: p.hardPity,
    softPity: p.softPity,
    guaranteed: p.guaranteed,
    guaranteeOdds: p.guaranteeOdds,
    total: p.total,
    fiveStars: p.fiveStars,
    fourStars: p.fourStars,
    avgFivePity: p.avgFivePity,
    avgFourPity: p.avgFourPity,
    recentFives: [
      FiveStar(name: first.name, pity: first.pity, time: first.time, offBanner: first.offBanner, outcome: outcome),
      ...p.recentFives.skip(1),
    ],
    recentFours: p.recentFours,
  );
}

/// Every banner of one account with its stored answer applied. [stored] is the settings map of answers.
List<BannerPity> applyFirstFiveAnswers(List<BannerPity> list, GameId game, String uid, Map<String, String> stored) => [
      for (final p in list)
        () {
          final first = firstFiveToAsk(p);
          if (first == null) return p;
          return applyFirstFiveAnswer(p, firstFiveAnswerFor(stored[firstFiveKey(game, uid, p.banner)], first));
        }(),
    ];
