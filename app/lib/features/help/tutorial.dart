import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../state/settings.dart';
import '../../ui/tokens.dart';

const _steps = [
  ('Open the history page in the game', 'In your game, open the wish / warp / signal history and wait until the list loads. This makes the game save a link Wishtory can use.'),
  ('Press Refresh', 'Wishtory reads that link from the game\'s cache (read-only) and downloads your history. Nothing in the game is changed.'),
  ('Read your pity at a glance', 'The Overview shows pity per banner, whether your next 5★ is guaranteed, and how lucky your recent 5★ were. History has every pull.'),
];

Future<void> showTutorial(BuildContext context, WidgetRef ref) {
  return showDialog<void>(
    context: context,
    builder: (ctx) => const _Tutorial(),
  ).whenComplete(() => ref.read(settingsProvider.notifier).finishTutorial());
}

class _Tutorial extends StatefulWidget {
  const _Tutorial();

  @override
  State<_Tutorial> createState() => _TutorialState();
}

class _TutorialState extends State<_Tutorial> {
  int _i = 0;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final (title, body) = _steps[_i];
    final last = _i == _steps.length - 1;
    return AlertDialog(
      backgroundColor: c.panel,
      title: Text('${_i + 1} of ${_steps.length}', style: Ty.label.copyWith(color: c.faint)),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420),
        child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(title, style: Ty.h2),
          const SizedBox(height: 8),
          Text(body, style: Ty.body.copyWith(color: c.muted)),
        ]),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Skip')),
        if (_i > 0) TextButton(onPressed: () => setState(() => _i--), child: const Text('Back')),
        FilledButton(autofocus: true, onPressed: () => last ? Navigator.pop(context) : setState(() => _i++), child: Text(last ? 'Done' : 'Next')),
      ],
    );
  }
}
