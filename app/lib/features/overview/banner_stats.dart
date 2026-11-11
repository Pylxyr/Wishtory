import 'package:flutter/material.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';
import '../../ui/density.dart';
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

const _firstChips = 30;

/// Every 5★ (newest first) as a chip: the item, a thin bar for how deep the pity went, the pity number in a colour
/// that says how lucky it was, and a symbol for how it was decided. The 4★ follow, smaller.
class PullChips extends StatefulWidget {
  const PullChips({super.key, required this.p, required this.game});
  final BannerPity p;
  final GameId game;

  @override
  State<PullChips> createState() => _PullChipsState();
}

class _PullChipsState extends State<PullChips> {
  bool _allFives = false;
  bool _allFours = false;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final p = widget.p;
    final info = GameInfo.of(widget.game);
    final fives = p.recentFives.reversed.toList();
    final fours = p.recentFours.reversed.toList();
    final shownFives = _allFives ? fives : fives.take(_firstChips).toList();
    final shownFours = _allFours ? fours : fours.take(_firstChips).toList();

    Widget more(bool all, int total, VoidCallback toggle) => total <= _firstChips
        ? const SizedBox.shrink()
        : Padding(
            padding: const EdgeInsets.only(top: 8),
            child: TextButton(onPressed: toggle, child: Text(all ? 'Show fewer' : 'Show all $total')),
          );

    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Row(children: [
        Expanded(child: Text('${info.r5} · newest first', style: Ty.h3)),
        const _Key(),
      ]),
      const SizedBox(height: 10),
      if (fives.isEmpty)
        Text('No ${info.r5} on this banner yet.', style: Ty.body.copyWith(color: c.muted))
      else
        Wrap(spacing: 8, runSpacing: 8, children: [for (final f in shownFives) FiveChip(f: f, p: p, game: widget.game)]),
      more(_allFives, fives.length, () => setState(() => _allFives = !_allFives)),
      if (fours.isNotEmpty) ...[
        const SizedBox(height: 18),
        Text('${info.r4} · newest first', style: Ty.h3),
        const SizedBox(height: 10),
        Wrap(spacing: 6, runSpacing: 6, children: [for (final f in shownFours) FourChip(f: f, game: widget.game)]),
        more(_allFours, fours.length, () => setState(() => _allFours = !_allFours)),
      ],
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
  const FiveChip({super.key, required this.f, required this.p, required this.game});
  final FiveStar f;
  final BannerPity p;
  final GameId game;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final tier = pityTier(f.pity, p.hardPity, p.softPity);
    final tierColor = pityTierColor(c, tier);
    final outcome = _outcomeStyle(c, f.outcome);
    final tracked = p.guaranteeOdds != null;
    final words = '${f.name}, ${f.time.substring(0, 10)}. Pity ${f.pity} of ${p.hardPity} (${pityTierWords(tier).toLowerCase()}). ${resultWords(f.outcome, p.guaranteeOdds)}.';
    return Tooltip(
      message: words,
      child: Semantics(
        label: words,
        child: ExcludeSemantics(
          child: Container(
            padding: const EdgeInsets.fromLTRB(6, 5, 10, 5),
            decoration: BoxDecoration(
              color: c.panel2,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: tracked && f.outcome != null ? outcome.color.withValues(alpha: 0.85) : c.line, width: 1.3),
            ),
            child: Row(mainAxisSize: MainAxisSize.min, children: [
              ItemIcon(game: game, name: f.name, rank: 5, size: 28),
              const SizedBox(width: 8),
              Column(crossAxisAlignment: CrossAxisAlignment.start, mainAxisSize: MainAxisSize.min, children: [
                ConstrainedBox(constraints: const BoxConstraints(maxWidth: 150), child: Text(f.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.small.copyWith(color: c.text, fontWeight: FontWeight.w600))),
                const SizedBox(height: 3),
                SizedBox(width: 90, child: PityBar(value: f.pity, hard: p.hardPity, soft: p.softPity, color: tierColor, height: 4)),
              ]),
              const SizedBox(width: 10),
              Text('${f.pity}', style: Ty.num.copyWith(fontWeight: FontWeight.w700, color: tierColor)),
              if (tracked) ...[
                const SizedBox(width: 6),
                Icon(outcome.icon, size: 15, color: outcome.color),
              ],
            ]),
          ),
        ),
      ),
    );
  }
}

class FourChip extends StatelessWidget {
  const FourChip({super.key, required this.f, required this.game});
  final FourStar f;
  final GameId game;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final words = '${f.name}, ${f.time.substring(0, 10)}. Pity ${f.pity}. ${f.isCharacter ? 'Character' : 'Weapon'}.';
    return Tooltip(
      message: words,
      child: Semantics(
        label: words,
        child: ExcludeSemantics(
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 4),
            decoration: BoxDecoration(color: c.panel2, borderRadius: BorderRadius.circular(8), border: Border.all(color: c.purple.withValues(alpha: 0.55))),
            child: Row(mainAxisSize: MainAxisSize.min, children: [
              Icon(f.isCharacter ? Icons.person_outline : Icons.shield_outlined, size: 13, color: c.purple),
              const SizedBox(width: 5),
              ConstrainedBox(constraints: const BoxConstraints(maxWidth: 150), child: Text(f.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.small.copyWith(color: c.text))),
              const SizedBox(width: 7),
              Text('${f.pity}', style: Ty.num.copyWith(fontSize: 12.5, fontWeight: FontWeight.w700, color: c.purple)),
            ]),
          ),
        ),
      ),
    );
  }
}
