import 'dart:convert';

import 'package:flutter/services.dart' show rootBundle;

import 'planning.dart';

/// A character or weapon that uses a series of materials.
class MaterialUser {
  const MaterialUser(this.name, this.rarity);
  final String name;
  final int rarity;
}

/// Talent books or weapon materials that share a domain and the days it offers them. Monday = 1 ...
/// Saturday = 6; Sunday is never listed because every domain offers everything on Sundays.
class MaterialSeries {
  const MaterialSeries({required this.id, required this.name, required this.title, required this.days, required this.users});

  final String id;

  /// The short name ("Freedom") and the first item's full name ("Teachings of Freedom").
  final String name, title;
  final List<int> days;
  final List<MaterialUser> users;

  bool opensOn(int weekday) => weekday == DateTime.sunday || days.contains(weekday);
}

/// Which Genshin talent books and weapon materials can be farmed on which weekday (`assets/data/materials.json`,
/// written by `scripts/update-materials.mjs`).
class MaterialsData {
  const MaterialsData({required this.books, required this.weapons});

  final List<MaterialSeries> books, weapons;

  static Future<MaterialsData> load() async => parse(await rootBundle.loadString('assets/data/materials.json'));

  static MaterialsData parse(String source) {
    final raw = jsonDecode(source) as Map<String, dynamic>;
    List<MaterialSeries> series(String key) => [
          for (final s in (raw[key] as List<dynamic>? ?? const []).cast<Map<String, dynamic>>())
            MaterialSeries(
              id: s['id'] as String,
              name: s['name'] as String,
              title: (s['title'] as String?) ?? s['name'] as String,
              days: [for (final d in (s['days'] as List<dynamic>)) d as int],
              users: [
                for (final u in (s['users'] as List<dynamic>).cast<Map<String, dynamic>>()) MaterialUser(u['name'] as String, (u['rarity'] as num).toInt()),
              ],
            ),
        ];
    return MaterialsData(books: series('books'), weapons: series('weapons'));
  }

  List<MaterialSeries> booksOn(int weekday) => [
        for (final s in books)
          if (s.opensOn(weekday)) s,
      ];

  List<MaterialSeries> weaponsOn(int weekday) => [
        for (final s in weapons)
          if (s.opensOn(weekday)) s,
      ];
}

/// The game's farming day starts at 04:00 server time.
const dailyResetHour = 4;

/// Which farming day it is on a server, and how long until the next one starts.
class FarmDay {
  const FarmDay(this.weekday, this.msUntilReset);

  /// 1 = Monday ... 7 = Sunday.
  final int weekday;
  final int msUntilReset;
}

/// The farming day for a server [tzHours] from UTC (8 for Asia and TW/HK/MO, 1 for Europe, -5 for America).
/// Before 04:00 it is still the previous day, as in the game.
FarmDay farmDay(int tzHours, [DateTime? now]) {
  final clock = serverClockMs(tzHours, now);
  final shifted = DateTime.fromMillisecondsSinceEpoch(clock - dailyResetHour * 3600000, isUtc: true);
  final next = DateTime.utc(shifted.year, shifted.month, shifted.day + 1, dailyResetHour);
  return FarmDay(shifted.weekday, next.millisecondsSinceEpoch - clock);
}

const weekdayNames = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
const weekdayShort = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

/// "Asia (UTC+8)", "Europe (UTC+1)", "America (UTC-5)"; any other offset is shown as plain UTC.
String serverLabel(int tzHours) {
  final utc = 'UTC${tzHours >= 0 ? '+' : '−'}${tzHours.abs()}';
  return switch (tzHours) {
    8 => 'Asia, TW/HK/MO ($utc)',
    1 => 'Europe ($utc)',
    -5 => 'America ($utc)',
    _ => utc,
  };
}

/// "3 h 12 min", "45 min".
String untilReset(int ms) {
  final min = (ms ~/ 60000) < 1 ? 1 : ms ~/ 60000;
  final h = min ~/ 60;
  return h >= 1 ? '$h h ${min % 60} min' : '$min min';
}

/// "Mon · Thu", the days a series opens besides Sunday.
String daysLabel(List<int> days) => days.map((d) => weekdayShort[d - 1]).join(' · ');
