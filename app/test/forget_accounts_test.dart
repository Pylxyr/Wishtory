import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/state/nav.dart';
import 'package:wishtory/state/providers.dart';
import 'package:wishtory/state/settings.dart';

Future<ProviderContainer> launch(Map<String, Object> saved) async {
  SharedPreferences.setMockInitialValues(saved);
  final prefs = await SharedPreferences.getInstance();
  final container = ProviderContainer(overrides: [sharedPrefsProvider.overrideWithValue(prefs)]);
  addTearDown(container.dispose);
  return container;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const saved = <String, Object>{
    'aliases': '{"hk4e|800000001":"Main"}',
    'lastRefresh': '{"hk4e|800000001":1}',
    'firstFiveAnswers': '{"hk4e|800000001|Venti|2024-01-01 00:00:00":"won"}',
    'nav.uids': '{"hk4e":"800000001","hkrpg":"700000002"}',
  };

  test('clearing all history forgets every nickname, refresh time, answer and selected account', () async {
    final container = await launch(saved);
    expect(container.read(settingsProvider).aliases, isNotEmpty);
    container.read(settingsProvider.notifier).forgetAllAccounts();
    container.read(navProvider.notifier).forgetAllAccounts();
    expect(container.read(settingsProvider).aliases, isEmpty);
    expect(container.read(settingsProvider).lastRefresh, isEmpty);
    expect(container.read(settingsProvider).firstFiveAnswers, isEmpty);
    expect(container.read(navProvider).uids, isEmpty);
    final prefs = container.read(sharedPrefsProvider);
    for (final key in ['aliases', 'lastRefresh', 'firstFiveAnswers', 'nav.uids']) {
      expect(prefs.containsKey(key), isFalse, reason: key);
    }
  });

  test('deleting one account stops remembering it as the selected one, and only it', () async {
    final container = await launch(saved);
    final nav = container.read(navProvider.notifier);
    nav.forgetAccount(GameId.genshin, '999999999'); // not the selected one: nothing changes
    expect(container.read(navProvider).uids, {'hk4e': '800000001', 'hkrpg': '700000002'});
    nav.forgetAccount(GameId.genshin, '800000001');
    expect(container.read(navProvider).uids, {'hkrpg': '700000002'});
    expect(container.read(sharedPrefsProvider).getString('nav.uids'), contains('hkrpg'));
    expect(container.read(sharedPrefsProvider).getString('nav.uids'), isNot(contains('hk4e')));
  });
}
