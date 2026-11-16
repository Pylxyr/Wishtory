import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/materials.dart';
import '../../state/providers.dart';
import '../../state/settings.dart';
import '../../ui/pull_tiles.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

/// Genshin only: the talent books and weapon materials the domains offer today on the account's server, and
/// on any other day of the week. The farming day starts at 04:00 server time, and Sunday opens everything.
class DailyDomains extends ConsumerStatefulWidget {
  const DailyDomains({super.key, required this.account});

  final Account account;

  @override
  ConsumerState<DailyDomains> createState() => _DailyDomainsState();
}

class _DailyDomainsState extends ConsumerState<DailyDomains> {
  /// The weekday being looked at; null follows today.
  int? _picked;
  DateTime _now = DateTime.now();
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    // The countdown and the day itself move on without a refresh.
    _timer = Timer.periodic(const Duration(seconds: 30), (_) {
      if (mounted) setState(() => _now = DateTime.now());
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final data = ref.watch(materialsProvider).asData?.value;
    if (data == null) return const SizedBox.shrink();
    final c = context.wc;
    final today = farmDay(widget.account.timezone, _now);
    final day = _picked ?? today.weekday;
    final isToday = day == today.weekday;
    final subtitle = [
      serverLabel(widget.account.timezone),
      if (isToday) 'new day in ${untilReset(today.msUntilReset)}',
    ].join('  ·  ');
    final books = data.booksOn(day);
    final weapons = data.weaponsOn(day);

    final header = LayoutBuilder(builder: (context, box) {
      final title = Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text(isToday ? 'Today · ${weekdayNames[day - 1]}' : weekdayNames[day - 1], style: Ty.h2),
        const SizedBox(height: 2),
        Text(subtitle, style: Ty.small.copyWith(color: c.muted)),
      ]);
      final picker = SegmentedButton<int>(
        showSelectedIcon: false,
        segments: [
          for (var d = 1; d <= 7; d++)
            ButtonSegment(
              value: d,
              label: Text(weekdayShort[d - 1]),
              icon: d == today.weekday ? const Icon(Icons.today, size: 14) : null,
              tooltip: d == today.weekday ? '${weekdayNames[d - 1]} (today)' : weekdayNames[d - 1],
            ),
        ],
        selected: {day},
        onSelectionChanged: (s) => setState(() => _picked = s.first == today.weekday ? null : s.first),
      );
      if (box.maxWidth < 900) {
        return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [title, const SizedBox(height: 10), picker]);
      }
      return Row(children: [Expanded(child: title), const SizedBox(width: 16), picker]);
    });

    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      const SectionLabel('Farming'),
      Panel(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          header,
          if (day == DateTime.sunday) ...[
            const SizedBox(height: 8),
            Text('On Sundays every domain offers every material, so pick whichever you need.', style: Ty.small.copyWith(color: c.faint)),
          ],
          const SizedBox(height: 14),
          LayoutBuilder(builder: (context, box) {
            final talent = _SeriesColumn(title: 'Talent books', series: books, game: widget.account.game);
            final weapon = _SeriesColumn(title: 'Weapon materials', series: weapons, game: widget.account.game);
            if (box.maxWidth < 1000) {
              return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [talent, const SizedBox(height: 16), weapon]);
            }
            return Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Expanded(child: talent),
              const SizedBox(width: 20),
              Expanded(child: weapon),
            ]);
          }),
        ]),
      ),
    ]);
  }
}

class _SeriesColumn extends StatelessWidget {
  const _SeriesColumn({required this.title, required this.series, required this.game});

  final String title;
  final List<MaterialSeries> series;
  final GameId game;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text(title.toUpperCase(), style: Ty.label.copyWith(color: c.faint)),
      const SizedBox(height: 8),
      if (series.isEmpty)
        Text('Nothing is offered on this day.', style: Ty.body.copyWith(color: c.muted))
      else
        for (final s in series) Padding(padding: const EdgeInsets.only(bottom: 8), child: _SeriesCard(series: s, game: game)),
    ]);
  }
}

class _SeriesCard extends ConsumerWidget {
  const _SeriesCard({required this.series, required this.game});

  final MaterialSeries series;
  final GameId game;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final view = ref.watch(settingsProvider.select((s) => s.pullView));
    final users = series.users;
    final Widget list;
    if (users.isEmpty) {
      list = Text('No characters or weapons listed yet.', style: Ty.small.copyWith(color: c.muted));
    } else {
      list = switch (view) {
        PullView.iconsOnly => Wrap(spacing: 6, runSpacing: 6, children: [
            for (final u in users) Tooltip(message: u.name, child: ItemIcon(game: game, name: u.name, rank: u.rarity, size: 34)),
          ]),
        PullView.namesOnly => TileGrid(minTile: 130, gap: 4, children: [
            for (final u in users) Text(u.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.small.copyWith(color: c.rarity(u.rarity))),
          ]),
        PullView.iconsAndNames => TileGrid(minTile: 160, gap: 6, children: [
            for (final u in users)
              Row(children: [
                ItemIcon(game: game, name: u.name, rank: u.rarity, size: 24),
                const SizedBox(width: 7),
                Expanded(child: Text(u.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.small.copyWith(color: c.text))),
              ]),
          ]),
      };
    }
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(color: c.panel2, borderRadius: BorderRadius.circular(12), border: Border.all(color: c.line)),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(children: [
          Expanded(child: Text(series.name, style: Ty.h3)),
          Text('${daysLabel(series.days)} · Sun', style: Ty.small.copyWith(color: c.faint)),
        ]),
        Text(series.title, style: Ty.small.copyWith(color: c.muted)),
        const SizedBox(height: 10),
        list,
      ]),
    );
  }
}
