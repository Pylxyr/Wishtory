import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../state/nav.dart';
import '../../state/providers.dart';
import '../../state/refresh.dart';
import '../../domain/game_info.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';
import 'catalog_logic.dart';

/// The one place for game data in Settings: a single version, the last check, **Check for updates**, a clear
/// waiting state, the setting for automatic updates and a short history.
class CatalogCard extends ConsumerWidget {
  const CatalogCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final status = ref.watch(catalogStatusProvider).data;
    final ui = ref.watch(catalogControllerProvider);
    final controller = ref.read(catalogControllerProvider.notifier);
    final waiting = status?.waiting;
    return Panel(
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(children: [
          const Expanded(child: Text('Game catalog', style: Ty.h3)),
          if (waiting != null) StatusChip('Review needed', color: c.gold, icon: Icons.rule_folder_outlined),
        ]),
        const SizedBox(height: 4),
        Text(
          status == null ? 'Loading…' : 'Version ${status.version} · ${lastCheckLabel(status.lastCheck?.toInt())}',
          style: Ty.body.copyWith(color: c.text),
        ),
        const SizedBox(height: 4),
        Text(
          'Item names, banner dates and pity rules arrive together as one signed release. Wishtory checks once a day and keeps working offline with the copy it has. '
          'The check is a plain request for public files; nothing about your history is sent.',
          style: Ty.body.copyWith(color: c.muted),
        ),
        if (waiting != null) ...[
          const SizedBox(height: 12),
          _Waiting(waiting: waiting),
        ],
        const SizedBox(height: 12),
        Wrap(spacing: 8, runSpacing: 8, crossAxisAlignment: WrapCrossAlignment.center, children: [
          OutlinedButton.icon(
            onPressed: ui.busy ? null : () => controller.check(),
            icon: ui.checking
                ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2, semanticsLabel: 'Checking for updates'))
                : const Icon(Icons.refresh, size: 18),
            label: Text(ui.checking ? 'Checking…' : 'Check for updates'),
          ),
          if (waiting != null)
            FilledButton(
              onPressed: ui.busy ? null : () => ref.read(navProvider.notifier).goTo(AppPage.catalogReview),
              child: const Text('Review update'),
            ),
        ]),
        // Announced to screen readers when it appears.
        if (ui.checking)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Semantics(liveRegion: true, child: Text('Checking for a newer catalog. This can take a few seconds.', style: Ty.small.copyWith(color: c.muted))),
          ),
        const Divider(height: 28),
        SwitchListTile(
          contentPadding: EdgeInsets.zero,
          title: const Text('Apply updates that change none of my saved pulls', style: Ty.h3),
          subtitle: Text(
            'An update that would change any saved pull always waits for your review, whatever this says.',
            style: Ty.small.copyWith(color: c.muted),
          ),
          value: status?.autoApply ?? true,
          onChanged: status == null ? null : controller.setAutoApply,
        ),
        const _History(),
      ]),
    );
  }
}

class _Waiting extends StatelessWidget {
  const _Waiting({required this.waiting});
  final WaitingRelease waiting;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(color: c.panel2, borderRadius: BorderRadius.circular(8), border: Border.all(color: c.gold.withValues(alpha: 0.6))),
      child: Semantics(
        container: true,
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text('Version ${waiting.version} is ready and has not been applied.', style: Ty.h3),
          const SizedBox(height: 2),
          Text(
            '${affectedSentence(pulls: waiting.affectedPulls.toInt(), accounts: waiting.affectedAccounts.toInt())} '
            '(${countsLabel(major: waiting.major.toInt(), minor: waiting.minor.toInt(), info: waiting.info.toInt())}.)',
            style: Ty.body.copyWith(color: c.muted),
          ),
        ]),
      ),
    );
  }
}

/// The last few applied releases.
class _History extends ConsumerWidget {
  const _History();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final history = ref.watch(catalogHistoryProvider).value ?? const <CatalogHistoryEntry>[];
    return ExpansionTile(
      tilePadding: EdgeInsets.zero,
      childrenPadding: EdgeInsets.zero,
      title: const Text('Catalog history', style: Ty.h3),
      subtitle: Text(history.isEmpty ? 'No updates applied yet on this PC.' : 'The last ${history.length > 5 ? 5 : history.length} applied updates.', style: Ty.small.copyWith(color: c.muted)),
      children: [
        for (final e in history.take(5))
          ListTile(
            dense: true,
            contentPadding: EdgeInsets.zero,
            title: Text('Version ${e.version}', style: Ty.body),
            subtitle: Text(
              '${ago(DateTime.fromMillisecondsSinceEpoch(e.appliedAt.toInt() * 1000))} · '
              '${e.automatic ? 'applied automatically' : 'applied after your review'} · '
              '${e.affectedPulls == 0 ? 'no saved pull changed' : '${pullsWord(e.affectedPulls.toInt())} changed'} · '
              '${e.changesTotal} ${e.changesTotal == 1 ? 'change' : 'changes'}',
              style: Ty.small.copyWith(color: c.muted),
            ),
          ),
      ],
    );
  }
}
