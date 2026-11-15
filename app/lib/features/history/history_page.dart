import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';
import '../../domain/recovery.dart';
import '../../state/providers.dart';
import '../../state/refresh.dart';
import '../../state/settings.dart';
import '../../state/toasts.dart';
import '../../ui/density.dart';
import '../../ui/pull_tiles.dart';
import '../../ui/tokens.dart';
import '../../ui/view_controls.dart';
import '../../ui/widgets.dart';
import '../overview/banner_stats.dart' show resultWords;
import '../shell/shell.dart';
import 'history_logic.dart';

class HistoryFilterController extends Notifier<HistoryFilter> {
  @override
  HistoryFilter build() {
    // A different game or account starts from a clean filter (its banner keys differ).
    ref.watch(currentAccountProvider.select((a) => a == null ? null : '${a.game.name}|${a.uid}'));
    return const HistoryFilter();
  }

  void update(HistoryFilter Function(HistoryFilter) change) => state = change(state);
  void clear() => state = HistoryFilter(group: state.group, sort: state.sort, ascending: state.ascending);
}

final historyFilterProvider = NotifierProvider<HistoryFilterController, HistoryFilter>(HistoryFilterController.new);

/// 5★ outcome and hard pity per banner, joined from the pity engine so History shows the same
/// results as the Overview. Key: "bannerKey|name|time|pity".
class PityFacts {
  const PityFacts(this.outcomes, this.hardPity, this.odds);
  final Map<String, PullOutcome?> outcomes;
  final Map<String, int> hardPity;
  final Map<String, int?> odds;
}

final pityFactsProvider = Provider<PityFacts>((ref) {
  final list = ref.watch(pityProvider).data ?? const <BannerPity>[];
  final outcomes = <String, PullOutcome?>{};
  final hard = <String, int>{};
  final odds = <String, int?>{};
  for (final p in list) {
    hard[p.banner] = p.hardPity;
    odds[p.banner] = p.guaranteeOdds;
    for (final f in p.recentFives) {
      outcomes['${p.banner}|${f.name}|${f.time}|${f.pity}'] = f.outcome;
    }
  }
  return PityFacts(outcomes, hard, odds);
});

final selectedPullProvider = NotifierProvider<SelectedPull, String?>(SelectedPull.new);

class SelectedPull extends Notifier<String?> {
  @override
  String? build() {
    ref.watch(currentAccountProvider);
    return null;
  }

  void select(String? id) => state = id;
}

class HistoryPage extends ConsumerStatefulWidget {
  const HistoryPage({super.key});

  @override
  ConsumerState<HistoryPage> createState() => _HistoryPageState();
}

class _HistoryPageState extends ConsumerState<HistoryPage> {
  final _search = FocusNode();
  final _searchText = TextEditingController();

  @override
  void dispose() {
    _search.dispose();
    _searchText.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final account = ref.watch(currentAccountProvider);
    final history = ref.watch(historyProvider);
    final settings = ref.watch(settingsProvider);
    final filter = ref.watch(historyFilterProvider);
    final rare = settings.historyMode == HistoryMode.rare;

    Widget body;
    if (account == null) {
      body = EmptyState(
        icon: Icons.inbox_outlined,
        title: 'No history to show yet',
        body: 'Refresh after opening the history page in the game, or import a file.',
        actions: [FilledButton(onPressed: () => ref.read(refreshProvider.notifier).run(), child: const Text('Refresh'))],
      );
    } else if (history.failed && history.data == null) {
      body = Padding(
        padding: const EdgeInsets.all(24),
        child: RecoveryCard(recovery: recoveryFor(history.error!, game: account.game), onRetry: () => ref.read(dataTickProvider.notifier).bump()),
      );
    } else if (history.data == null) {
      body = Padding(
        padding: const EdgeInsets.all(24),
        child: Column(children: [for (var i = 0; i < 8; i++) const Padding(padding: EdgeInsets.only(bottom: 8), child: Skeleton(height: 36, radius: 8))]),
      );
    } else {
      // Time order follows the persisted "newest first" choice; other sort columns keep their own direction.
      final effective = (rare || filter.sort == SortColumn.time) ? filter.copyWith(sort: SortColumn.time, ascending: !settings.newestFirst) : filter;
      final rows = applyFilters(history.data!, effective, rare: rare, includeFours: settings.showFours);
      body = _Content(all: history.data!, rows: rows, account: account, rare: rare, searchFocus: _search, searchText: _searchText);
    }

    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.keyF, control: true): () => _search.requestFocus(),
      },
      child: Column(children: [
        const Padding(padding: EdgeInsets.fromLTRB(24, 16, 24, 0), child: RefreshPanel()),
        Expanded(child: body),
      ]),
    );
  }
}

class _Content extends ConsumerWidget {
  const _Content({required this.all, required this.rows, required this.account, required this.rare, required this.searchFocus, required this.searchText});

  final List<HistoryPull> all;
  final List<HistoryPull> rows;
  final Account account;
  final bool rare;
  final FocusNode searchFocus;
  final TextEditingController searchText;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final filter = ref.watch(historyFilterProvider);
    final wide = MediaQuery.sizeOf(context).width >= 1280;
    final selectedId = ref.watch(selectedPullProvider);
    final selected = selectedId == null ? null : rows.where((p) => p.id == selectedId).firstOrNull;

    final list = Column(children: [
      _Toolbar(account: account, total: all.length, shown: rows.length, rare: rare, searchFocus: searchFocus, searchText: searchText, rows: rows),
      if (all.isEmpty)
        const Expanded(child: EmptyState(icon: Icons.hourglass_empty, title: 'No pulls saved for this account', body: 'Press Refresh to download them.'))
      else if (rows.isEmpty)
        Expanded(
          child: EmptyState(
            icon: Icons.filter_alt_off_outlined,
            title: 'Nothing matches these filters',
            body: rare && filter.isDefault ? 'No ${GameInfo.of(account.game).r5} pulls yet.' : 'Try removing a filter.',
            actions: [
              if (!filter.isDefault)
                OutlinedButton(
                  onPressed: () {
                    searchText.clear();
                    ref.read(historyFilterProvider.notifier).clear();
                  },
                  child: const Text('Clear filters'),
                ),
            ],
          ),
        )
      else
        Expanded(child: rare ? _RareList(rows: rows, account: account) : _AllTable(rows: rows, account: account)),
    ]);

    if (selected != null && wide) {
      return Row(children: [
        Expanded(child: list),
        Container(
          width: 340,
          decoration: BoxDecoration(color: c.panel, border: Border(left: BorderSide(color: c.line))),
          child: _Detail(pull: selected, account: account),
        ),
      ]);
    }
    return list;
  }
}

// ------------------------------------------------------------------ toolbar

class _Toolbar extends ConsumerWidget {
  const _Toolbar({
    required this.account,
    required this.total,
    required this.shown,
    required this.rare,
    required this.searchFocus,
    required this.searchText,
    required this.rows,
  });

  final Account account;
  final int total;
  final int shown;
  final bool rare;
  final FocusNode searchFocus;
  final TextEditingController searchText;
  final List<HistoryPull> rows;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final info = GameInfo.of(account.game);
    final filter = ref.watch(historyFilterProvider);
    final settings = ref.watch(settingsProvider);
    final meta = (ref.watch(metaProvider).data ?? const <GameMeta>[]).where((m) => m.game == account.game).firstOrNull;
    final controller = ref.read(historyFilterProvider.notifier);
    final prefs = ref.read(settingsProvider.notifier);

    return Container(
      padding: const EdgeInsets.fromLTRB(24, 12, 24, 10),
      decoration: BoxDecoration(border: Border(bottom: BorderSide(color: c.line))),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Wrap(spacing: 12, runSpacing: 10, crossAxisAlignment: WrapCrossAlignment.center, children: [
          SegmentedButton<HistoryMode>(
            showSelectedIcon: false,
            segments: const [
              ButtonSegment(value: HistoryMode.rare, label: Text('Rare pulls'), icon: Icon(Icons.star_outline, size: 16)),
              ButtonSegment(value: HistoryMode.all, label: Text('All pulls'), icon: Icon(Icons.table_rows_outlined, size: 16)),
            ],
            selected: {settings.historyMode},
            onSelectionChanged: (s) => prefs.setHistoryMode(s.first),
          ),
          SizedBox(
            width: 240,
            child: TextField(
              controller: searchText,
              focusNode: searchFocus,
              decoration: fieldDecoration(
                context,
                hint: 'Search items (Ctrl+F)',
                prefix: const Icon(Icons.search, size: 18),
                suffix: filter.query.isEmpty
                    ? null
                    : IconButton(
                        tooltip: 'Clear search',
                        icon: const Icon(Icons.close, size: 16),
                        onPressed: () {
                          searchText.clear();
                          controller.update((f) => f.copyWith(query: ''));
                        },
                      ),
              ),
              onChanged: (v) => controller.update((f) => f.copyWith(query: v)),
            ),
          ),
          if (rare)
            FilterChip(
              label: Text('Include ${info.r4}'),
              selected: settings.showFours,
              onSelected: prefs.setShowFours,
            )
          else
            SegmentedButton<int>(
              showSelectedIcon: false,
              segments: [
                const ButtonSegment(value: 0, label: Text('All')),
                ButtonSegment(value: 5, label: Text(info.r5)),
                ButtonSegment(value: 4, label: Text(info.r4)),
              ],
              selected: {filter.rank},
              onSelectionChanged: (s) => controller.update((f) => f.copyWith(rank: s.first)),
            ),
          _DateRangeButton(filter: filter),
          DropdownButtonHideUnderline(
            child: DropdownButton<GroupBy>(
              value: filter.group,
              borderRadius: BorderRadius.circular(8),
              items: const [
                DropdownMenuItem(value: GroupBy.none, child: Text('No grouping')),
                DropdownMenuItem(value: GroupBy.banner, child: Text('Group by banner')),
                DropdownMenuItem(value: GroupBy.month, child: Text('Group by month')),
              ],
              onChanged: (g) => controller.update((f) => f.copyWith(group: g)),
            ),
          ),
          const ViewControls(),
        ]),
        const SizedBox(height: 10),
        Wrap(spacing: 8, runSpacing: 8, crossAxisAlignment: WrapCrossAlignment.center, children: [
          ChoiceChip(
            label: const Text('All banners'),
            selected: filter.bannerKey == null,
            onSelected: (_) => controller.update((f) => f.copyWith(clearBanner: true)),
          ),
          for (final b in meta?.banners ?? const <BannerFilter>[])
            ChoiceChip(
              label: Text(b.label),
              selected: filter.bannerKey == b.key,
              onSelected: (_) => controller.update((f) => f.copyWith(bannerKey: b.key)),
            ),
          const SizedBox(width: 8),
          Semantics(
            liveRegion: true,
            child: Text('${groupDigits(shown)} of ${groupDigits(total)} pulls', style: Ty.small.copyWith(color: c.muted)),
          ),
          if (!filter.isDefault)
            TextButton(
              onPressed: () {
                searchText.clear();
                controller.clear();
              },
              child: const Text('Clear filters'),
            ),
          TextButton.icon(
            onPressed: rows.isEmpty
                ? null
                : () async {
                    await Clipboard.setData(ClipboardData(text: toCsv(rows)));
                    ref.read(toastProvider.notifier).ok('Copied ${plural(rows.length, 'row')} as CSV.');
                  },
            icon: const Icon(Icons.copy_all_outlined, size: 16),
            label: const Text('Copy CSV'),
          ),
        ]),
      ]),
    );
  }
}

class _DateRangeButton extends ConsumerWidget {
  const _DateRangeButton({required this.filter});
  final HistoryFilter filter;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.read(historyFilterProvider.notifier);
    final active = filter.from != null || filter.to != null;
    String day(DateTime d) => '${d.year.toString().padLeft(4, '0')}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
    return OutlinedButton.icon(
      onPressed: () async {
        final picked = await showDateRangePicker(
          context: context,
          firstDate: DateTime(2020),
          lastDate: DateTime.now().add(const Duration(days: 2)),
        );
        if (picked != null) controller.update((f) => f.copyWith(from: day(picked.start), to: day(picked.end)));
      },
      icon: const Icon(Icons.date_range, size: 16),
      label: Text(active ? '${filter.from} → ${filter.to}' : 'Any date'),
    ).wrapClear(
      active,
      () => controller.update((f) => f.copyWith(clearRange: true)),
    );
  }
}

extension on Widget {
  /// Adds a small clear button next to an active control.
  Widget wrapClear(bool active, VoidCallback onClear) => active
      ? Row(mainAxisSize: MainAxisSize.min, children: [this, IconButton(tooltip: 'Clear date range', onPressed: onClear, icon: const Icon(Icons.close, size: 16))])
      : this;
}

// ------------------------------------------------------------------ rare pulls (timeline)

class _RareList extends ConsumerWidget {
  const _RareList({required this.rows, required this.account});
  final List<HistoryPull> rows;
  final Account account;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final filter = ref.watch(historyFilterProvider);
    final view = ref.watch(settingsProvider.select((s) => s.pullView));
    final flat = flatten(rows, filter.group == GroupBy.none ? GroupBy.month : filter.group);
    return LayoutBuilder(builder: (context, box) {
      const gap = 8.0;
      const iconTile = 52.0;
      final inner = box.maxWidth - 48;
      final cols = switch (view) {
        PullView.iconsOnly => tileColumns(inner, iconTile, 6),
        PullView.namesOnly => tileColumns(inner, 300, gap),
        PullView.iconsAndNames => tileColumns(inner, 400, gap),
      };
      // Group headers get a row of their own; the pulls between them are cut into rows of [cols].
      final blocks = <_Block>[];
      var run = <HistoryPull>[];
      void flush() {
        for (var i = 0; i < run.length; i += cols) {
          blocks.add(_Block.pulls(run.sublist(i, i + cols > run.length ? run.length : i + cols)));
        }
        run = <HistoryPull>[];
      }

      for (final r in flat) {
        if (r.isHeader) {
          flush();
          blocks.add(_Block.header(r.header!, r.count));
        } else {
          run.add(r.pull!);
        }
      }
      flush();
      return ListView.builder(
        padding: const EdgeInsets.fromLTRB(24, 8, 24, 24),
        itemCount: blocks.length,
        itemBuilder: (context, i) {
          final b = blocks[i];
          if (b.title != null) return _GroupHeader(title: b.title!, count: b.count);
          return _RareTileRow(pulls: b.pulls, cols: cols, view: view, gap: view == PullView.iconsOnly ? 6 : gap, account: account);
        },
      );
    });
  }
}

/// One entry of the timeline: a group header or a row of pulls.
class _Block {
  const _Block.header(this.title, this.count) : pulls = const [];
  const _Block.pulls(this.pulls)
      : title = null,
        count = 0;
  final String? title;
  final int count;
  final List<HistoryPull> pulls;
}

class _RareTileRow extends StatelessWidget {
  const _RareTileRow({required this.pulls, required this.cols, required this.view, required this.gap, required this.account});
  final List<HistoryPull> pulls;
  final int cols;
  final PullView view;
  final double gap;
  final Account account;

  @override
  Widget build(BuildContext context) {
    final icons = view == PullView.iconsOnly;
    final cells = <Widget>[];
    for (var i = 0; i < cols; i++) {
      if (i > 0) cells.add(SizedBox(width: gap));
      final child = i < pulls.length ? _RareTile(pull: pulls[i], account: account, view: view) : const SizedBox.shrink();
      // Every row has the same number of equal cells, so the columns line up and fill the width.
      cells.add(icons ? SizedBox(width: 52, child: child) : Expanded(child: child));
    }
    return Padding(
      padding: EdgeInsets.only(bottom: gap),
      child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: cells),
    );
  }
}

class _GroupHeader extends StatelessWidget {
  const _GroupHeader({required this.title, required this.count});
  final String title;
  final int count;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    return Padding(
      padding: const EdgeInsets.fromLTRB(0, 16, 0, 6),
      child: Semantics(
        header: true,
        child: Row(children: [
          Flexible(child: Text(title, style: Ty.h3.copyWith(color: c.muted), overflow: TextOverflow.ellipsis)),
          const SizedBox(width: 8),
          Text('$count', style: Ty.small.copyWith(color: c.faint, fontFamily: Ty.mono)),
        ]),
      ),
    );
  }
}

class _RareTile extends ConsumerWidget {
  const _RareTile({required this.pull, required this.account, required this.view});
  final HistoryPull pull;
  final Account account;
  final PullView view;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final facts = ref.watch(pityFactsProvider);
    final selected = ref.watch(selectedPullProvider) == pull.id;
    final hard = facts.hardPity[pull.bannerKey] ?? (pull.rank == 5 ? 90 : 10);
    final odds = facts.odds[pull.bannerKey];
    final key = '${pull.bannerKey}|${pull.name}|${pull.time}|${pull.pity}';
    final outcome = pull.rank == 5 ? facts.outcomes[key] : null;
    final tracked = pull.rank == 5 && odds != null;
    final tint = switch (outcome) {
      PullOutcome.win => c.ok,
      PullOutcome.loss => c.err,
      PullOutcome.guaranteed => c.blue,
      null => c.accent,
    };
    final outcomeIcon = switch (outcome) {
      PullOutcome.win => Icons.check_circle_outline,
      PullOutcome.loss => Icons.cancel_outlined,
      PullOutcome.guaranteed => Icons.lock_outline,
      null => Icons.help_outline,
    };
    final outcomeColor = outcome == null ? c.faint : tint;
    final pity = pull.pity;
    final date = pull.time.length >= 10 ? pull.time.substring(0, 10) : pull.time;
    final words = '${pull.name}, ${rankLabel(account.game, pull.rank)}, pity ${pity ?? 'unknown'}, ${pull.time}'
        '${pull.bannerName == null ? '' : ', ${pull.bannerName}'}${tracked ? '. ${resultWords(outcome, odds)}' : ''}';
    final pityColor = pull.rank == 5 ? tint : c.purple;
    void open() => openDetail(context, ref, pull, account);

    if (view == PullView.iconsOnly) {
      return PullIconTile(
        game: account.game,
        name: pull.name,
        rank: pull.rank,
        pity: pity,
        pityColor: pityColor,
        border: tracked && outcome != null ? tint.withValues(alpha: 0.85) : c.line,
        tooltip: words,
        outcomeIcon: tracked ? outcomeIcon : null,
        outcomeColor: outcomeColor,
        onTap: open,
        selected: selected,
      );
    }

    final nameColor = c.rarity(pull.rank);
    final Widget row;
    if (view == PullView.namesOnly) {
      row = Row(children: [
        Expanded(child: Text(pull.name, maxLines: 1, overflow: TextOverflow.ellipsis, style: Ty.body.copyWith(color: nameColor, fontWeight: FontWeight.w600))),
        const SizedBox(width: 8),
        Text(date, style: Ty.small.copyWith(color: c.muted, fontFamily: Ty.mono)),
        SizedBox(width: 40, child: Text(pity == null ? '–' : '$pity', textAlign: TextAlign.right, style: Ty.num.copyWith(fontWeight: FontWeight.w700, color: pityColor))),
        if (tracked) ...[const SizedBox(width: 6), Icon(outcomeIcon, size: 15, color: outcomeColor)],
      ]);
    } else {
      row = Row(children: [
        ItemIcon(game: account.game, name: pull.name, rank: pull.rank, size: 36),
        const SizedBox(width: 10),
        Expanded(
          flex: 5,
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Row(children: [
              Flexible(child: Text(pull.name, style: Ty.h3.copyWith(color: nameColor), maxLines: 1, overflow: TextOverflow.ellipsis)),
              if (pull.rank != 5) ...[const SizedBox(width: 6), RarityChip(game: account.game, rank: pull.rank)],
            ]),
            Text(
              [pull.bannerName ?? pull.pool, date].join('  ·  '),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Ty.small.copyWith(color: c.muted),
            ),
          ]),
        ),
        const SizedBox(width: 10),
        if (pity != null) Expanded(flex: 2, child: PityBar(value: pity, hard: hard, color: pityColor, height: 6)) else const Spacer(flex: 2),
        SizedBox(width: 38, child: Text(pity == null ? '–' : '$pity', textAlign: TextAlign.right, style: Ty.num.copyWith(fontWeight: FontWeight.w700))),
        if (tracked) ...[const SizedBox(width: 6), Icon(outcomeIcon, size: 16, color: outcomeColor)],
      ]);
    }
    return Tooltip(
      message: words,
      child: Panel(
        selected: selected,
        padding: EdgeInsets.symmetric(horizontal: 10, vertical: view == PullView.namesOnly ? 6 : 7),
        onTap: open,
        child: Semantics(label: words, child: row),
      ),
    );
  }
}

/// Wide windows show the detail panel beside the list; narrow ones use a dialog.
void openDetail(BuildContext context, WidgetRef ref, HistoryPull pull, Account account) {
  ref.read(selectedPullProvider.notifier).select(pull.id);
  if (MediaQuery.sizeOf(context).width >= 1280) return;
  showDialog<void>(
    context: context,
    builder: (ctx) => Dialog(
      backgroundColor: context.wc.panel,
      child: ConstrainedBox(constraints: const BoxConstraints(maxWidth: 420), child: _Detail(pull: pull, account: account, dialog: true)),
    ),
  ).whenComplete(() => ref.read(selectedPullProvider.notifier).select(null));
}

// ------------------------------------------------------------------ all pulls (table)

class _AllTable extends ConsumerWidget {
  const _AllTable({required this.rows, required this.account});
  final List<HistoryPull> rows;
  final Account account;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final filter = ref.watch(historyFilterProvider);
    final rowHeight = Density.of(context).tableRow;
    final flat = flatten(rows, filter.group);
    return Column(children: [
      _TableHeader(filter: filter),
      Expanded(
        child: Scrollbar(
          child: ListView.builder(
            padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
            itemCount: flat.length,
            itemExtentBuilder: (i, dims) => flat[i].isHeader ? 40.0 : rowHeight,
            itemBuilder: (context, i) {
              final r = flat[i];
              if (r.isHeader) return Align(alignment: Alignment.bottomLeft, child: _GroupHeader(title: r.header!, count: r.count));
              return _TableRow(pull: r.pull!, account: account, striped: i.isOdd, c: c);
            },
          ),
        ),
      ),
    ]);
  }
}

class _TableHeader extends ConsumerWidget {
  const _TableHeader({required this.filter});
  final HistoryFilter filter;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final newestFirst = ref.watch(settingsProvider.select((s) => s.newestFirst));
    Widget cell(String label, SortColumn col, {double? width, int flex = 0, TextAlign align = TextAlign.left}) {
      final active = filter.sort == col;
      // Time is ordered by the same persisted choice as every other list; the other columns keep their own direction.
      final ascending = col == SortColumn.time ? !newestFirst : filter.ascending;
      final inner = InkWell(
        onTap: () {
          if (col == SortColumn.time) {
            if (active) {
              ref.read(settingsProvider.notifier).setNewestFirst(!newestFirst);
            } else {
              ref.read(historyFilterProvider.notifier).update((f) => f.copyWith(sort: SortColumn.time));
            }
            return;
          }
          ref.read(historyFilterProvider.notifier).update((f) => f.copyWith(
                sort: col,
                ascending: active ? !f.ascending : (col == SortColumn.item),
              ));
        },
        child: Row(mainAxisAlignment: align == TextAlign.right ? MainAxisAlignment.end : MainAxisAlignment.start, children: [
          Flexible(child: Text(label, style: Ty.label.copyWith(color: active ? c.text : c.faint), overflow: TextOverflow.ellipsis)),
          if (active) Icon(ascending ? Icons.arrow_upward : Icons.arrow_downward, size: 12, color: c.text),
        ]),
      );
      final sem = Semantics(
        button: true,
        label: 'Sort by $label',
        sortKey: null,
        child: inner,
      );
      return width != null ? SizedBox(width: width, child: sem) : Expanded(flex: flex, child: sem);
    }

    return Container(
      height: 34,
      padding: const EdgeInsets.symmetric(horizontal: 24),
      decoration: BoxDecoration(color: c.rail, border: Border(bottom: BorderSide(color: c.line))),
      child: Row(children: [
        cell('TIME', SortColumn.time, width: 160),
        cell('ITEM', SortColumn.item, flex: 3),
        cell('RARITY', SortColumn.rarity, width: 90),
        cell('PITY', SortColumn.pity, width: 70, align: TextAlign.right),
        const SizedBox(width: 20),
        Expanded(flex: 4, child: Text('BANNER', style: Ty.label.copyWith(color: c.faint))),
      ]),
    );
  }
}

class _TableRow extends ConsumerWidget {
  const _TableRow({required this.pull, required this.account, required this.striped, required this.c});
  final HistoryPull pull;
  final Account account;
  final bool striped;
  final WishColors c;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selected = ref.watch(selectedPullProvider) == pull.id;
    final view = ref.watch(settingsProvider.select((s) => s.pullView));
    final rarity = c.rarity(pull.rank);
    final nameStyle = TextStyle(color: pull.rank >= 4 ? rarity : c.text, fontWeight: pull.rank >= 4 ? FontWeight.w600 : FontWeight.w400);
    return InkWell(
      onTap: () => openDetail(context, ref, pull, account),
      child: Container(
        color: selected ? c.accent.withValues(alpha: 0.14) : (striped ? c.panel.withValues(alpha: 0.5) : null),
        child: Row(children: [
          const SizedBox(width: 0),
          SizedBox(width: 160, child: Text(pull.time, style: Ty.num.copyWith(fontSize: 12.5, color: c.muted))),
          Expanded(
            flex: 3,
            child: Row(children: [
              if (view != PullView.namesOnly) ...[
                Tooltip(message: pull.name, child: ItemIcon(game: account.game, name: pull.name, rank: pull.rank, size: 20)),
                if (view == PullView.iconsAndNames) const SizedBox(width: 8),
              ],
              if (view != PullView.iconsOnly) Expanded(child: Text(pull.name, overflow: TextOverflow.ellipsis, style: nameStyle)),
            ]),
          ),
          SizedBox(width: 90, child: Text(rankLabel(account.game, pull.rank), style: TextStyle(color: rarity))),
          SizedBox(width: 70, child: Text(pull.pity == null ? '' : '${pull.pity}', textAlign: TextAlign.right, style: Ty.num)),
          const SizedBox(width: 20),
          Expanded(flex: 4, child: Text(pull.bannerName ?? pull.pool, overflow: TextOverflow.ellipsis, style: TextStyle(color: c.muted))),
        ]),
      ),
    );
  }
}

// ------------------------------------------------------------------ detail

class _Detail extends ConsumerWidget {
  const _Detail({required this.pull, required this.account, this.dialog = false});
  final HistoryPull pull;
  final Account account;
  final bool dialog;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final facts = ref.watch(pityFactsProvider);
    final odds = facts.odds[pull.bannerKey];
    final outcome = pull.rank == 5 ? facts.outcomes['${pull.bannerKey}|${pull.name}|${pull.time}|${pull.pity}'] : null;

    Widget row(String k, String v, {bool mono = false}) => Padding(
          padding: const EdgeInsets.symmetric(vertical: 5),
          child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
            SizedBox(width: 92, child: Text(k, style: Ty.small.copyWith(color: c.faint))),
            Expanded(child: SelectableText(v, style: mono ? Ty.num.copyWith(fontSize: 12.5) : Ty.body)),
          ]),
        );

    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
        Row(children: [
          ItemIcon(game: account.game, name: pull.name, rank: pull.rank, size: 48),
          const SizedBox(width: 12),
          Expanded(child: Text(pull.name, style: Ty.h2.copyWith(color: c.rarity(pull.rank)))),
          IconButton(
            tooltip: 'Close',
            icon: const Icon(Icons.close, size: 18),
            onPressed: () {
              ref.read(selectedPullProvider.notifier).select(null);
              if (dialog) Navigator.of(context).pop();
            },
          ),
        ]),
        const SizedBox(height: 10),
        Wrap(spacing: 6, children: [
          RarityChip(game: account.game, rank: pull.rank),
          if (pull.rank == 5 && odds != null) OutcomeChip(outcome: outcome, odds: odds),
        ]),
        const SizedBox(height: 12),
        row('Time', '${pull.time}  (${utcLabel(account.timezone)})', mono: true),
        row('Pity', pull.pity == null ? 'n/a' : '${pull.pity} pulls since the last ${pull.rank == 4 ? '${GameInfo.of(account.game).r4} or ${GameInfo.of(account.game).r5}' : GameInfo.of(account.game).r5}'),
        row('Pool', pull.pool),
        row('Banner', pull.bannerName ?? 'Not matched to a banner event'),
        row('Type', pull.itemType),
        row('Record id', pull.id, mono: true),
        const SizedBox(height: 6),
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton.icon(
            onPressed: () async {
              await Clipboard.setData(ClipboardData(text: pull.id));
              ref.read(toastProvider.notifier).ok('Record id copied.');
            },
            icon: const Icon(Icons.copy, size: 14),
            label: const Text('Copy record id'),
          ),
        ),
      ]),
    );
  }
}
