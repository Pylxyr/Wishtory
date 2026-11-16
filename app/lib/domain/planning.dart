import '../bridge/generated/api/types.dart';
import 'game_info.dart';

class Guarantee {
  const Guarantee(this.fiveStar, this.featured);

  /// Pulls to the next 5★ at worst.
  final int fiveStar;

  /// Pulls to a guaranteed *featured* 5★ at worst; null on banners that track no 50/50.
  final int? featured;
}

/// Worst case under the rules Wishtory tracks: how many more pulls guarantee a five-star and, on
/// banners that track the 50/50 or 75/25, a guaranteed featured one. Only banners with a soft-pity
/// ladder qualify. Estimates, not promises. (Port of `pullsToGuarantee`.)
Guarantee? pullsToGuarantee(BannerPity p) {
  if (p.softPity == null || p.hardPity <= 0) return null;
  final toFive = (p.hardPity - p.pity).clamp(0, p.hardPity);
  if (p.guaranteeOdds == null) return Guarantee(toFive, null);
  return Guarantee(toFive, p.guaranteed == true ? toFive : toFive + p.hardPity);
}

final _clock = RegExp(r'^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})$');

/// "YYYY-MM-DD HH:MM:SS" (a server wall-clock time, no zone) as milliseconds on a plain UTC axis.
int? clockMs(String s) {
  final m = _clock.firstMatch(s);
  if (m == null) return null;
  int g(int i) => int.parse(m.group(i)!);
  return DateTime.utc(g(1), g(2), g(3), g(4), g(5), g(6)).millisecondsSinceEpoch;
}

/// The server's wall clock now, on the same axis as [clockMs], for an account at [tzHours].
int serverClockMs(int tzHours, [DateTime? now]) => (now ?? DateTime.now()).millisecondsSinceEpoch + tzHours * 3600000;

class LiveBanner {
  const LiveBanner({
    required this.id,
    required this.name,
    required this.pool,
    required this.featured,
    required this.msLeft,
  });

  final String id, name, pool;
  final List<String> featured;
  final int msLeft;
}

/// Event banners running right now, soonest ending first. Start and end are inclusive
/// server-local times, like each pull's time.
List<LiveBanner> liveBanners(BannerList? set, GameId game, int tzHours, GameCatalog? catalog, [DateTime? now]) {
  if (set == null) return const [];
  final clock = serverClockMs(tzHours, now);
  final key = gameKey(game);
  final names = <String, String>{};
  for (final item in catalog?.items ?? const <CatalogItem>[]) {
    if (item.game == key) names[item.id] = item.name;
  }
  final out = <LiveBanner>[];
  for (final b in set.banners) {
    if (b.game != game) continue;
    final start = clockMs(b.start);
    final end = clockMs(b.end);
    if (start == null || end == null || clock < start || clock > end + 999) continue;
    out.add(LiveBanner(
      id: b.id,
      name: b.name,
      pool: b.pool,
      featured: [
        for (final id in b.featured5)
          if (names[id] != null) names[id]!,
      ],
      msLeft: end + 1000 - clock,
    ));
  }
  out.sort((a, b) => a.msLeft.compareTo(b.msLeft));
  return out;
}

/// One card of the Live now row. The game runs a character banner that features two characters as two
/// banners that share one pity, so it gets two cards, each with its own title and picture. Any other banner,
/// the weapon banner included, is one card.
class LiveCard {
  const LiveCard({required this.banner, required this.title, required this.featured});

  /// The running banner this card belongs to: its pool and time left, and so the pity it shows.
  final LiveBanner banner;

  /// What the card is called, and the items it features.
  final String title;
  final List<String> featured;
}

/// The cards for [live], in the order given; a split banner's cards keep the order of its featured items.
List<LiveCard> liveCards(List<LiveBanner> live) {
  final out = <LiveCard>[];
  for (final b in live) {
    if (b.pool == 'character' && b.featured.length > 1) {
      // Banner names list one title per featured character, in the same order, joined by " / ". When the
      // counts do not line up, every card carries the whole name rather than a guess.
      final titles = b.name.split(' / ');
      final one = titles.length == b.featured.length;
      for (var i = 0; i < b.featured.length; i++) {
        out.add(LiveCard(banner: b, title: one ? titles[i].trim() : b.name, featured: [b.featured[i]]));
      }
    } else {
      out.add(LiveCard(banner: b, title: b.name, featured: b.featured));
    }
  }
  return out;
}

/// "6 d 4 h left", "5 h 12 min left", "9 min left".
String timeLeft(int ms) {
  final min = (ms ~/ 60000) < 1 ? 1 : ms ~/ 60000;
  final d = min ~/ 1440;
  final h = (min % 1440) ~/ 60;
  if (d >= 1) return '$d d $h h left';
  if (h >= 1) return '$h h ${min % 60} min left';
  return '$min min left';
}
