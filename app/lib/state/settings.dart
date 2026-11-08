import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../bridge/wishtory_api.dart';
import '../domain/game_info.dart';
import 'providers.dart';

enum HistoryMode { rare, all }

class AppSettings {
  const AppSettings({
    this.themeMode = ThemeMode.system,
    this.comfortable = false,
    this.historyMode = HistoryMode.rare,
    this.showFours = false,
    this.tutorialDone = false,
    this.loggingEnabled = false,
    this.aliases = const {},
    this.lastRefresh = const {},
  });

  final ThemeMode themeMode;

  /// Comfortable (44 px) rows instead of compact (32 px).
  final bool comfortable;
  final HistoryMode historyMode;
  final bool showFours;
  final bool tutorialDone;
  final bool loggingEnabled;

  /// Local display names, by "game|uid".
  final Map<String, String> aliases;

  /// Epoch milliseconds of the last successful refresh on this PC, by "game|uid".
  final Map<String, int> lastRefresh;

  AppSettings copyWith({
    ThemeMode? themeMode,
    bool? comfortable,
    HistoryMode? historyMode,
    bool? showFours,
    bool? tutorialDone,
    bool? loggingEnabled,
    Map<String, String>? aliases,
    Map<String, int>? lastRefresh,
  }) =>
      AppSettings(
        themeMode: themeMode ?? this.themeMode,
        comfortable: comfortable ?? this.comfortable,
        historyMode: historyMode ?? this.historyMode,
        showFours: showFours ?? this.showFours,
        tutorialDone: tutorialDone ?? this.tutorialDone,
        loggingEnabled: loggingEnabled ?? this.loggingEnabled,
        aliases: aliases ?? this.aliases,
        lastRefresh: lastRefresh ?? this.lastRefresh,
      );
}

String accountKey(GameId game, String uid) => '${gameKey(game)}|$uid';

class SettingsController extends Notifier<AppSettings> {
  @override
  AppSettings build() {
    final p = ref.read(sharedPrefsProvider);
    Map<String, T> map<T>(String key) {
      try {
        return (jsonDecode(p.getString(key) ?? '{}') as Map<String, dynamic>).map((k, v) => MapEntry(k, v as T));
      } catch (_) {
        return <String, T>{};
      }
    }

    return AppSettings(
      themeMode: ThemeMode.values.asNameMap()[p.getString('theme')] ?? ThemeMode.system,
      comfortable: p.getBool('comfortable') ?? false,
      historyMode: HistoryMode.values.asNameMap()[p.getString('historyMode')] ?? HistoryMode.rare,
      showFours: p.getBool('showFours') ?? false,
      tutorialDone: p.getBool('tutorialDone') ?? false,
      loggingEnabled: false, // diagnostics are opt-in per session, as before
      aliases: map<String>('aliases'),
      lastRefresh: map<int>('lastRefresh'),
    );
  }

  void setTheme(ThemeMode mode) {
    state = state.copyWith(themeMode: mode);
    ref.read(sharedPrefsProvider).setString('theme', mode.name);
  }

  void setComfortable(bool value) {
    state = state.copyWith(comfortable: value);
    ref.read(sharedPrefsProvider).setBool('comfortable', value);
  }

  void setHistoryMode(HistoryMode mode) {
    state = state.copyWith(historyMode: mode);
    ref.read(sharedPrefsProvider).setString('historyMode', mode.name);
  }

  void setShowFours(bool value) {
    state = state.copyWith(showFours: value);
    ref.read(sharedPrefsProvider).setBool('showFours', value);
  }

  void finishTutorial() {
    state = state.copyWith(tutorialDone: true);
    ref.read(sharedPrefsProvider).setBool('tutorialDone', true);
  }

  Future<void> setLogging(bool enabled) async {
    state = state.copyWith(loggingEnabled: enabled);
    await ref.read(apiProvider).setLoggingEnabled(enabled);
  }

  void setAlias(GameId game, String uid, String name) {
    final aliases = {...state.aliases};
    final key = accountKey(game, uid);
    if (name.trim().isEmpty) {
      aliases.remove(key);
    } else {
      aliases[key] = name.trim();
    }
    state = state.copyWith(aliases: aliases);
    ref.read(sharedPrefsProvider).setString('aliases', jsonEncode(aliases));
  }

  void markRefreshed(GameId game, String uid) {
    final map = {...state.lastRefresh, accountKey(game, uid): DateTime.now().millisecondsSinceEpoch};
    state = state.copyWith(lastRefresh: map);
    ref.read(sharedPrefsProvider).setString('lastRefresh', jsonEncode(map));
  }

  void forgetAccount(GameId game, String uid) {
    final key = accountKey(game, uid);
    final aliases = {...state.aliases}..remove(key);
    final last = {...state.lastRefresh}..remove(key);
    state = state.copyWith(aliases: aliases, lastRefresh: last);
    final p = ref.read(sharedPrefsProvider);
    p.setString('aliases', jsonEncode(aliases));
    p.setString('lastRefresh', jsonEncode(last));
  }
}

final settingsProvider = NotifierProvider<SettingsController, AppSettings>(SettingsController.new);

/// "Nickname" the person gave an account, else the in-game nickname, else the UID.
String accountName(AppSettings s, Account a) {
  final alias = s.aliases[accountKey(a.game, a.uid)];
  if (alias != null && alias.isNotEmpty) return alias;
  return a.nickname.isNotEmpty ? a.nickname : a.uid;
}
