import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/domain/planning.dart';
import 'package:wishtory/state/refresh.dart';

BannerPity pity({int p = 40, int hard = 90, int? soft = 74, bool? guaranteed, int? odds = 50}) => BannerPity(
      banner: 'character',
      label: 'Character Event',
      pity: p,
      hardPity: hard,
      softPity: soft,
      guaranteed: guaranteed,
      guaranteeOdds: odds,
      total: 100,
      fiveStars: 2,
      fourStars: 10,
      avgFivePity: null,
      avgFourPity: null,
      recentFives: const [],
      recentFours: const [],
    );

void main() {
  test('guarantee: tracked 50/50, not guaranteed doubles the worst case', () {
    final g = pullsToGuarantee(pity(guaranteed: false))!;
    expect(g.fiveStar, 50);
    expect(g.featured, 140);
  });

  test('guarantee: already guaranteed needs only the next 5★', () {
    final g = pullsToGuarantee(pity(guaranteed: true))!;
    expect(g.featured, 50);
  });

  test('guarantee: banners without soft pity or odds', () {
    expect(pullsToGuarantee(pity(soft: null)), isNull);
    expect(pullsToGuarantee(pity(odds: null))!.featured, isNull);
  });

  test('clockMs parses server time and rejects junk', () {
    expect(clockMs('2024-01-01 00:00:00'), DateTime.utc(2024).millisecondsSinceEpoch);
    expect(clockMs('2024-01-01'), isNull);
  });

  test('timeLeft wording', () {
    expect(timeLeft(90 * 60000), '1 h 30 min left');
    expect(timeLeft(2 * 86400000 + 3 * 3600000), '2 d 3 h left');
    expect(timeLeft(10), '1 min left');
  });

  test('progress lines of the same banner replace each other', () {
    var lines = <String>[];
    lines = mergeProgress(lines, 'Character Event: page 1');
    lines = mergeProgress(lines, 'Character Event: page 2');
    lines = mergeProgress(lines, 'Weapon Event: page 1');
    expect(lines, ['Character Event: page 2', 'Weapon Event: page 1']);
  });

  group('live cards', () {
    LiveBanner banner(String pool, String name, List<String> featured, {int msLeft = 1000}) =>
        LiveBanner(id: '$pool-$name', name: name, pool: pool, featured: featured, msLeft: msLeft);

    test('a character banner with two characters becomes one card each, with its own title', () {
      final b = banner('character', 'Myriad Patterns Unfold / Chanson of Many Waters', ['Mitya', 'Furina']);
      final cards = liveCards([b]);
      expect(cards.map((c) => c.title), ['Myriad Patterns Unfold', 'Chanson of Many Waters']);
      expect(cards.map((c) => c.featured), [
        ['Mitya'],
        ['Furina']
      ]);
      // Both cards belong to the one banner, so they share its pity.
      expect(cards.every((c) => identical(c.banner, b)), isTrue);
    });

    test('the weapon banner stays one card with both weapons', () {
      final cards = liveCards([banner('weapon', 'Epitome Invocation', ['Starsealed Edict', 'Splendor of Tranquil Waters'])]);
      expect(cards, hasLength(1));
      expect(cards.single.title, 'Epitome Invocation');
      expect(cards.single.featured, ['Starsealed Edict', 'Splendor of Tranquil Waters']);
    });

    test('a single-character banner is one card', () {
      final cards = liveCards([banner('character', 'Ballad in Goblets', ['Venti'])]);
      expect(cards.map((c) => c.title), ['Ballad in Goblets']);
    });

    test('when the titles do not line up with the characters, each card keeps the whole name', () {
      final cards = liveCards([banner('character', 'One Title Only', ['A', 'B'])]);
      expect(cards.map((c) => c.title), ['One Title Only', 'One Title Only']);
      expect(cards.map((c) => c.featured), [
        ['A'],
        ['B']
      ]);
    });

    test('the order of the banners given is kept, so the three live cards read Mitya, Furina, weapons', () {
      final cards = liveCards([
        banner('character', 'Myriad Patterns Unfold / Chanson of Many Waters', ['Mitya', 'Furina']),
        banner('weapon', 'Epitome Invocation', ['Starsealed Edict', 'Splendor of Tranquil Waters']),
      ]);
      expect(cards.map((c) => c.featured.first), ['Mitya', 'Furina', 'Starsealed Edict']);
    });
  });
}
