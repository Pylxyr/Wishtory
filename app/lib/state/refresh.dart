import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../bridge/wishtory_api.dart';
import 'loadable.dart';
import 'nav.dart';
import 'providers.dart';
import 'settings.dart';
import 'toasts.dart';

class RefreshResult {
  const RefreshResult({required this.game, required this.uid, required this.fetched, required this.added, this.warning});
  final GameId game;
  final String uid;
  final int fetched;
  final int added;
  final String? warning;
}

class RefreshState {
  const RefreshState({this.busy = false, this.game, this.lines = const [], this.result, this.error});

  final bool busy;
  final GameId? game;

  /// Latest status lines while running (one per banner; see [mergeProgress]).
  final List<String> lines;
  final RefreshResult? result;
  final Object? error;
}

/// A line like "Character Event: page 3" replaces the previous line of the same banner instead of
/// stacking up a hundred lines.
List<String> mergeProgress(List<String> lines, String next) {
  String head(String s) {
    final i = s.indexOf(':');
    return i > 0 ? s.substring(0, i) : s;
  }

  final out = [...lines];
  if (out.isNotEmpty && out.last.contains(':') && head(out.last) == head(next)) {
    out[out.length - 1] = next;
  } else {
    out.add(next);
  }
  return out.length > 6 ? out.sublist(out.length - 6) : out;
}

class RefreshController extends Notifier<RefreshState> {
  @override
  RefreshState build() => const RefreshState();

  /// Refresh the selected game from its cache, or from a pasted [url].
  Future<void> run({bool full = false, String? url}) async {
    if (state.busy) return;
    final game = ref.read(navProvider).game;
    state = RefreshState(busy: true, game: game, lines: const ['Starting…']);
    final api = ref.read(apiProvider);
    final events = url == null ? api.refreshFromGame(game, full: full) : api.fetchHistory(game, url, full: full);
    try {
      await for (final e in events) {
        switch (e.kind) {
          case RefreshEventKind.progress:
            state = RefreshState(busy: true, game: game, lines: mergeProgress(state.lines, e.message));
          case RefreshEventKind.finished:
            await _finished(game, e.summary!);
            return;
          case RefreshEventKind.failed:
            state = RefreshState(game: game, error: e.error ?? e.message);
            return;
        }
      }
      if (state.busy) state = RefreshState(game: game);
    } catch (e) {
      state = RefreshState(game: game, error: e);
    }
  }

  Future<void> _finished(GameId game, FetchSummary s) async {
    ref.read(settingsProvider.notifier).markRefreshed(s.game, s.uid);
    await ref.read(accountsProvider.notifier).reload();
    ref.read(navProvider.notifier).selectAccount(s.game, s.uid);
    ref.read(dataTickProvider.notifier).bump();
    state = RefreshState(
      game: game,
      result: RefreshResult(game: s.game, uid: s.uid, fetched: s.fetched, added: s.added, warning: s.warning),
    );
  }

  void dismiss() => state = RefreshState(game: state.game);
}

final refreshProvider = NotifierProvider<RefreshController, RefreshState>(RefreshController.new);

/// Once-a-day signed feed refreshes (catalog, pity rules, banner list) and the app update check,
/// started when the window opens. Failures are silent: the bundled copies keep working offline.
Future<void> runStartupTasks(WidgetRef ref) async {
  final api = ref.read(apiProvider);
  try {
    final status = await api.refreshCatalog();
    if (status == UpdateStatus.updated) {
      await ref.read(catalogProvider.notifier).reload();
      ref.read(dataTickProvider.notifier).bump();
    }
  } catch (_) {}
  try {
    final update = await api.refreshBanners();
    if (update.status == UpdateStatus.updated) {
      await ref.read(bannersProvider.notifier).reload();
      ref.read(dataTickProvider.notifier).bump();
    }
  } catch (_) {}
  unawaited(ref.read(appUpdateProvider.notifier).check(silent: true));
}

class AppUpdateController extends Notifier<Loadable<AppUpdate>> {
  @override
  Loadable<AppUpdate> build() => const Loadable();

  Future<void> check({bool silent = false}) async {
    if (state.loading) return;
    final p = ref.read(sharedPrefsProvider);
    final now = DateTime.now().millisecondsSinceEpoch;
    if (silent && now - (p.getInt('lastUpdateCheck') ?? 0) < 24 * 3600 * 1000) return;
    state = Loadable(data: state.data, loading: true);
    try {
      final update = await ref.read(apiProvider).checkAppUpdate();
      if (!ref.mounted) return;
      await p.setInt('lastUpdateCheck', now);
      state = Loadable(data: update);
    } catch (e) {
      if (!ref.mounted) return;
      state = Loadable(data: state.data, error: silent ? null : e);
      if (!silent) ref.read(toastProvider.notifier).error('Could not check for updates.');
    }
  }
}

final appUpdateProvider = NotifierProvider<AppUpdateController, Loadable<AppUpdate>>(AppUpdateController.new);
