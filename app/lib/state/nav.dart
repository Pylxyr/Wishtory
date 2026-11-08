import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../bridge/wishtory_api.dart';
import '../domain/game_info.dart';
import 'providers.dart';

enum AppPage { overview, history, importExport, accounts, settings, help, about, logs }

extension AppPageInfo on AppPage {
  String get title => switch (this) {
        AppPage.overview => 'Overview',
        AppPage.history => 'History',
        AppPage.importExport => 'Import / Export',
        AppPage.accounts => 'Accounts',
        AppPage.settings => 'Settings',
        AppPage.help => 'Help',
        AppPage.about => 'About',
        AppPage.logs => 'Logs',
      };

  /// Pages about one game's data show the game and account bar.
  bool get isGamePage => this == AppPage.overview || this == AppPage.history;
}

class NavState {
  const NavState({required this.page, required this.game, required this.uids});

  final AppPage page;
  final GameId game;

  /// Chosen account per game key.
  final Map<String, String> uids;

  NavState copyWith({AppPage? page, GameId? game, Map<String, String>? uids}) =>
      NavState(page: page ?? this.page, game: game ?? this.game, uids: uids ?? this.uids);
}

/// Page, game and account; remembered between launches.
class NavController extends Notifier<NavState> {
  static const _kPage = 'nav.page';
  static const _kGame = 'nav.game';
  static const _kUids = 'nav.uids';

  @override
  NavState build() {
    final p = ref.read(sharedPrefsProvider);
    final page = AppPage.values.asNameMap()[p.getString(_kPage)] ?? AppPage.overview;
    var uids = <String, String>{};
    try {
      uids = (jsonDecode(p.getString(_kUids) ?? '{}') as Map<String, dynamic>).map((k, v) => MapEntry(k, v as String));
    } catch (_) {}
    return NavState(
      page: page == AppPage.logs ? AppPage.overview : page,
      game: gameFromKey(p.getString(_kGame)) ?? GameId.genshin,
      uids: uids,
    );
  }

  void goTo(AppPage page) {
    state = state.copyWith(page: page);
    ref.read(sharedPrefsProvider).setString(_kPage, page.name);
  }

  void selectGame(GameId game) {
    state = state.copyWith(game: game);
    ref.read(sharedPrefsProvider).setString(_kGame, gameKey(game));
  }

  void selectAccount(GameId game, String uid) {
    final uids = {...state.uids, gameKey(game): uid};
    state = state.copyWith(game: game, uids: uids);
    final p = ref.read(sharedPrefsProvider);
    p.setString(_kGame, gameKey(game));
    p.setString(_kUids, jsonEncode(uids));
  }

  /// Show a specific account's data (used after a refresh or an import).
  void showAccount(Account a) {
    selectAccount(a.game, a.uid);
  }
}

final navProvider = NotifierProvider<NavController, NavState>(NavController.new);
