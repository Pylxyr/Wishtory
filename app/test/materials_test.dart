import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/domain/materials.dart';

const _sample = '''
{"schema":1,"books":[
 {"id":"teachings_of_freedom","name":"Freedom","title":"Teachings of Freedom","days":[1,4],"users":[{"name":"Venti","rarity":5},{"name":"Amber","rarity":4}]},
 {"id":"teachings_of_gold","name":"Gold","days":[3,6],"users":[]}],
 "weapons":[{"id":"w","name":"Decarabian","title":"Piece of Decarabian","days":[1,4],"users":[{"name":"Sword","rarity":4}]}]}
''';

void main() {
  test('the sample parses, and a missing title falls back to the name', () {
    final data = MaterialsData.parse(_sample);
    expect(data.books.map((s) => s.name), ['Freedom', 'Gold']);
    expect(data.books[1].title, 'Gold');
    expect(data.books[0].users.map((u) => '${u.name}${u.rarity}'), ['Venti5', 'Amber4']);
    expect(data.weapons.single.days, [1, 4]);
  });

  test('a series opens on its days, and everything opens on Sunday', () {
    final data = MaterialsData.parse(_sample);
    expect(data.booksOn(DateTime.monday).map((s) => s.name), ['Freedom']);
    expect(data.booksOn(DateTime.wednesday).map((s) => s.name), ['Gold']);
    expect(data.booksOn(DateTime.tuesday), isEmpty);
    expect(data.booksOn(DateTime.sunday).map((s) => s.name), ['Freedom', 'Gold']);
    expect(data.weaponsOn(DateTime.thursday), hasLength(1));
    expect(data.weaponsOn(DateTime.friday), isEmpty);
  });

  group('the farming day', () {
    test('Asia: 11:00 on a Monday is Monday, and the day turns over at 04:00', () {
      final day = farmDay(8, DateTime.utc(2026, 11, 16, 3));
      expect(day.weekday, DateTime.monday);
      expect(day.msUntilReset, 17 * 3600000);
    });

    test('before 04:00 it is still the day before', () {
      // 02:00 Monday in Asia.
      final day = farmDay(8, DateTime.utc(2026, 11, 15, 18));
      expect(day.weekday, DateTime.sunday);
      expect(day.msUntilReset, 2 * 3600000);
    });

    test('Europe and America use their own clocks', () {
      expect(farmDay(1, DateTime.utc(2026, 11, 16, 2, 30)).weekday, DateTime.sunday);
      expect(farmDay(1, DateTime.utc(2026, 11, 16, 2, 30)).msUntilReset, 30 * 60000);
      // Exactly 04:00 Monday in America: the new day has just begun.
      final america = farmDay(-5, DateTime.utc(2026, 11, 16, 9));
      expect(america.weekday, DateTime.monday);
      expect(america.msUntilReset, 24 * 3600000);
    });

    test('the same instant can be a different day on different servers', () {
      final instant = DateTime.utc(2026, 11, 16, 1);
      // 09:00 on Monday in Asia.
      expect(farmDay(8, instant).weekday, DateTime.monday);
      // 20:00 on Sunday in America.
      expect(farmDay(-5, instant).weekday, DateTime.sunday);
    });

    test('a month end rolls into the next month', () {
      final day = farmDay(8, DateTime.utc(2026, 11, 30, 20));
      expect(day.weekday, DateTime.tuesday);
      expect(day.msUntilReset, 24 * 3600000);
    });
  });

  test('labels', () {
    expect(serverLabel(8), 'Asia, TW/HK/MO (UTC+8)');
    expect(serverLabel(1), 'Europe (UTC+1)');
    expect(serverLabel(-5), 'America (UTC\u22125)');
    expect(serverLabel(9), 'UTC+9');
    expect(untilReset(11580000), '3 h 13 min');
    expect(untilReset(45 * 60000), '45 min');
    expect(untilReset(20000), '1 min');
    expect(untilReset(3600000), '1 h 0 min');
    expect(daysLabel([2, 5]), 'Tue · Fri');
    expect(weekdayNames, hasLength(7));
    expect(weekdayShort.first, 'Mon');
  });

  group('the bundled schedule', () {
    final data = MaterialsData.parse(File('assets/data/materials.json').readAsStringSync());
    final catalog = (jsonDecode(File('../catalog/catalog.json').readAsStringSync()) as Map<String, dynamic>)['items'] as List<dynamic>;
    final genshin = {
      for (final i in catalog.cast<Map<String, dynamic>>())
        if (i['game'] == 'hk4e') '${i['kind']}|${i['name']}': i['rarity'] as int,
    };

    test('every series has two weekdays and a name', () {
      for (final s in [...data.books, ...data.weapons]) {
        expect(s.days, hasLength(2), reason: s.id);
        expect(s.days.every((d) => d >= 1 && d <= 6), isTrue, reason: s.id);
        expect(s.name, isNotEmpty);
        expect(s.title, isNotEmpty);
      }
      expect(data.books.length, greaterThanOrEqualTo(10));
      expect(data.weapons.length, greaterThanOrEqualTo(10));
    });

    test('each weekday offers talent books and weapon materials', () {
      for (var d = 1; d <= 7; d++) {
        expect(data.booksOn(d), isNotEmpty, reason: 'books on day $d');
        expect(data.weaponsOn(d), isNotEmpty, reason: 'weapons on day $d');
      }
    });

    test('a character or weapon belongs to one series only', () {
      for (final series in [data.books, data.weapons]) {
        final seen = <String>{};
        for (final s in series) {
          for (final u in s.users) {
            expect(seen.add(u.name), isTrue, reason: '${u.name} is listed twice');
          }
        }
      }
    });

    test('every name is in the catalog with the same rarity, so its picture can be found', () {
      for (final s in data.books) {
        for (final u in s.users) {
          expect(genshin['character|${u.name}'], u.rarity, reason: u.name);
        }
      }
      for (final s in data.weapons) {
        for (final u in s.users) {
          expect(genshin['weapon|${u.name}'], u.rarity, reason: u.name);
        }
      }
    });
  });
}
