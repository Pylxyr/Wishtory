import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/features/history/history_logic.dart';

HistoryPull pull(String id, String name, int rank, String time, {int? pity, String key = 'character', String? banner}) => HistoryPull(
      id: id,
      gachaType: '301',
      gachaId: '',
      itemId: '1',
      name: name,
      itemType: 'Character',
      time: time,
      rank: rank,
      pool: 'Character',
      bannerKey: key,
      bannerName: banner,
      pity: pity,
    );

void main() {
  final all = [
    pull('3', 'Venti', 5, '2024-03-02 10:00:00', pity: 80, banner: 'B2'),
    pull('2', 'Amber', 4, '2024-02-20 09:00:00', pity: 3, banner: 'B1'),
    pull('1', 'Diluc', 5, '2024-02-01 08:00:00', pity: 60, key: 'standard'),
  ];

  test('rare mode keeps 5★ and optionally 4★', () {
    expect(applyFilters(all, const HistoryFilter(), rare: true, includeFours: false).map((p) => p.id), ['3', '1']);
    expect(applyFilters(all, const HistoryFilter(), rare: true, includeFours: true).length, 3);
  });

  test('banner, search and date range narrow the list', () {
    expect(applyFilters(all, const HistoryFilter(bannerKey: 'standard'), rare: false, includeFours: false).single.id, '1');
    expect(applyFilters(all, const HistoryFilter(query: 'ven'), rare: false, includeFours: false).single.id, '3');
    final inFeb = applyFilters(all, const HistoryFilter(from: '2024-02-01', to: '2024-02-29'), rare: false, includeFours: false);
    expect(inFeb.map((p) => p.id), ['2', '1']);
  });

  test('sorting by pity descending and time ascending', () {
    final byPity = applyFilters(all, const HistoryFilter(sort: SortColumn.pity), rare: false, includeFours: false);
    expect(byPity.map((p) => p.pity), [80, 60, 3]);
    final oldest = applyFilters(all, const HistoryFilter(ascending: true), rare: false, includeFours: false);
    expect(oldest.map((p) => p.id), ['1', '2', '3']);
  });

  test('grouping by banner puts unplaced pulls last', () {
    final rows = flatten(all, GroupBy.banner);
    final headers = rows.where((r) => r.isHeader).map((r) => r.header).toList();
    expect(headers, ['B2', 'B1', 'Unplaced']);
  });

  test('csv escapes commas and quotes', () {
    expect(csvEscape('a,b'), '"a,b"');
    expect(csvEscape('say "hi"'), '"say ""hi"""');
    expect(toCsv([all.first]).split('\n')[1], startsWith('2024-03-02 10:00:00,Venti,5,80,'));
  });
}
