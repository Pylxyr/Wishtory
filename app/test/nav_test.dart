import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/state/nav.dart';
import 'package:wishtory/state/providers.dart';

Future<SharedPreferences> prefsWith([Map<String, Object> saved = const {}]) async {
  SharedPreferences.setMockInitialValues(saved);
  return SharedPreferences.getInstance();
}

ProviderContainer launch(SharedPreferences prefs) {
  final container = ProviderContainer(overrides: [sharedPrefsProvider.overrideWithValue(prefs)]);
  addTearDown(container.dispose);
  return container;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('starting up', () {
    test('a first launch shows the Genshin overview', () async {
      final nav = launch(await prefsWith()).read(navProvider);
      expect(nav.page, AppPage.overview);
      expect(nav.game, GameId.genshin);
      expect(nav.gamePage, AppPage.overview);
    });

    test('the saved page and game come back', () async {
      final nav = launch(await prefsWith({'nav.page': 'history', 'nav.game': 'hkrpg'})).read(navProvider);
      expect(nav.page, AppPage.history);
      expect(nav.gamePage, AppPage.history);
    });

    test('launching on Settings remembers Overview as the game page', () async {
      final nav = launch(await prefsWith({'nav.page': 'settings'})).read(navProvider);
      expect(nav.page, AppPage.settings);
      expect(nav.gamePage, AppPage.overview);
    });

    test('the logs page is never a start page', () async {
      final nav = launch(await prefsWith({'nav.page': 'logs'})).read(navProvider);
      expect(nav.page, AppPage.overview);
    });
  });

  group('openGame', () {
    test('on a game page it switches the game and keeps the page', () async {
      final container = launch(await prefsWith());
      final controller = container.read(navProvider.notifier);
      controller.goTo(AppPage.history);
      controller.openGame(GameId.starRail);
      final nav = container.read(navProvider);
      expect(nav.game, GameId.starRail);
      expect(nav.page, AppPage.history);
    });

    test('from Settings it leaves Settings for the game page that was open last', () async {
      final container = launch(await prefsWith());
      final controller = container.read(navProvider.notifier);
      controller.goTo(AppPage.history);
      controller.goTo(AppPage.settings);
      controller.openGame(GameId.zenless);
      final nav = container.read(navProvider);
      expect(nav.game, GameId.zenless);
      expect(nav.page, AppPage.history);
    });

    test('from About, Accounts, Help and Import / Export it also opens the game', () async {
      for (final page in [AppPage.about, AppPage.accounts, AppPage.help, AppPage.importExport, AppPage.logs]) {
        final container = launch(await prefsWith());
        final controller = container.read(navProvider.notifier);
        controller.goTo(page);
        controller.openGame(GameId.starRail);
        final nav = container.read(navProvider);
        expect(nav.page.isGamePage, isTrue, reason: 'from ${page.name}');
        expect(nav.game, GameId.starRail, reason: 'from ${page.name}');
      }
    });

    test('with no game page visited yet it opens the overview', () async {
      final container = launch(await prefsWith({'nav.page': 'about'}));
      container.read(navProvider.notifier).openGame(GameId.genshin);
      expect(container.read(navProvider).page, AppPage.overview);
    });

    test('the choice is remembered for the next launch', () async {
      final prefs = await prefsWith();
      final first = launch(prefs);
      first.read(navProvider.notifier)
        ..goTo(AppPage.history)
        ..goTo(AppPage.settings)
        ..openGame(GameId.zenless);
      final next = launch(prefs).read(navProvider);
      expect(next.game, GameId.zenless);
      expect(next.page, AppPage.history);
    });
  });

  group('selectGame', () {
    test('changes the game without leaving the page', () async {
      final container = launch(await prefsWith());
      final controller = container.read(navProvider.notifier);
      controller.goTo(AppPage.importExport);
      controller.selectGame(GameId.starRail);
      final nav = container.read(navProvider);
      expect(nav.game, GameId.starRail);
      expect(nav.page, AppPage.importExport);
    });
  });

  group('goTo', () {
    test('only game pages change which game page is remembered', () async {
      final container = launch(await prefsWith());
      final controller = container.read(navProvider.notifier);
      controller.goTo(AppPage.history);
      controller.goTo(AppPage.accounts);
      expect(container.read(navProvider).gamePage, AppPage.history);
      controller.goTo(AppPage.overview);
      expect(container.read(navProvider).gamePage, AppPage.overview);
    });
  });
}
