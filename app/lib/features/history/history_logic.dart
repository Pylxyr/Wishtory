import '../../bridge/wishtory_api.dart';

enum GroupBy { none, banner, month }

enum SortColumn { time, item, rarity, pity }

/// Everything the History toolbar can narrow or reorder. Pure data so the logic below is testable.
class HistoryFilter {
  const HistoryFilter({
    this.bannerKey,
    this.rank = 0,
    this.query = '',
    this.group = GroupBy.none,
    this.sort = SortColumn.time,
    this.ascending = false,
    this.from,
    this.to,
  });

  /// One of the keys from `GameMeta.banners`; null for every banner.
  final String? bannerKey;

  /// 0 = all, 5 or 4.
  final int rank;
  final String query;
  final GroupBy group;
  final SortColumn sort;
  final bool ascending;

  /// Inclusive day range on the pull's own server date ("YYYY-MM-DD").
  final String? from;
  final String? to;

  bool get isDefault => bannerKey == null && rank == 0 && query.isEmpty && from == null && to == null;

  HistoryFilter copyWith({
    String? bannerKey,
    bool clearBanner = false,
    int? rank,
    String? query,
    GroupBy? group,
    SortColumn? sort,
    bool? ascending,
    String? from,
    String? to,
    bool clearRange = false,
  }) =>
      HistoryFilter(
        bannerKey: clearBanner ? null : (bannerKey ?? this.bannerKey),
        rank: rank ?? this.rank,
        query: query ?? this.query,
        group: group ?? this.group,
        sort: sort ?? this.sort,
        ascending: ascending ?? this.ascending,
        from: clearRange ? null : (from ?? this.from),
        to: clearRange ? null : (to ?? this.to),
      );
}

/// [rare]: only 5★ (and 4★ when [includeFours]); otherwise the rarity filter applies.
List<HistoryPull> applyFilters(List<HistoryPull> all, HistoryFilter f, {required bool rare, required bool includeFours}) {
  final q = f.query.trim().toLowerCase();
  final out = <HistoryPull>[];
  for (final p in all) {
    if (rare) {
      if (p.rank != 5 && !(includeFours && p.rank == 4)) continue;
    } else if (f.rank != 0 && p.rank != f.rank) {
      continue;
    }
    if (f.bannerKey != null && p.bannerKey != f.bannerKey) continue;
    if (q.isNotEmpty && !p.name.toLowerCase().contains(q)) continue;
    final day = p.time.length >= 10 ? p.time.substring(0, 10) : p.time;
    if (f.from != null && day.compareTo(f.from!) < 0) continue;
    if (f.to != null && day.compareTo(f.to!) > 0) continue;
    out.add(p);
  }
  if (!rare) _sort(out, f);
  return out;
}

void _sort(List<HistoryPull> rows, HistoryFilter f) {
  // The backend sends newest first; that is already the default order.
  if (f.sort == SortColumn.time) {
    if (f.ascending) {
      final reversed = rows.reversed.toList();
      rows
        ..clear()
        ..addAll(reversed);
    }
    return;
  }
  int cmp(HistoryPull a, HistoryPull b) => switch (f.sort) {
        SortColumn.item => a.name.toLowerCase().compareTo(b.name.toLowerCase()),
        SortColumn.rarity => a.rank.compareTo(b.rank),
        SortColumn.pity => (a.pity ?? -1).compareTo(b.pity ?? -1),
        SortColumn.time => 0,
      };
  // Stable on ties: Dart's List.sort is not guaranteed stable, so tie-break on the original index.
  final index = {for (var i = 0; i < rows.length; i++) rows[i].id: i};
  rows.sort((a, b) {
    final c = cmp(a, b);
    final r = c != 0 ? c : (index[a.id]! - index[b.id]!);
    return f.ascending ? r : (c != 0 ? -c : r);
  });
}

/// A row of the flattened list: either a group header or a pull.
class HistoryRow {
  const HistoryRow.header(this.header, this.count) : pull = null;
  const HistoryRow.pull(this.pull)
      : header = null,
        count = 0;

  final String? header;
  final int count;
  final HistoryPull? pull;

  bool get isHeader => header != null;
}

List<HistoryRow> flatten(List<HistoryPull> rows, GroupBy group) {
  if (group == GroupBy.none) return [for (final p in rows) HistoryRow.pull(p)];
  String keyOf(HistoryPull p) => switch (group) {
        GroupBy.banner => p.bannerName ?? 'Unplaced',
        GroupBy.month => p.time.length >= 7 ? p.time.substring(0, 7) : 'Unknown',
        GroupBy.none => '',
      };
  final order = <String>[];
  final buckets = <String, List<HistoryPull>>{};
  for (final p in rows) {
    final k = keyOf(p);
    (buckets[k] ??= (() {
      order.add(k);
      return <HistoryPull>[];
    })())
        .add(p);
  }
  if (group == GroupBy.banner) {
    // "Unplaced" last; the other groups keep the order of their newest pull.
    order.sort((a, b) => (a == 'Unplaced' ? 1 : 0) - (b == 'Unplaced' ? 1 : 0));
  }
  return [
    for (final k in order) ...[
      HistoryRow.header(k, buckets[k]!.length),
      for (final p in buckets[k]!) HistoryRow.pull(p),
    ],
  ];
}

String csvEscape(String v) => v.contains(RegExp(r'[",\n\r]')) ? '"${v.replaceAll('"', '""')}"' : v;

String toCsv(List<HistoryPull> rows) {
  final b = StringBuffer('time,item,rarity,pity,pool,banner,record_id\n');
  for (final p in rows) {
    b.writeln([p.time, p.name, '${p.rank}', '${p.pity ?? ''}', p.pool, p.bannerName ?? '', p.id].map(csvEscape).join(','));
  }
  return b.toString();
}
