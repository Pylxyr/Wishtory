import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

/// Which Help topic should be open; set by the "Open help" button of an error card.
class HelpTopic extends Notifier<String?> {
  @override
  String? build() => null;
  void open(String topic) => state = topic;
}

final helpTopicProvider = NotifierProvider<HelpTopic, String?>(HelpTopic.new);

class _Topic {
  const _Topic(this.id, this.title, this.body);
  final String id, title, body;
}

const _topics = [
  _Topic(
    'getting-started',
    'Getting started',
    'Open your game\'s wish / warp / signal history page and wait until the list loads. Then press Refresh in Wishtory. '
        'Wishtory reads the link the game saved in its own cache (read-only) and downloads your history from HoYoverse. '
        'Do this regularly: the game only keeps the last six months.',
  ),
  _Topic(
    'refresh',
    'Refresh says the link is missing or expired',
    'The link only exists after the history page was opened, and it expires after a short while.\n'
        '1. Keep the game open.\n2. Open the history page again and wait until the list loads.\n3. Press Refresh again.',
  ),
  _Topic(
    'paste-link',
    'Pasting a link by hand',
    'Use the arrow next to Refresh → "Paste a link…". Paste the whole link: it starts with https:// and contains authkey=. '
        'The link is used once and is never saved or logged.',
  ),
  _Topic(
    'game-not-found',
    'The game was not found',
    'Launch the game at least once so it creates its log file. On Linux, add the folder or Wine/Proton prefix under Settings → Game locations. '
        'Wishtory never writes to the game folder.',
  ),
  _Topic(
    'pity',
    'How pity and the 50/50 are counted',
    'Pity is the number of pulls since your last 5★ on that banner type. Soft pity (where odds rise) and hard pity (guaranteed) come from signed rules that can be updated without a new app version. '
        'A 5★ counts as lost only when it is in the permanent pool and was not featured on the banner you pulled on. Banners without a clear featured item track no result.',
  ),
  _Topic(
    'import-export',
    'Moving history between trackers',
    'Import reads UIGF v4.x, older UIGF, SRGF and Paimon.moe backups. Importing the same file twice does not duplicate pulls. Export supports UIGF v4.2 (recommended), UIGF v3.0, SRGF v1.0 and Paimon.moe.',
  ),
  _Topic(
    'privacy',
    'What leaves this PC',
    'Only HoYoverse\'s own history service is contacted with your link, to download your history. Wishtory also fetches signed catalog and banner files and checks GitHub for new versions, and loads item pictures from Enka.Network. '
        'No account or wish data is sent to any of them. Everything you save stays in Wishtory\'s folder on this PC.',
  ),
];

class HelpPage extends ConsumerStatefulWidget {
  const HelpPage({super.key});

  @override
  ConsumerState<HelpPage> createState() => _HelpPageState();
}

class _HelpPageState extends ConsumerState<HelpPage> {
  String _query = '';

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final open = ref.watch(helpTopicProvider);
    final q = _query.trim().toLowerCase();
    final shown = [
      for (final t in _topics)
        if (q.isEmpty || t.title.toLowerCase().contains(q) || t.body.toLowerCase().contains(q)) t,
    ];
    return PageBody(maxWidth: 800, children: [
      const Text('Help', style: Ty.h1),
      const SizedBox(height: 12),
      TextField(decoration: fieldDecoration(context, hint: 'Search help', prefix: const Icon(Icons.search, size: 18)), onChanged: (v) => setState(() => _query = v)),
      const SizedBox(height: 16),
      if (shown.isEmpty) Text('No topic matches "$_query".', style: Ty.body.copyWith(color: c.muted)),
      for (final t in shown)
        Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: Panel(
            padding: EdgeInsets.zero,
            child: Theme(
              data: Theme.of(context).copyWith(dividerColor: Colors.transparent),
              child: ExpansionTile(
                key: PageStorageKey('help-${t.id}-${open == t.id}'),
                initiallyExpanded: open == t.id,
                title: Text(t.title, style: Ty.h3),
                childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
                expandedCrossAxisAlignment: CrossAxisAlignment.start,
                children: [Text(t.body, style: Ty.body.copyWith(color: c.muted))],
              ),
            ),
          ),
        ),
    ]);
  }
}
