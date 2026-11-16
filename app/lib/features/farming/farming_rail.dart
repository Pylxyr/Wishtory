import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/icon_cache.dart';
import '../../domain/icons.dart';
import '../../domain/materials.dart';
import '../../state/nav.dart';
import '../../state/providers.dart';
import '../../state/settings.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

/// Width of the open panel, and of the strip it leaves behind when it is closed.
const double _openWidth = 320;
const double _stripWidth = 48;

/// Genshin only: the talent books and weapon materials the domains offer today on the account's server, and
/// on any other day of the week, in a panel beside the navigation rail. Closing it leaves a thin strip and
/// gives the page the room; the choice is remembered. The farming day starts at 04:00 server time, and
/// Sunday opens everything.
class FarmingRail extends ConsumerStatefulWidget {
  const FarmingRail({super.key, required this.account});

  final Account account;

  @override
  ConsumerState<FarmingRail> createState() => _FarmingRailState();
}

class _FarmingRailState extends ConsumerState<FarmingRail> {
  /// The weekday being looked at; null follows today.
  int? _picked;

  /// Weapon materials instead of talent books.
  bool _weapons = false;

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

  void _setOpen(bool open) => ref.read(settingsProvider.notifier).setFarmingOpen(open);

  @override
  Widget build(BuildContext context) {
    final open = ref.watch(settingsProvider.select((s) => s.farmingOpen));
    final today = farmDay(widget.account.timezone, _now);
    if (!open) return _Strip(weekday: today.weekday, onOpen: () => _setOpen(true));

    final c = context.wc;
    final day = _picked ?? today.weekday;
    return Container(
      width: _openWidth,
      decoration: BoxDecoration(color: c.rail, border: Border(right: BorderSide(color: c.line))),
      child: Column(children: [
        // Same height and hairline as the other title bars, so they line up across the window.
        Container(
          height: kBarHeight,
          padding: const EdgeInsets.fromLTRB(16, 0, 6, 0),
          decoration: BoxDecoration(border: Border(bottom: BorderSide(color: c.line))),
          child: Row(children: [
            Icon(Icons.event_note_outlined, size: 19, color: c.accent),
            const SizedBox(width: 9),
            const Expanded(child: Text('Farming', style: TextStyle(fontSize: 17, fontWeight: FontWeight.w600, letterSpacing: -0.3))),
            IconButton(
              tooltip: 'Hide farming',
              onPressed: () => _setOpen(false),
              icon: const Icon(Icons.keyboard_double_arrow_left, size: 20),
            ),
          ]),
        ),
        Expanded(
          child: ref.watch(materialsProvider).when(
                loading: () => const SizedBox.shrink(),
                error: (_, _) => Padding(
                  padding: const EdgeInsets.all(16),
                  child: Text('The farming list could not be loaded.', style: Ty.body.copyWith(color: c.muted)),
                ),
                data: (data) => _body(context, data, today, day),
              ),
        ),
      ]),
    );
  }

  Widget _body(BuildContext context, MaterialsData data, FarmDay today, int day) {
    final c = context.wc;
    final isToday = day == today.weekday;
    final books = data.booksOn(day);
    final weapons = data.weaponsOn(day);
    final series = _weapons ? weapons : books;
    return ListView(
      padding: const EdgeInsets.fromLTRB(12, 14, 12, 16),
      children: [
        Text(isToday ? 'Today · ${weekdayNames[day - 1]}' : weekdayNames[day - 1], style: Ty.h2),
        const SizedBox(height: 2),
        Text(serverLabel(widget.account.timezone), style: Ty.small.copyWith(color: c.muted)),
        if (isToday) Text('New day in ${untilReset(today.msUntilReset)}', style: Ty.small.copyWith(color: c.muted)),
        const SizedBox(height: 12),
        Row(children: [
          for (var d = 1; d <= 7; d++) ...[
            if (d > 1) const SizedBox(width: 4),
            Expanded(
              child: _Choice(
                label: weekdayShort[d - 1],
                semantics: d == today.weekday ? '${weekdayNames[d - 1]} (today)' : weekdayNames[d - 1],
                selected: d == day,
                marked: d == today.weekday,
                onTap: () => setState(() => _picked = d == today.weekday ? null : d),
              ),
            ),
          ],
        ]),
        if (day == DateTime.sunday)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Text('On Sundays every domain offers every material, so pick whichever you need.', style: Ty.small.copyWith(color: c.faint)),
          ),
        const SizedBox(height: 12),
        Row(children: [
          Expanded(
            child: _Choice(
              label: 'Talent books · ${books.length}',
              semantics: 'Talent books, ${books.length} offered',
              icon: Icons.menu_book_outlined,
              selected: !_weapons,
              onTap: () => setState(() => _weapons = false),
            ),
          ),
          const SizedBox(width: 4),
          Expanded(
            child: _Choice(
              label: 'Weapons · ${weapons.length}',
              semantics: 'Weapon materials, ${weapons.length} offered',
              icon: Icons.hardware_outlined,
              selected: _weapons,
              onTap: () => setState(() => _weapons = true),
            ),
          ),
        ]),
        const SizedBox(height: 12),
        if (series.isEmpty)
          Text('Nothing is offered on this day.', style: Ty.body.copyWith(color: c.muted))
        else
          for (final s in series) Padding(padding: const EdgeInsets.only(bottom: 8), child: _SeriesCard(series: s, game: widget.account.game)),
        const SizedBox(height: 4),
        const _Credit(),
      ],
    );
  }
}

/// What is left of the panel when it is closed: a thin strip that opens it again.
class _Strip extends StatelessWidget {
  const _Strip({required this.weekday, required this.onOpen});

  final int weekday;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    return Semantics(
      button: true,
      label: 'Show farming schedule',
      child: Tooltip(
        message: 'Show farming',
        child: Material(
          color: c.rail,
          child: InkWell(
            onTap: onOpen,
            child: Container(
              width: _stripWidth,
              decoration: BoxDecoration(border: Border(right: BorderSide(color: c.line))),
              child: Column(children: [
                Container(
                  height: kBarHeight,
                  alignment: Alignment.center,
                  decoration: BoxDecoration(border: Border(bottom: BorderSide(color: c.line))),
                  child: Icon(Icons.keyboard_double_arrow_right, size: 20, color: c.muted),
                ),
                const SizedBox(height: 16),
                RotatedBox(
                  quarterTurns: 3,
                  child: Text('FARMING · ${weekdayShort[weekday - 1].toUpperCase()}', style: Ty.label.copyWith(color: c.faint)),
                ),
              ]),
            ),
          ),
        ),
      ),
    );
  }
}

/// A small selectable button: the weekdays and the talent books / weapons switch. [marked] is today's weekday.
class _Choice extends StatelessWidget {
  const _Choice({
    required this.label,
    required this.semantics,
    required this.selected,
    required this.onTap,
    this.marked = false,
    this.icon,
  });

  final String label;
  final String semantics;
  final bool selected;
  final bool marked;
  final IconData? icon;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final fg = selected ? c.accentInk : (marked ? c.accent : c.muted);
    final radius = BorderRadius.circular(8);
    return Semantics(
      button: true,
      selected: selected,
      label: semantics,
      child: ExcludeSemantics(
        child: Material(
          color: selected ? c.accent : c.panel2,
          borderRadius: radius,
          child: InkWell(
            borderRadius: radius,
            onTap: onTap,
            child: Container(
              height: 34,
              alignment: Alignment.center,
              decoration: BoxDecoration(borderRadius: radius, border: Border.all(color: selected || marked ? c.accent : c.line)),
              child: Row(mainAxisSize: MainAxisSize.min, children: [
                if (icon != null) ...[Icon(icon, size: 15, color: fg), const SizedBox(width: 5)],
                Flexible(child: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.small.copyWith(color: fg, fontWeight: FontWeight.w600))),
              ]),
            ),
          ),
        ),
      ),
    );
  }
}

/// One series of materials: its picture and name, the days it is offered, and who uses it.
class _SeriesCard extends ConsumerWidget {
  const _SeriesCard({required this.series, required this.game});

  final MaterialSeries series;
  final GameId game;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final view = ref.watch(settingsProvider.select((s) => s.pullView));
    final users = series.users;
    // Names wrap and flow instead of sitting in fixed columns, so none is cut short and nothing is left uneven.
    final Widget list;
    if (users.isEmpty) {
      list = Text('No characters or weapons listed yet.', style: Ty.small.copyWith(color: c.muted));
    } else {
      list = switch (view) {
        PullView.iconsOnly => Wrap(spacing: 6, runSpacing: 6, children: [
            for (final u in users) Tooltip(message: u.name, child: ItemIcon(game: game, name: u.name, rank: u.rarity, size: 32)),
          ]),
        PullView.namesOnly => Wrap(spacing: 12, runSpacing: 4, children: [
            for (final u in users) Text(u.name, style: Ty.small.copyWith(color: c.rarity(u.rarity))),
          ]),
        PullView.iconsAndNames => Wrap(spacing: 6, runSpacing: 6, children: [
            for (final u in users) _UserChip(game: game, user: u),
          ]),
      };
    }
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(color: c.panel2, borderRadius: BorderRadius.circular(12), border: Border.all(color: c.line)),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
          _MaterialPicture(seriesId: series.id, size: 46),
          const SizedBox(width: 10),
          Expanded(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Text(series.name, style: Ty.h3),
              Text(series.title, style: Ty.small.copyWith(color: c.muted)),
              Text('${daysLabel(series.days)} · Sun', style: Ty.small.copyWith(color: c.faint)),
            ]),
          ),
        ]),
        const SizedBox(height: 10),
        list,
      ]),
    );
  }
}

/// A character or weapon that uses a series: its icon and its name, on a rounded chip.
class _UserChip extends StatelessWidget {
  const _UserChip({required this.game, required this.user});

  final GameId game;
  final MaterialUser user;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    return Container(
      padding: const EdgeInsets.fromLTRB(3, 3, 10, 3),
      decoration: BoxDecoration(color: c.panel3.withValues(alpha: 0.55), borderRadius: BorderRadius.circular(99), border: Border.all(color: c.line)),
      child: Row(mainAxisSize: MainAxisSize.min, children: [
        ItemIcon(game: game, name: user.name, rank: user.rarity, size: 24),
        const SizedBox(width: 6),
        Flexible(child: Text(user.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.small.copyWith(color: c.text))),
      ]),
    );
  }
}

/// The picture of a series (its lowest-tier book or ore), from the disk cache once it has downloaded.
class _MaterialPicture extends ConsumerWidget {
  const _MaterialPicture({required this.seriesId, required this.size});

  final String seriesId;
  final double size;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final urls = ref.watch(iconTableProvider).materialUrls(seriesId);
    final fallback = Icon(Icons.inventory_2_outlined, size: size * 0.5, color: c.faint);
    final file = urls.isEmpty
        ? null
        : switch (ref.watch(iconFileProvider(IconRequest(urls)))) {
            AsyncData(:final value) => value,
            _ => null,
          };
    return Container(
      width: size,
      height: size,
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(color: c.panel3.withValues(alpha: 0.55), borderRadius: BorderRadius.circular(10)),
      child: file == null
          ? Center(child: fallback)
          : Image.file(
              file,
              fit: BoxFit.contain,
              cacheWidth: (size * MediaQuery.devicePixelRatioOf(context)).round(),
              gaplessPlayback: true,
              excludeFromSemantics: true,
              errorBuilder: (_, _, _) => Center(child: fallback),
            ),
    );
  }
}

/// Where the schedule and its pictures come from, with a way to the full credits.
class _Credit extends ConsumerWidget {
  const _Credit();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text('Schedule and pictures: Paimon.moe. Item icons: Enka.Network.', style: Ty.small.copyWith(color: c.faint)),
      TextButton(
        onPressed: () => ref.read(navProvider.notifier).goTo(AppPage.about),
        style: TextButton.styleFrom(
          padding: EdgeInsets.zero,
          minimumSize: const Size(0, 28),
          alignment: Alignment.centerLeft,
          tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        ),
        child: const Text('Credits and sources'),
      ),
    ]);
  }
}
