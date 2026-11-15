import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../state/settings.dart';

/// The two display choices that apply to every list of pulls in every game: what each pull shows
/// (icon and name, name only, icon only) and which end of time comes first. Both are remembered.
class ViewControls extends ConsumerWidget {
  const ViewControls({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final view = ref.watch(settingsProvider.select((s) => s.pullView));
    final newest = ref.watch(settingsProvider.select((s) => s.newestFirst));
    final prefs = ref.read(settingsProvider.notifier);
    return Wrap(spacing: 10, runSpacing: 8, crossAxisAlignment: WrapCrossAlignment.center, children: [
      SegmentedButton<PullView>(
        showSelectedIcon: false,
        segments: const [
          ButtonSegment(value: PullView.iconsAndNames, label: Text('Icons + names'), icon: Icon(Icons.view_list_outlined, size: 16), tooltip: 'Show the icon and the name'),
          ButtonSegment(value: PullView.namesOnly, label: Text('Names'), icon: Icon(Icons.text_fields, size: 16), tooltip: 'Names only: fits the most on screen'),
          ButtonSegment(value: PullView.iconsOnly, label: Text('Icons'), icon: Icon(Icons.grid_view_outlined, size: 16), tooltip: 'Icons only: hover one for its name'),
        ],
        selected: {view},
        onSelectionChanged: (s) => prefs.setPullView(s.first),
      ),
      SegmentedButton<bool>(
        showSelectedIcon: false,
        segments: const [
          ButtonSegment(value: true, label: Text('Newest first'), icon: Icon(Icons.arrow_downward, size: 16), tooltip: 'Most recent pull first'),
          ButtonSegment(value: false, label: Text('Oldest first'), icon: Icon(Icons.arrow_upward, size: 16), tooltip: 'First pull first'),
        ],
        selected: {newest},
        onSelectionChanged: (s) => prefs.setNewestFirst(s.first),
      ),
    ]);
  }
}
