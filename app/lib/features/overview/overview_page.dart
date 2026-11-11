import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';
import '../../domain/planning.dart';
import '../../domain/recovery.dart';
import '../../state/loadable.dart';
import '../../state/nav.dart';
import '../../state/providers.dart';
import '../../state/refresh.dart';
import '../../ui/density.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';
import '../shell/shell.dart';
import 'banner_stats.dart';

/// Which banner's detail is open; null means the first one.
class SelectedBanner extends Notifier<String?> {
  @override
  String? build() => null;
  void select(String key) => state = key;
}

final selectedBannerProvider = NotifierProvider<SelectedBanner, String?>(SelectedBanner.new);

class OverviewPage extends ConsumerWidget {
  const OverviewPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final nav = ref.watch(navProvider);
    final accounts = ref.watch(accountsProvider);
    final account = ref.watch(currentAccountProvider);
    final pity = ref.watch(pityProvider);
    final info = GameInfo.of(nav.game);

    if (accounts.data == null && accounts.loading) {
      return const PageBody(children: [_OverviewSkeleton()]);
    }
    if (account == null) {
      return PageBody(children: [
        const RefreshPanel(),
        SizedBox(
          height: 420,
          child: EmptyState(
            icon: Icons.inbox_outlined,
            title: 'No ${info.name} history yet',
            body: 'Two steps: ${info.howTo} Then come back here and press Refresh. '
                'Wishtory reads the link the game saved on this PC. Nothing is sent anywhere but HoYoverse.',
            actions: [
              FilledButton.icon(
                onPressed: () => ref.read(refreshProvider.notifier).run(),
                icon: const Icon(Icons.sync, size: 18),
                label: const Text('Refresh'),
              ),
              OutlinedButton(onPressed: () => ref.read(navProvider.notifier).goTo(AppPage.importExport), child: const Text('Import a file instead')),
            ],
          ),
        ),
      ]);
    }
    return PageBody(children: [
      const RefreshPanel(),
      _body(context, ref, account, pity),
    ]);
  }

  Widget _body(BuildContext context, WidgetRef ref, Account account, Loadable<List<BannerPity>> pity) {
    if (pity.failed && pity.data == null) {
      return RecoveryCard(
        recovery: recoveryFor(pity.error!, game: account.game),
        onRetry: () => ref.read(dataTickProvider.notifier).bump(),
      );
    }
    final list = pity.data;
    if (list == null) return const _OverviewSkeleton();
    if (list.isEmpty || list.every((p) => p.total == 0)) {
      return SizedBox(
        height: 360,
        child: EmptyState(
          icon: Icons.hourglass_empty,
          title: 'This account has no pulls yet',
          body: 'Press Refresh after opening the history page in the game.',
          actions: [FilledButton(onPressed: () => ref.read(refreshProvider.notifier).run(), child: const Text('Refresh'))],
        ),
      );
    }
    final selectedKey = ref.watch(selectedBannerProvider);
    final selected = list.firstWhere((p) => p.banner == selectedKey, orElse: () => list.first);
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      _StatusStrip(game: account.game, list: list),
      const SizedBox(height: 20),
      _LiveNow(account: account),
      const SectionLabel('Banners'),
      _BannerGrid(list: list, game: account.game, selected: selected.banner),
      const SizedBox(height: 20),
      _BannerDetail(p: selected, game: account.game),
    ]);
  }
}

class _OverviewSkeleton extends StatelessWidget {
  const _OverviewSkeleton();

  @override
  Widget build(BuildContext context) => Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(children: [for (var i = 0; i < 4; i++) const Expanded(child: Padding(padding: EdgeInsets.only(right: 12), child: Skeleton(height: 72, radius: 12)))]),
        const SizedBox(height: 24),
        Row(children: [for (var i = 0; i < 3; i++) const Expanded(child: Padding(padding: EdgeInsets.only(right: 12), child: Skeleton(height: 150, radius: 12)))]),
        const SizedBox(height: 24),
        const Skeleton(height: 240, radius: 12),
      ]);
}

// ------------------------------------------------------------------ status strip

class _StatusStrip extends StatelessWidget {
  const _StatusStrip({required this.game, required this.list});
  final GameId game;
  final List<BannerPity> list;

  @override
  Widget build(BuildContext context) {
    final info = GameInfo.of(game);
    final total = list.fold<int>(0, (a, p) => a + p.total);
    final fives = list.fold<int>(0, (a, p) => a + p.fiveStars);
    var weighted = 0.0;
    var weight = 0;
    FiveStar? latest;
    for (final p in list) {
      if (p.avgFivePity != null && p.fiveStars > 0) {
        weighted += p.avgFivePity! * p.fiveStars;
        weight += p.fiveStars;
      }
      for (final f in p.recentFives) {
        if (latest == null || f.time.compareTo(latest.time) > 0) latest = f;
      }
    }
    final avg = weight == 0 ? null : (weighted / weight).toStringAsFixed(1);
    final tiles = [
      _Stat('Total pulls', groupDigits(total), sub: '≈ ${groupDigits(total * info.pullCost)} ${info.currency}'),
      _Stat('${info.r5} pulls', groupDigits(fives), sub: total == 0 ? null : '${(fives * 100 / total).toStringAsFixed(2)}% of pulls'),
      _Stat('Average ${info.r5} pity', avg ?? '–', sub: avg == null ? 'No ${info.r5} yet' : 'pulls per ${info.r5}'),
      _Stat('Latest ${info.r5}', latest?.name ?? '–', sub: latest == null ? null : '${latest.pity} pulls · ${latest.time.substring(0, 10)}', text: true),
    ];
    return LayoutBuilder(builder: (context, box) {
      final gap = Density.of(context).gap;
      final cols = box.maxWidth >= 900 ? 4 : 2;
      final w = (box.maxWidth - (cols - 1) * gap) / cols;
      return Wrap(spacing: gap, runSpacing: gap, children: [for (final t in tiles) SizedBox(width: w, child: t)]);
    });
  }
}

class _Stat extends StatelessWidget {
  const _Stat(this.label, this.value, {this.sub, this.text = false});
  final String label;
  final String value;
  final String? sub;
  final bool text;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    return Panel(
      padding: const EdgeInsets.all(14),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text(label, style: Ty.small.copyWith(color: c.muted)),
        const SizedBox(height: 4),
        Text(value, maxLines: 1, overflow: TextOverflow.ellipsis, style: text ? Ty.h1 : Ty.bigNum.copyWith(fontSize: 26)),
        if (sub != null) Text(sub!, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.small.copyWith(color: c.faint)),
      ]),
    );
  }
}

// ------------------------------------------------------------------ live banners

class _LiveNow extends ConsumerWidget {
  const _LiveNow({required this.account});
  final Account account;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final banners = ref.watch(bannersProvider).data;
    final catalog = ref.watch(catalogProvider).data;
    final live = liveBanners(banners, account.game, account.timezone, catalog);
    if (live.isEmpty) return const SizedBox.shrink();
    final c = context.wc;
    return Padding(
      padding: const EdgeInsets.only(bottom: 20),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        const SectionLabel('Live now'),
        Wrap(spacing: 10, runSpacing: 10, children: [
          for (final b in live)
            Panel(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, mainAxisSize: MainAxisSize.min, children: [
                Text(b.name, style: Ty.h3),
                if (b.featured.isNotEmpty) Text(b.featured.join(' / '), style: Ty.small.copyWith(color: c.muted)),
                Text(timeLeft(b.msLeft), style: Ty.small.copyWith(color: c.faint)),
              ]),
            ),
        ]),
      ]),
    );
  }
}

// ------------------------------------------------------------------ banner cards

class _BannerGrid extends ConsumerWidget {
  const _BannerGrid({required this.list, required this.game, required this.selected});
  final List<BannerPity> list;
  final GameId game;
  final String selected;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return LayoutBuilder(builder: (context, box) {
      final gap = Density.of(context).gap;
      final cols = box.maxWidth >= 900 ? 3 : (box.maxWidth >= 600 ? 2 : 1);
      final w = (box.maxWidth - (cols - 1) * gap) / cols;
      return Wrap(spacing: gap, runSpacing: gap, children: [
        for (final p in list)
          SizedBox(
            width: w,
            child: _BannerCard(p: p, game: game, selected: p.banner == selected, onTap: () => ref.read(selectedBannerProvider.notifier).select(p.banner)),
          ),
      ]);
    });
  }
}

class _BannerCard extends StatelessWidget {
  const _BannerCard({required this.p, required this.game, required this.selected, required this.onTap});
  final BannerPity p;
  final GameId game;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final info = GameInfo.of(game);
    final inSoft = p.softPity != null && p.pity >= p.softPity!;
    final odds = p.guaranteeOdds;
    return Semantics(
      button: true,
      selected: selected,
      label: '${p.label}: ${p.pity} of ${p.hardPity} pulls${inSoft ? ', soft pity reached' : ''}',
      child: Panel(
        selected: selected,
        onTap: onTap,
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(p.label, style: Ty.h3, maxLines: 1, overflow: TextOverflow.ellipsis),
          const SizedBox(height: 8),
          Row(crossAxisAlignment: CrossAxisAlignment.baseline, textBaseline: TextBaseline.alphabetic, children: [
            Text('${p.pity}', style: Ty.bigNum),
            Text(' / ${p.hardPity}', style: Ty.num.copyWith(color: c.faint)),
            const Spacer(),
            Text('${groupDigits(p.total)} pulls', style: Ty.small.copyWith(color: c.faint)),
          ]),
          const SizedBox(height: 8),
          PityBar(value: p.pity, hard: p.hardPity, soft: p.softPity),
          const SizedBox(height: 10),
          Wrap(spacing: 6, runSpacing: 6, children: [
            if (odds != null && p.guaranteed == true) StatusChip('Guaranteed ${info.r5}', color: c.blue, icon: Icons.verified_outlined),
            if (odds != null && p.guaranteed == false) StatusChip('$odds/${100 - odds}', color: c.muted, icon: Icons.casino_outlined),
            if (inSoft) StatusChip('Soft pity', color: c.accent, icon: Icons.trending_up),
            if (p.guaranteed == null && !inSoft) Text('${p.fiveStars} ${info.fives}', style: Ty.small.copyWith(color: c.faint)),
          ]),
        ]),
      ),
    );
  }
}

// ------------------------------------------------------------------ banner detail

class _BannerDetail extends StatelessWidget {
  const _BannerDetail({required this.p, required this.game});
  final BannerPity p;
  final GameId game;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final info = GameInfo.of(game);
    final plan = p.total > 0 ? pullsToGuarantee(p) : null;
    String worth(int pulls) => '${groupDigits(pulls * info.pullCost)} ${info.currency.toLowerCase()}';
    final odds = p.guaranteeOdds;
    final inSoft = p.softPity != null && p.pity >= p.softPity!;

    String guarantee() {
      if (odds == null) return 'The featured item cannot be told from the history on this banner, so no guarantee is tracked.';
      return switch (p.guaranteed) {
        true => 'Next ${info.r5} is guaranteed to be featured. Your last ${info.r5} was not the featured one.',
        false => 'Not guaranteed: next ${info.r5} is a $odds/${100 - odds}. Your last ${info.r5} was the featured one.',
        null => 'Guarantee status unknown. Not enough history to tell.',
      };
    }

    String planText() {
      if (plan == null) return '';
      if (plan.featured == null) return 'Up to ${plan.fiveStar} more pulls (${worth(plan.fiveStar)}) to a guaranteed ${info.r5}.';
      if (p.guaranteed == true) return 'Up to ${plan.fiveStar} more pulls (${worth(plan.fiveStar)}) to the featured ${info.r5}.';
      return 'Worst case ${plan.featured} more pulls (${worth(plan.featured!)}) to a guaranteed featured ${info.r5}: '
          '${plan.fiveStar} to the next ${info.r5}, then up to ${p.hardPity} more if the $odds/${100 - odds!} is lost.';
    }

    return Panel(
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(children: [
          Expanded(child: Text(p.label, style: Ty.h2)),
          if (inSoft) const StatusChip('Past soft pity', icon: Icons.trending_up),
        ]),
        const SizedBox(height: 6),
        Text(guarantee(), style: Ty.body),
        if (plan != null) ...[
          const SizedBox(height: 4),
          Text(planText(), style: Ty.body.copyWith(color: c.muted)),
        ],
        const SizedBox(height: 18),
        LayoutBuilder(builder: (context, box) {
          final stats = StatsTable(p: p, game: game);
          final chips = PullChips(key: ValueKey(p.banner), p: p, game: game);
          if (box.maxWidth < 820) {
            return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [stats, const SizedBox(height: 22), chips]);
          }
          return Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
            SizedBox(width: 380, child: stats),
            const SizedBox(width: 28),
            Expanded(child: chips),
          ]);
        }),
      ]),
    );
  }
}
