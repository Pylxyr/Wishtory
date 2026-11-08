import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../bridge/wishtory_api.dart';
import '../domain/game_info.dart';
import '../domain/icons.dart';
import 'loadable.dart';
import 'nav.dart';

/// Overridden in `main` with the real instances.
final sharedPrefsProvider = Provider<SharedPreferences>((ref) => throw UnimplementedError('sharedPrefsProvider'));
final apiProvider = Provider<WishtoryApi>((ref) => throw UnimplementedError('apiProvider'));
final iconTableProvider = Provider<IconTable>((ref) => throw UnimplementedError('iconTableProvider'));

/// Bumped after anything that changes saved pulls (refresh, import, delete), so pity and history reload.
class DataTick extends Notifier<int> {
  @override
  int build() => 0;
  void bump() => state++;
}

final dataTickProvider = NotifierProvider<DataTick, int>(DataTick.new);

/// A value loaded once (and again on [reload]). Errors are kept, never thrown into the widget tree.
abstract class OnceLoader<T> extends Notifier<Loadable<T>> {
  Future<T> load(WishtoryApi api);

  bool _disposed = false;

  @override
  Loadable<T> build() {
    _disposed = false;
    ref.onDispose(() => _disposed = true);
    Future.microtask(reload);
    return const Loadable(loading: true);
  }

  Future<void> reload() async {
    final previous = state.data;
    state = Loadable(data: previous, loading: true);
    try {
      final value = await load(ref.read(apiProvider));
      if (!_disposed) state = Loadable(data: value);
    } catch (e) {
      if (!_disposed) state = Loadable(data: previous, error: e);
    }
  }
}

class AccountsLoader extends OnceLoader<List<Account>> {
  @override
  Future<List<Account>> load(WishtoryApi api) => api.listAccounts();
}

class CatalogLoader extends OnceLoader<GameCatalog> {
  @override
  Future<GameCatalog> load(WishtoryApi api) => api.getCatalog();
}

class BannersLoader extends OnceLoader<BannerList> {
  @override
  Future<BannerList> load(WishtoryApi api) => api.getBanners();
}

class MetaLoader extends OnceLoader<List<GameMeta>> {
  @override
  Future<List<GameMeta>> load(WishtoryApi api) => api.gameMeta();
}

final accountsProvider = NotifierProvider<AccountsLoader, Loadable<List<Account>>>(AccountsLoader.new);
final catalogProvider = NotifierProvider<CatalogLoader, Loadable<GameCatalog>>(CatalogLoader.new);
final bannersProvider = NotifierProvider<BannersLoader, Loadable<BannerList>>(BannersLoader.new);
final metaProvider = NotifierProvider<MetaLoader, Loadable<List<GameMeta>>>(MetaLoader.new);

/// Item name → catalog entry, rebuilt when the catalog changes.
final catalogIndexProvider = Provider<CatalogIndex>((ref) => CatalogIndex(ref.watch(catalogProvider).data));

/// The accounts of the selected game, in the order the backend lists them.
final gameAccountsProvider = Provider<List<Account>>((ref) {
  final game = ref.watch(navProvider.select((n) => n.game));
  final all = ref.watch(accountsProvider).data ?? const <Account>[];
  return [
    for (final a in all)
      if (a.game == game) a,
  ];
});

/// The account every game page shows: the one chosen for this game, else the first.
final currentAccountProvider = Provider<Account?>((ref) {
  final nav = ref.watch(navProvider);
  final accounts = ref.watch(gameAccountsProvider);
  if (accounts.isEmpty) return null;
  final wanted = nav.uids[gameKey(nav.game)];
  for (final a in accounts) {
    if (a.uid == wanted) return a;
  }
  return accounts.first;
});

/// Loads something per (game, account) and reloads when either, or the data tick, changes.
/// Previous data for the *same* account stays visible while reloading; another account never
/// shows the previous account's numbers.
abstract class AccountLoader<T> extends Notifier<Loadable<T>> {
  Future<T> load(WishtoryApi api, GameId game, String uid);

  int _gen = 0;
  bool _disposed = false;
  T? _last;
  String? _lastKey;

  @override
  Loadable<T> build() {
    _disposed = false;
    ref.onDispose(() => _disposed = true);
    final account = ref.watch(currentAccountProvider);
    ref.watch(dataTickProvider);
    final gen = ++_gen;
    if (account == null) {
      _last = null;
      _lastKey = null;
      return const Loadable();
    }
    final key = '${gameKey(account.game)}|${account.uid}';
    final keep = key == _lastKey ? _last : null;
    final api = ref.read(apiProvider);
    Future(() async {
      try {
        final value = await load(api, account.game, account.uid);
        if (_disposed || gen != _gen) return;
        _last = value;
        _lastKey = key;
        state = Loadable(data: value);
      } catch (e) {
        if (_disposed || gen != _gen) return;
        state = Loadable(data: key == _lastKey ? _last : null, error: e);
      }
    });
    return Loadable(data: keep, loading: true);
  }
}

class PityLoader extends AccountLoader<List<BannerPity>> {
  @override
  Future<List<BannerPity>> load(WishtoryApi api, GameId game, String uid) => api.getPity(game, uid);
}

class HistoryLoader extends AccountLoader<List<HistoryPull>> {
  @override
  Future<List<HistoryPull>> load(WishtoryApi api, GameId game, String uid) => api.getHistory(game, uid);
}

final pityProvider = NotifierProvider<PityLoader, Loadable<List<BannerPity>>>(PityLoader.new);
final historyProvider = NotifierProvider<HistoryLoader, Loadable<List<HistoryPull>>>(HistoryLoader.new);
