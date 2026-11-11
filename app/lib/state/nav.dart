import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../bridge/wishtory_api.dart';
import '../domain/game_info.dart';
import 'providers.dart';

enum AppPage { overview, history, importExport, accounts, settings, help, about, logs, catalogReview }

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
        AppPage.catalogReview => 'Game catalog update',
      };

  /// Pages about one game's data show the game and account bar.
  bool get isGamePage => this == AppPage.overview || this == AppPage.history;
}

class NavState {
  const NavState({required this.page, required this.game, required this.uids, this.gamePage = AppPage.overview});

  final AppPage page;
  final GameId game;

  /// Chosen account per game key.
  final Map<String, String> uids;

  /// Which game page (Overview or History) was open last. Picking a game from Settings, About and the
  /// other pages goes here.
  final AppPage gamePage;

  NavState copyWith({AppPage? page, GameId? game, Map<String, String>? uids, AppPage? gamePage}) => NavState(
        page: page ?? this.page,
        game: game ?? this.game,
        uids: uids ?? this.uids,
        gamePage: gamePage ?? this.gamePage,
      );
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
    final shown = page == AppPage.logs || page == AppPage.catalogReview ? AppPage.overview : page;
    return NavState(
      page: shown,
      game: gameFromKey(p.getString(_kGame)) ?? GameId.genshin,
      uids: uids,
      gamePage: shown.isGamePage ? shown : AppPage.overview,
    );
  }

  void goTo(AppPage page) {
    state = state.copyWith(page: page, gamePage: page.isGamePage ? page : null);
    ref.read(sharedPrefsProvider).setString(_kPage, page.name);
  }

  /// Show [game] and its data. On Overview or History this only switches the game and keeps the page.
  /// From any other page (Settings, About, Accounts, Help...) it also leaves that page, for the game page
  /// that was open last, so choosing a game always visibly does something.
  void openGame(GameId game) {
    final page = state.page.isGamePage ? state.page : state.gamePage;
    state = state.copyWith(game: game, page: page);
    final p = ref.read(sharedPrefsProvider);
    p.setString(_kGame, gameKey(game));
    p.setString(_kPage, page.name);
  }

  /// Switch the selected game without leaving the current page (for pages that have their own game
  /// picker, like Import / Export).
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
