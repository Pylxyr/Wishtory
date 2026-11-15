import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';
import '../../state/settings.dart';
import '../../ui/density.dart';
import '../../ui/pull_tiles.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

// ------------------------------------------------------------------ numbers

/// Everything the statistics table shows for one banner, worked out from what the core already returns.
class BannerStats {
  const BannerStats({
    required this.total,
    required this.fives,
    required this.fours,
    required this.avgFive,
    required this.avgFour,
    required this.wins,
    required this.losses,
    required this.guaranteed,
    required this.unclassified,
    required this.fourCharacters,
    required this.fourWeapons,
    required this.tracksResult,
  });

  factory BannerStats.of(BannerPity p) {
    var wins = 0, losses = 0, guaranteed = 0, unclassified = 0;
    for (final f in p.recentFives) {
      switch (f.outcome) {
        case PullOutcome.win:
          wins++;
        case PullOutcome.loss:
          losses++;
        case PullOutcome.guaranteed:
          guaranteed++;
        case null:
          unclassified++;
      }
    }
    final characters = p.recentFours.where((f) => f.isCharacter).length;
    return BannerStats(
      total: p.total,
      fives: p.fiveStars,
      fours: p.fourStars,
      avgFive: p.avgFivePity,
      avgFour: p.avgFourPity,
      wins: wins,
      losses: losses,
      guaranteed: guaranteed,
      unclassified: p.guaranteeOdds == null ? 0 : unclassified,
      fourCharacters: characters,
      fourWeapons: p.recentFours.length - characters,
      tracksResult: p.guaranteeOdds != null,
    );
  }

  final int total, fives, fours, wins, losses, guaranteed, unclassified, fourCharacters, fourWeapons;
  final double? avgFive, avgFour;

  /// Whether this banner has a 50/50 (or 75/25) to track.
  final bool tracksResult;

  /// Share of all pulls, in percent; null when there are no pulls.
  double? share(int count) => total == 0 ? null : count * 100 / total;

  /// Won / (won + lost): the guaranteed 5★ that follow a loss are not a coin flip, so they are left out.
  double? get winRate => wins + losses == 0 ? null : wins * 100 / (wins + losses);
}

String percentText(double? value) => value == null ? '–' : '${value.toStringAsFixed(2)}%';
String averageText(double? value) => value == null ? '–' : value.toStringAsFixed(2);

// ------------------------------------------------------------------ pity tiers

enum PityTier { early, average, late }

/// How lucky a 5★ was, by how deep into the pity counter it came: early is well before soft pity, late is at or past it.
PityTier pityTier(int pity, int hard, int? soft) {
  final edge = soft ?? (hard * 0.8).round();
  if (pity <= edge * 0.6) return PityTier.early;
  if (pity < edge) return PityTier.average;
  return PityTier.late;
}

String pityTierWords(PityTier t) => switch (t) {
      PityTier.early => 'Early',
      PityTier.average => 'Average',
      PityTier.late => 'Late',
    };

Color pityTierColor(WishColors c, PityTier t) => switch (t) {
      PityTier.early => c.ok,
      PityTier.average => c.accent,
      PityTier.late => c.err,
    };

/// The result of a 5★ in words, for tooltips and screen readers (never colour alone).
String resultWords(PullOutcome? outcome, int? odds) {
  final chance = odds == null ? '50/50' : '$odds/${100 - odds}';
  return switch (outcome) {
    PullOutcome.win => 'Won the $chance',
    PullOutcome.loss => 'Lost the $chance',
    PullOutcome.guaranteed => 'Guaranteed after a loss',
    null => odds == null ? 'No result tracked' : 'Result unknown',
  };
}

// ------------------------------------------------------------------ statistics table

/// A compact table: totals, share of pulls and average pity for 5★ and 4★, with how the 50/50 went and what the 4★ were.
class StatsTable extends StatelessWidget {
  const StatsTable({super.key, required this.p, required this.game});
  final BannerPity p;
  final GameId game;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final d = Density.of(context);
    final info = GameInfo.of(game);
    final s = BannerStats.of(p);
    final chance = p.guaranteeOdds == null ? '' : '${p.guaranteeOdds}/${100 - p.guaranteeOdds!}';

    TableRow row(String label, String total, String share, String avg, {bool sub = false, Color? color, bool strong = false}) {
      final style = (sub ? Ty.body : Ty.h3).copyWith(color: color ?? c.text, fontWeight: strong ? FontWeight.w700 : null);
      Widget cell(String text, {bool left = false}) => Padding(
            padding: EdgeInsets.symmetric(vertical: d.rowPad),
            child: Text(text, textAlign: left ? TextAlign.left : TextAlign.right, style: left ? style : style.copyWith(fontFeatures: const [FontFeature.tabularFigures()])),
          );
      return TableRow(children: [
        Padding(
          padding: EdgeInsets.only(left: sub ? 14 : 0, top: d.rowPad, bottom: d.rowPad),
          child: Text(label, style: style),
        ),
        cell(total),
        cell(share),
        cell(avg),
      ]);
    }

    final head = Ty.label.copyWith(color: c.faint);
    return Semantics(
      container: true,
      label: 'Statistics for ${p.label}',
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Table(
          columnWidths: const {0: FlexColumnWidth(), 1: FixedColumnWidth(56), 2: FixedColumnWidth(72), 3: FixedColumnWidth(72)},
          defaultVerticalAlignment: TableCellVerticalAlignment.middle,
          border: TableBorder(horizontalInside: BorderSide(color: c.line)),
          children: [
            TableRow(children: [
              Padding(padding: EdgeInsets.only(bottom: d.rowPad), child: Text('STATISTICS', style: head)),
              Text('TOTAL', textAlign: TextAlign.right, style: head),
              Text('SHARE', textAlign: TextAlign.right, style: head),
              Text('AVG PITY', textAlign: TextAlign.right, style: head),
            ]),
            row(info.r5, groupDigits(s.fives), percentText(s.share(s.fives)), averageText(s.avgFive), color: c.rarity(5), strong: true),
            if (s.tracksResult) ...[
              row('Won the $chance', '${s.wins}', percentText(s.winRate), '', sub: true, color: c.ok),
              row('Lost the $chance', '${s.losses}', percentText(s.winRate == null ? null : 100 - s.winRate!), '', sub: true, color: c.err),
              row('Guaranteed', '${s.guaranteed}', '', '', sub: true, color: c.blue),
            ],
            row(info.r4, groupDigits(s.fours), percentText(s.share(s.fours)), averageText(s.avgFour), color: c.rarity(4), strong: true),
            row('Characters', groupDigits(s.fourCharacters), percentText(s.share(s.fourCharacters)), '', sub: true),
            row('Weapons', groupDigits(s.fourWeapons), percentText(s.share(s.fourWeapons)), '', sub: true),
          ],
        ),
        if (s.tracksResult && s.wins + s.losses + s.guaranteed > 0) ...[
          const SizedBox(height: 12),
          _ResultBar(s: s),
        ],
        if (s.unclassified > 0) ...[
          const SizedBox(height: 8),
          Text(
            '${s.unclassified} ${info.r5} at the start of your history ${s.unclassified == 1 ? 'is' : 'are'} not counted above: it cannot be told whether they were a 50/50 or a guarantee.',
            style: Ty.small.copyWith(color: c.faint),
          ),
        ],
      ]),
    );
  }
}

/// How the 5★ were decided, as one bar with a legend in words.
class _ResultBar extends StatelessWidget {
  const _ResultBar({required this.s});
  final BannerStats s;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final parts = [
      ('Won', s.wins, c.ok),
      ('Lost', s.losses, c.err),
      ('Guaranteed', s.guaranteed, c.blue),
    ].where((x) => x.$2 > 0).toList();
    final all = parts.fold<int>(0, (a, x) => a + x.$2);
    return Semantics(
      container: true,
      label: [for (final x in parts) '${x.$1} ${x.$2}'].join(', '),
      child: ExcludeSemantics(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: SizedBox(
              height: 8,
              child: Row(children: [for (final x in parts) Expanded(flex: x.$2, child: Container(color: x.$3, margin: const EdgeInsets.only(right: 1)))]),
            ),
          ),
          const SizedBox(height: 6),
          Wrap(spacing: 14, children: [
            for (final x in parts)
              Row(mainAxisSize: MainAxisSize.min, children: [
                Container(width: 8, height: 8, decoration: BoxDecoration(color: x.$3, shape: BoxShape.circle)),
                const SizedBox(width: 5),
                Text('${x.$1} ${x.$2} · ${(x.$2 * 100 / all).toStringAsFixed(0)}%', style: Ty.small.copyWith(color: c.muted)),
              ]),
          ]),
        ]),
      ),
    );
  }
}

// ------------------------------------------------------------------ pull chips

const _firstFives = 120;
const _firstFours = 60;

/// Lays the chips out for the chosen [view]: icons and names or names only in equal columns that fill the
/// row, icons only in a tight run of squares.
Widget _chipGrid(PullView view, {required double withIcons, required double namesOnly, required List<Widget> children}) => switch (view) {
      PullView.iconsOnly => Wrap(spacing: 6, runSpacing: 6, children: children),
      PullView.namesOnly => TileGrid(minTile: namesOnly, children: children),
      PullView.iconsAndNames => TileGrid(minTile: withIcons, children: children),
    };

/// The 5★ and the 4★ of one banner. Wide windows put them side by side so neither leaves a gap beside the other.
class PullChips extends StatelessWidget {
  const PullChips({super.key, required this.p, required this.game});
  final BannerPity p;
  final GameId game;

  @override
  Widget build(BuildContext context) {
    final fives = FiveChips(key: ValueKey('${p.banner}-fives'), p: p, game: game);
    if (p.recentFours.isEmpty) return fives;
    final fours = FourChips(key: ValueKey('${p.banner}-fours'), p: p, game: game);
    return LayoutBuilder(builder: (context, box) {
      if (box.maxWidth >= 1000) {
        return Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Expanded(flex: 3, child: fives),
          const SizedBox(width: 24),
          Expanded(flex: 2, child: fours),
        ]);
      }
      return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [fives, const SizedBox(height: 16), fours]);
    });
  }
}

/// Every 5★ as a chip: the item, a thin bar for how deep the pity went, the pity number in a colour that says
/// how lucky it was, and a symbol for how it was decided.
class FiveChips extends ConsumerStatefulWidget {
  const FiveChips({super.key, required this.p, required this.game});
  final BannerPity p;
  final GameId game;

  @override
  ConsumerState<FiveChips> createState() => _FiveChipsState();
}

class _FiveChipsState extends ConsumerState<FiveChips> {
  bool _all = false;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final p = widget.p;
    final info = GameInfo.of(widget.game);
    final view = ref.watch(settingsProvider.select((s) => s.pullView));
    final newest = ref.watch(settingsProvider.select((s) => s.newestFirst));
    final fives = newest ? p.recentFives.reversed.toList() : p.recentFives.toList();
    final shown = _all ? fives : fives.take(_firstFives).toList();
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Row(children: [
        Expanded(child: Text('${info.r5} · ${fives.length} · ${newest ? 'newest' : 'oldest'} first', style: Ty.h3)),
        const _Key(),
      ]),
      const SizedBox(height: 8),
      if (fives.isEmpty)
        Text('No ${info.r5} on this banner yet.', style: Ty.body.copyWith(color: c.muted))
      else
        _chipGrid(view, withIcons: 190, namesOnly: 150, children: [for (final f in shown) FiveChip(f: f, p: p, game: widget.game, view: view)]),
      if (fives.length > _firstFives)
        Padding(
          padding: const EdgeInsets.only(top: 8),
          child: TextButton(onPressed: () => setState(() => _all = !_all), child: Text(_all ? 'Show fewer' : 'Show all ${fives.length}')),
        ),
    ]);
  }
}

/// Every 4★, smaller than the 5★: the item and how many pulls it took.
class FourChips extends ConsumerStatefulWidget {
  const FourChips({super.key, required this.p, required this.game});
  final BannerPity p;
  final GameId game;

  @override
  ConsumerState<FourChips> createState() => _FourChipsState();
}

class _FourChipsState extends ConsumerState<FourChips> {
  bool _all = false;

  @override
  Widget build(BuildContext context) {
    final p = widget.p;
    final info = GameInfo.of(widget.game);
    final view = ref.watch(settingsProvider.select((s) => s.pullView));
    final newest = ref.watch(settingsProvider.select((s) => s.newestFirst));
    final fours = newest ? p.recentFours.reversed.toList() : p.recentFours.toList();
    final shown = _all ? fours : fours.take(_firstFours).toList();
    if (fours.isEmpty) return const SizedBox.shrink();
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Text('${info.r4} · ${fours.length} · ${newest ? 'newest' : 'oldest'} first', style: Ty.h3),
      const SizedBox(height: 8),
      _chipGrid(view, withIcons: 150, namesOnly: 130, children: [for (final f in shown) FourChip(f: f, game: widget.game, view: view)]),
      if (fours.length > _firstFours)
        Padding(
          padding: const EdgeInsets.only(top: 8),
          child: TextButton(onPressed: () => setState(() => _all = !_all), child: Text(_all ? 'Show fewer' : 'Show all ${fours.length}')),
        ),
    ]);
  }
}

/// What the chips mean, in words and symbols so it does not rely on colour.
class _Key extends StatelessWidget {
  const _Key();

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    Widget item(IconData icon, Color color, String text) => Row(mainAxisSize: MainAxisSize.min, children: [
          Icon(icon, size: 13, color: color),
          const SizedBox(width: 3),
          Text(text, style: Ty.small.copyWith(color: c.muted)),
        ]);
    return Wrap(spacing: 10, children: [
      item(Icons.check_circle_outline, c.ok, 'Won'),
      item(Icons.cancel_outlined, c.err, 'Lost'),
      item(Icons.lock_outline, c.blue, 'Guaranteed'),
    ]);
  }
}

({IconData icon, Color color}) _outcomeStyle(WishColors c, PullOutcome? o) => switch (o) {
      PullOutcome.win => (icon: Icons.check_circle_outline, color: c.ok),
      PullOutcome.loss => (icon: Icons.cancel_outlined, color: c.err),
      PullOutcome.guaranteed => (icon: Icons.lock_outline, color: c.blue),
      null => (icon: Icons.circle_outlined, color: c.faint),
    };

class FiveChip extends StatelessWidget {
  const FiveChip({super.key, required this.f, required this.p, required this.game, this.view = PullView.iconsAndNames});
  final FiveStar f;
  final BannerPity p;
  final GameId game;
  final PullView view;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final tier = pityTier(f.pity, p.hardPity, p.softPity);
    final tierColor = pityTierColor(c, tier);
    final outcome = _outcomeStyle(c, f.outcome);
    final tracked = p.guaranteeOdds != null;
    final border = tracked && f.outcome != null ? outcome.color.withValues(alpha: 0.85) : c.line;
    final words = '${f.name}, ${f.time.substring(0, 10)}. Pity ${f.pity} of ${p.hardPity} (${pityTierWords(tier).toLowerCase()}). ${resultWords(f.outcome, p.guaranteeOdds)}.';

    if (view == PullView.iconsOnly) {
      return PullIconTile(
        game: game,
        name: f.name,
        rank: 5,
        pity: f.pity,
        pityColor: tierColor,
        border: border,
        tooltip: words,
        outcomeIcon: tracked ? outcome.icon : null,
        outcomeColor: outcome.color,
      );
    }

    final nameStyle = Ty.small.copyWith(color: c.text, fontWeight: FontWeight.w600);
    final pityStyle = Ty.num.copyWith(fontWeight: FontWeight.w700, color: tierColor);
    final Widget content;
    if (view == PullView.namesOnly) {
      content = Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.stretch, children: [
        Row(children: [
          Expanded(child: Text(f.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: nameStyle)),
          const SizedBox(width: 6),
          Text('${f.pity}', style: pityStyle.copyWith(fontSize: 13)),
          if (tracked) ...[const SizedBox(width: 5), Icon(outcome.icon, size: 14, color: outcome.color)],
        ]),
        const SizedBox(height: 3),
        PityBar(value: f.pity, hard: p.hardPity, soft: p.softPity, color: tierColor, height: 3),
      ]);
    } else {
      content = Row(children: [
        ItemIcon(game: game, name: f.name, rank: 5, size: 28),
        const SizedBox(width: 8),
        Expanded(
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, mainAxisSize: MainAxisSize.min, children: [
            Text(f.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: nameStyle),
            const SizedBox(height: 3),
            PityBar(value: f.pity, hard: p.hardPity, soft: p.softPity, color: tierColor, height: 4),
          ]),
        ),
        const SizedBox(width: 8),
        Text('${f.pity}', style: pityStyle),
        if (tracked) ...[const SizedBox(width: 6), Icon(outcome.icon, size: 15, color: outcome.color)],
      ]);
    }
    final names = view == PullView.namesOnly;
    return Tooltip(
      message: words,
      child: Semantics(
        label: words,
        child: ExcludeSemantics(
          child: Container(
            padding: names ? const EdgeInsets.symmetric(horizontal: 8, vertical: 5) : const EdgeInsets.fromLTRB(6, 5, 10, 5),
            decoration: BoxDecoration(
              color: c.panel2,
              borderRadius: BorderRadius.circular(names ? 8 : 10),
              border: Border.all(color: border, width: 1.3),
            ),
            child: content,
          ),
        ),
      ),
    );
  }
}

class FourChip extends StatelessWidget {
  const FourChip({super.key, required this.f, required this.game, this.view = PullView.iconsAndNames});
  final FourStar f;
  final GameId game;
  final PullView view;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final words = '${f.name}, ${f.time.substring(0, 10)}. Pity ${f.pity}. ${f.isCharacter ? 'Character' : 'Weapon'}.';
    final border = c.purple.withValues(alpha: 0.55);
    if (view == PullView.iconsOnly) {
      return PullIconTile(game: game, name: f.name, rank: 4, pity: f.pity, pityColor: c.purple, border: border, tooltip: words, size: 44);
    }
    return Tooltip(
      message: words,
      child: Semantics(
        label: words,
        child: ExcludeSemantics(
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 4),
            decoration: BoxDecoration(color: c.panel2, borderRadius: BorderRadius.circular(8), border: Border.all(color: border)),
            child: Row(children: [
              if (view == PullView.iconsAndNames) ...[ItemIcon(game: game, name: f.name, rank: 4, size: 20), const SizedBox(width: 7)],
              Expanded(child: Text(f.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.small.copyWith(color: c.text))),
              const SizedBox(width: 7),
              Text('${f.pity}', style: Ty.num.copyWith(fontSize: 12.5, fontWeight: FontWeight.w700, color: c.purple)),
            ]),
          ),
        ),
      ),
    );
  }
}
