import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/features/overview/banner_stats.dart';

FiveStar five(int pity, PullOutcome? outcome) => FiveStar(name: 'F$pity', pity: pity, time: '2026-01-01 00:00:00', offBanner: null, outcome: outcome);
FourStar four(bool character) => FourStar(name: 'x', pity: 3, time: '2026-01-01 00:00:00', isCharacter: character);

BannerPity banner({int total = 200, List<FiveStar> fives = const [], List<FourStar> fours = const [], int? odds = 50}) => BannerPity(
      banner: 'character',
      label: 'Character Event',
      pity: 10,
      hardPity: 90,
      softPity: 74,
      guaranteed: null,
      guaranteeOdds: odds,
      total: total,
      fiveStars: fives.length,
      fourStars: fours.length,
      avgFivePity: fives.isEmpty ? null : fives.fold<int>(0, (a, f) => a + f.pity) / fives.length,
      avgFourPity: fours.isEmpty ? null : 3,
      recentFives: fives,
      recentFours: fours,
    );

void main() {
  group('BannerStats', () {
    test('counts how the 50/50 went and leaves guarantees out of the win rate', () {
      final s = BannerStats.of(banner(fives: [
        five(60, null),
        five(70, PullOutcome.win),
        five(80, PullOutcome.loss),
        five(30, PullOutcome.guaranteed),
        five(75, PullOutcome.win),
      ]));
      expect((s.wins, s.losses, s.guaranteed, s.unclassified), (2, 1, 1, 1));
      expect(s.winRate, closeTo(66.67, 0.01));
      expect(s.tracksResult, isTrue);
    });

    test('a banner without a 50/50 has no result rows and nothing unclassified', () {
      final s = BannerStats.of(banner(odds: null, fives: [five(60, null), five(40, null)]));
      expect(s.tracksResult, isFalse);
      expect(s.unclassified, 0);
      expect(s.winRate, isNull);
    });

    test('shares are of all pulls and the 4★ are split into characters and weapons', () {
      final s = BannerStats.of(banner(total: 200, fives: [five(50, null)], fours: [four(true), four(true), four(false), four(false)]));
      expect((s.fourCharacters, s.fourWeapons), (2, 2));
      expect(s.share(s.fives), 0.5);
      expect(s.share(s.fourCharacters), 1.0);
      expect(percentText(s.share(s.fives)), '0.50%');
    });

    test('no pulls means no percentages', () {
      final s = BannerStats.of(banner(total: 0));
      expect(s.share(0), isNull);
      expect(percentText(null), '–');
      expect(averageText(null), '–');
      expect(averageText(59.65), '59.65');
    });
  });

  group('pity tiers', () {
    test('early, average and late follow soft pity', () {
      expect(pityTier(20, 90, 74), PityTier.early);
      expect(pityTier(44, 90, 74), PityTier.early);
      expect(pityTier(45, 90, 74), PityTier.average);
      expect(pityTier(73, 90, 74), PityTier.average);
      expect(pityTier(74, 90, 74), PityTier.late);
      expect(pityTier(90, 90, 74), PityTier.late);
    });

    test('a banner without soft pity uses most of the hard pity', () {
      expect(pityTier(10, 50, null), PityTier.early);
      expect(pityTier(45, 50, null), PityTier.late);
    });

    test('results and tiers are always put into words', () {
      expect(resultWords(PullOutcome.win, 50), 'Won the 50/50');
      expect(resultWords(PullOutcome.loss, 25), 'Lost the 25/75');
      expect(resultWords(PullOutcome.guaranteed, 50), 'Guaranteed after a loss');
      expect(resultWords(null, null), 'No result tracked');
      expect(resultWords(null, 50), 'Result unknown');
      for (final t in PityTier.values) {
        expect(pityTierWords(t), isNotEmpty);
      }
    });
  });
}
