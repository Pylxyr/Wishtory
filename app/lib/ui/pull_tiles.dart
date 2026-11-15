import 'package:flutter/material.dart';

import '../bridge/wishtory_api.dart';
import 'tokens.dart';
import 'widgets.dart';

/// How many equal columns, each at least [minTile] wide with [gap] between them, fit in [width]. Never
/// less than one.
int tileColumns(double width, double minTile, double gap) {
  if (!width.isFinite || width <= 0) return 1;
  final n = ((width + gap) / (minTile + gap)).floor();
  return n < 1 ? 1 : n;
}

/// Children in equal-width columns that fill the row exactly, so no space is left over on the right
/// however wide the window is.
class TileGrid extends StatelessWidget {
  const TileGrid({super.key, required this.minTile, required this.children, this.gap = 6});

  final double minTile;
  final double gap;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) => LayoutBuilder(builder: (context, box) {
        final total = box.maxWidth.isFinite ? box.maxWidth : minTile;
        final cols = tileColumns(total, minTile, gap);
        final width = (total - (cols - 1) * gap) / cols;
        return Wrap(
          spacing: gap,
          runSpacing: gap,
          children: [for (final child in children) SizedBox(width: width, child: child)],
        );
      });
}

/// An item icon with its pity in the corner, for the lists that show icons only. The border says how
/// it went (the colour is never the only signal: the tooltip and screen readers get the words).
class PullIconTile extends StatelessWidget {
  const PullIconTile({
    super.key,
    required this.game,
    required this.name,
    required this.rank,
    required this.pity,
    required this.pityColor,
    required this.border,
    required this.tooltip,
    this.outcomeIcon,
    this.outcomeColor,
    this.size = 52,
    this.onTap,
    this.selected = false,
  });

  final GameId game;
  final String name;
  final int rank;
  final int? pity;
  final Color pityColor;
  final Color border;
  final String tooltip;
  final IconData? outcomeIcon;
  final Color? outcomeColor;
  final double size;
  final VoidCallback? onTap;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final tile = Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: c.panel2,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: selected ? c.accent : border, width: selected ? 2 : 1.3),
      ),
      child: Stack(alignment: Alignment.center, children: [
        ItemIcon(game: game, name: name, rank: rank, size: size * 0.7),
        if (pity != null)
          Positioned(
            right: 2,
            bottom: 1,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 3),
              decoration: BoxDecoration(color: c.bg.withValues(alpha: 0.88), borderRadius: BorderRadius.circular(4)),
              child: Text('$pity', style: Ty.num.copyWith(fontSize: 11, height: 1.25, fontWeight: FontWeight.w700, color: pityColor)),
            ),
          ),
        if (outcomeIcon != null) Positioned(left: 3, top: 3, child: Icon(outcomeIcon, size: 12, color: outcomeColor)),
      ]),
    );
    return Tooltip(
      message: tooltip,
      child: Semantics(
        button: onTap != null,
        label: tooltip,
        onTap: onTap,
        child: ExcludeSemantics(
          child: onTap == null
              ? tile
              : Material(
                  color: Colors.transparent,
                  borderRadius: BorderRadius.circular(10),
                  child: InkWell(borderRadius: BorderRadius.circular(10), onTap: onTap, child: tile),
                ),
        ),
      ),
    );
  }
}
