import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/first_five.dart';
import '../../domain/game_info.dart';
import '../../state/providers.dart';
import '../../state/settings.dart';
import '../../ui/tokens.dart';

/// Asks how the very first 5★ of a banner went when the history cannot say.
///
/// The game keeps only about six months, so a 5★ at the start of the history may have been a 50/50 win or the
/// guarantee after a loss that is no longer listed. The player knows; "not sure" leaves it unknown. Nothing is asked
/// when that 5★ was a permanent-pool loss (that is certain) or when the banner tracks no 50/50.
class FirstFivePrompt extends ConsumerWidget {
  const FirstFivePrompt({super.key, required this.p, required this.game});
  final BannerPity p;
  final GameId game;

  static String _words(FirstFiveAnswer a) => switch (a) {
        FirstFiveAnswer.win => 'a win',
        FirstFiveAnswer.guaranteed => 'a guarantee',
        FirstFiveAnswer.unsure => 'not sure',
      };

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final first = firstFiveToAsk(p);
    final account = ref.watch(currentAccountProvider);
    if (first == null || account == null) return const SizedBox.shrink();
    final key = firstFiveKey(game, account.uid, p.banner);
    final answer = firstFiveAnswerFor(ref.watch(settingsProvider.select((s) => s.firstFiveAnswers[key])), first);
    final info = GameInfo.of(game);
    final c = context.wc;
    final odds = p.guaranteeOdds!;
    void set(FirstFiveAnswer? a) => ref.read(settingsProvider.notifier).setFirstFiveAnswer(game, account.uid, p.banner, first, a);

    if (answer != null) {
      return Padding(
        padding: const EdgeInsets.only(top: 10),
        child: Wrap(crossAxisAlignment: WrapCrossAlignment.center, spacing: 8, children: [
          Text('Your 1st ${info.r5} here, ${first.name}, is marked as ${_words(answer)}.', style: Ty.small.copyWith(color: c.muted)),
          TextButton(
            style: TextButton.styleFrom(visualDensity: VisualDensity.compact, padding: const EdgeInsets.symmetric(horizontal: 8)),
            onPressed: () => set(null),
            child: const Text('Change'),
          ),
        ]),
      );
    }

    return Padding(
      padding: const EdgeInsets.only(top: 12),
      child: Container(
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(color: c.panel2, borderRadius: BorderRadius.circular(12), border: Border.all(color: c.line2)),
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text('Was your 1st ${info.r5} here a $odds/${100 - odds} win?', style: Ty.h3),
          const SizedBox(height: 4),
          Text(
            '${first.name} (${first.time.substring(0, 10)}) is the oldest ${info.r5} in your history and it was the featured one. '
            'The game only keeps part of your history, so Wishtory cannot see whether it came from the $odds/${100 - odds} or from a guarantee after an older loss. '
            'If you know, say so and the result will be counted; if you are not sure, it stays unknown.',
            style: Ty.small.copyWith(color: c.muted),
          ),
          const SizedBox(height: 10),
          Wrap(spacing: 8, runSpacing: 8, children: [
            FilledButton(onPressed: () => set(FirstFiveAnswer.win), child: Text('Won the $odds/${100 - odds}')),
            OutlinedButton(onPressed: () => set(FirstFiveAnswer.guaranteed), child: const Text('It was guaranteed')),
            OutlinedButton(onPressed: () => set(FirstFiveAnswer.unsure), child: const Text('Not sure')),
          ]),
        ]),
      ),
    );
  }
}
