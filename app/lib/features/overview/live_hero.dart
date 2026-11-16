import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/planning.dart';
import '../../state/providers.dart';
import '../../ui/banner_art.dart';
import '../../ui/tokens.dart';

/// The banners running right now, large and centered at the top of the Overview, each on the artwork of
/// what it features. Tapping one opens that banner's pity below.
class LiveHero extends ConsumerWidget {
  const LiveHero({super.key, required this.account, required this.pity, required this.onSelect});

  final Account account;
  final List<BannerPity> pity;
  final ValueChanged<String> onSelect;

  /// Character banners first, then weapon banners, then the rest; soonest ending first within each.
  static int _order(String pool) => switch (pool) {
        'character' => 0,
        'weapon' => 1,
        _ => 2,
      };

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final banners = ref.watch(bannersProvider).data;
    final catalog = ref.watch(catalogProvider).data;
    final live = liveBanners(banners, account.game, account.timezone, catalog);
    if (live.isEmpty) return const SizedBox.shrink();
    final sorted = [...live]..sort((a, b) {
        final byPool = _order(a.pool).compareTo(_order(b.pool));
        return byPool != 0 ? byPool : a.msLeft.compareTo(b.msLeft);
      });
    final c = context.wc;
    return Padding(
      padding: const EdgeInsets.only(bottom: 24),
      child: Column(children: [
        Semantics(
          header: true,
          child: Row(mainAxisSize: MainAxisSize.min, children: [
            Container(width: 8, height: 8, decoration: BoxDecoration(color: c.ok, shape: BoxShape.circle)),
            const SizedBox(width: 8),
            Text('LIVE NOW', style: Ty.label.copyWith(color: c.muted)),
          ]),
        ),
        const SizedBox(height: 12),
        LayoutBuilder(builder: (context, box) {
          const gap = 16.0;
          var cols = box.maxWidth >= 1500 ? 3 : (box.maxWidth >= 980 ? 2 : 1);
          if (cols > sorted.length) cols = sorted.length;
          var width = (box.maxWidth - (cols - 1) * gap) / cols;
          final cap = sorted.length == 1 ? 960.0 : 780.0;
          if (width > cap) width = cap;
          return Wrap(
            alignment: WrapAlignment.center,
            spacing: gap,
            runSpacing: gap,
            children: [
              for (final b in sorted)
                SizedBox(
                  width: width,
                  child: _LiveCard(
                    game: account.game,
                    banner: b,
                    pity: pity.where((p) => p.banner == b.pool).firstOrNull,
                    onSelect: onSelect,
                  ),
                ),
            ],
          );
        }),
      ]),
    );
  }
}

class _LiveCard extends StatelessWidget {
  const _LiveCard({required this.game, required this.banner, required this.pity, required this.onSelect});

  final GameId game;
  final LiveBanner banner;
  final BannerPity? pity;
  final ValueChanged<String> onSelect;

  String get _kind => switch (banner.pool) {
        'character' => game == GameId.starRail ? 'Character warp' : 'Character event',
        'weapon' => game == GameId.starRail ? 'Light cone warp' : 'Weapon event',
        final other => other.isEmpty ? 'Event' : '${other[0].toUpperCase()}${other.substring(1)}',
      };

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final p = pity;
    final words = '${banner.name}. $_kind. ${banner.featured.join(', ')}. ${timeLeft(banner.msLeft)}.${p == null ? '' : ' Your pity ${p.pity} of ${p.hardPity}.'}';
    final radius = BorderRadius.circular(18);
    return Semantics(
      button: p != null,
      label: words,
      onTap: p == null ? null : () => onSelect(banner.pool),
      child: ExcludeSemantics(
        child: Container(
          height: 250,
          decoration: BoxDecoration(borderRadius: radius, border: Border.all(color: c.line2)),
          clipBehavior: Clip.antiAlias,
          child: Material(
            color: Colors.transparent,
            child: InkWell(
              onTap: p == null ? null : () => onSelect(banner.pool),
              child: Stack(children: [
                // A tinted base, so the card is never empty while its artwork downloads.
                Positioned.fill(
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      gradient: LinearGradient(
                        begin: Alignment.topLeft,
                        end: Alignment.bottomRight,
                        colors: [c.panel2, Color.alphaBlend(c.accent.withValues(alpha: 0.22), c.panel)],
                      ),
                    ),
                  ),
                ),
                // The artwork sits in the right part; the text has the left.
                Positioned.fill(left: 150, child: BannerArt(game: game, names: banner.featured)),
                // A fade from the card colour keeps the text readable over any picture.
                Positioned.fill(
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      gradient: LinearGradient(
                        begin: Alignment.centerLeft,
                        end: Alignment.centerRight,
                        stops: const [0, 0.42, 0.78],
                        colors: [c.panel.withValues(alpha: 0.97), c.panel.withValues(alpha: 0.72), c.panel.withValues(alpha: 0)],
                      ),
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(22, 18, 22, 18),
                  child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 3),
                      decoration: BoxDecoration(color: c.accent, borderRadius: BorderRadius.circular(6)),
                      child: Text(_kind.toUpperCase(), style: Ty.label.copyWith(color: c.accentInk)),
                    ),
                    const Spacer(),
                    FractionallySizedBox(
                      widthFactor: 0.62,
                      alignment: Alignment.centerLeft,
                      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                        Text(
                          banner.name,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(fontSize: 26, fontWeight: FontWeight.w700, height: 1.15, letterSpacing: -0.5, color: c.text),
                        ),
                        const SizedBox(height: 8),
                        for (final name in banner.featured)
                          Padding(
                            padding: const EdgeInsets.only(bottom: 2),
                            child: Row(mainAxisSize: MainAxisSize.min, children: [
                              Icon(Icons.star_rounded, size: 16, color: c.gold),
                              const SizedBox(width: 4),
                              Flexible(child: Text(name, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.h3.copyWith(color: c.text))),
                            ]),
                          ),
                      ]),
                    ),
                    const SizedBox(height: 10),
                    Row(children: [
                      Icon(Icons.schedule, size: 15, color: c.muted),
                      const SizedBox(width: 5),
                      Text(timeLeft(banner.msLeft), style: Ty.small.copyWith(color: c.muted)),
                      const Spacer(),
                      if (p != null)
                        Container(
                          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                          decoration: BoxDecoration(color: c.bg.withValues(alpha: 0.78), borderRadius: BorderRadius.circular(8), border: Border.all(color: c.line2)),
                          child: Text('Your pity ${p.pity} / ${p.hardPity}', style: Ty.num.copyWith(fontSize: 13, fontWeight: FontWeight.w600, color: c.text)),
                        ),
                    ]),
                  ]),
                ),
              ]),
            ),
          ),
        ),
      ),
    );
  }
}
