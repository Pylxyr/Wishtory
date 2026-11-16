import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/domain/first_five.dart';

FiveStar five(String name, String time, {bool? off = false, PullOutcome? outcome}) =>
    FiveStar(name: name, pity: 40, time: time, offBanner: off, outcome: outcome);

BannerPity banner(List<FiveStar> fives, {int? odds = 50, String key = 'character'}) => BannerPity(
      banner: key,
      label: 'Character Event',
      pity: 3,
      hardPity: 90,
      guaranteeOdds: odds,
      guaranteed: false,
      total: 100,
      fiveStars: fives.length,
      fourStars: 0,
      recentFives: fives,
      recentFours: const [],
    );

void main() {
  final first = five('Mistsplitter Reforged', '2026-01-02 10:00:00');
  final later = five('Amos', '2026-02-02 10:00:00', off: true, outcome: PullOutcome.loss);

  group('firstFiveToAsk', () {
    test('asks about a featured first 5★ on a banner that tracks a 50/50', () {
      expect(firstFiveToAsk(banner([first, later])), first);
    });
    test('a permanent-pool first 5★ is a certain loss, so nothing is asked', () {
      expect(firstFiveToAsk(banner([five('Amos', '2026-01-02 10:00:00', off: true, outcome: PullOutcome.loss)])), isNull);
    });
    test('nothing is asked where no 50/50 is tracked or there is no 5★', () {
      expect(firstFiveToAsk(banner([first], odds: null)), isNull);
      expect(firstFiveToAsk(banner([])), isNull);
    });
  });

  group('applyFirstFiveAnswer', () {
    test('a win or guarantee answer sets the first 5★ and nothing else', () {
      final p = banner([first, later]);
      final won = applyFirstFiveAnswer(p, FirstFiveAnswer.win);
      expect(won.recentFives.first.outcome, PullOutcome.win);
      expect(won.recentFives.last, later);
      expect(won.guaranteed, p.guaranteed);
      expect(applyFirstFiveAnswer(p, FirstFiveAnswer.guaranteed).recentFives.first.outcome, PullOutcome.guaranteed);
    });
    test('not sure and no answer leave it unknown', () {
      final p = banner([first]);
      expect(applyFirstFiveAnswer(p, FirstFiveAnswer.unsure).recentFives.first.outcome, isNull);
      expect(applyFirstFiveAnswer(p, null).recentFives.first.outcome, isNull);
    });
  });

  group('stored answers', () {
    test('an answer belongs to the exact 5★ it was given for', () {
      final stored = firstFiveValue(first, FirstFiveAnswer.guaranteed);
      expect(firstFiveAnswerFor(stored, first), FirstFiveAnswer.guaranteed);
      // Older history imported later gives a different first 5★: the question is asked again.
      expect(firstFiveAnswerFor(stored, five('Other', '2025-12-01 10:00:00')), isNull);
      expect(firstFiveAnswerFor(null, first), isNull);
      expect(firstFiveAnswerFor('garbage', first), isNull);
    });

    test('answers are applied per banner of the account', () {
      final list = [banner([first, later]), banner([five('Weapon', '2026-01-03 10:00:00')], key: 'weapon')];
      final stored = {
        firstFiveKey(GameId.genshin, '800', 'character'): firstFiveValue(first, FirstFiveAnswer.win),
      };
      final out = applyFirstFiveAnswers(list, GameId.genshin, '800', stored);
      expect(out[0].recentFives.first.outcome, PullOutcome.win);
      expect(out[1].recentFives.first.outcome, isNull, reason: 'no answer for the weapon banner yet');
      // Another account's answers do not apply.
      expect(applyFirstFiveAnswers(list, GameId.genshin, '999', stored)[0].recentFives.first.outcome, isNull);
    });
  });
}
