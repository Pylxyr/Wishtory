import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../bridge/wishtory_api.dart';
import '../domain/recovery.dart';
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
    unawaited(runCloudBackupIfDue(ref.read(apiProvider)));
    state = RefreshState(
      game: game,
      result: RefreshResult(game: s.game, uid: s.uid, fetched: s.fetched, added: s.added, warning: s.warning),
    );
  }

  void dismiss() => state = RefreshState(game: state.game);
}

final refreshProvider = NotifierProvider<RefreshController, RefreshState>(RefreshController.new);

/// The once-a-day game catalog check and the app update check, started when the window opens. Failures are
/// silent: the bundled copy keeps working offline. A release that changes none of the saved pulls is applied
/// by itself; any other one waits and shows the start-up notice.
Future<void> runStartupTasks(WidgetRef ref) async {
  await ref.read(catalogControllerProvider.notifier).check(silent: true);
  unawaited(ref.read(appUpdateProvider.notifier).check(silent: true));
  unawaited(runCloudBackupIfDue(ref.read(apiProvider)));
}

class CatalogUiState {
  const CatalogUiState({this.checking = false, this.applying = false});
  final bool checking;
  final bool applying;
  bool get busy => checking || applying;
}

/// The game catalog actions: check, apply and "Not now". Each reloads what changed.
class CatalogController extends Notifier<CatalogUiState> {
  @override
  CatalogUiState build() => const CatalogUiState();

  Future<void> _reloadAll({required bool dataChanged}) async {
    await ref.read(catalogStatusProvider.notifier).reload();
    if (dataChanged) {
      await ref.read(catalogProvider.notifier).reload();
      await ref.read(bannersProvider.notifier).reload();
      ref.read(dataTickProvider.notifier).bump();
      ref.invalidate(catalogHistoryProvider);
    }
  }

  /// [silent] is the start-up check: no messages, and the once-a-day gate applies.
  Future<void> check({bool silent = false}) async {
    if (state.busy) return;
    state = const CatalogUiState(checking: true);
    final toast = ref.read(toastProvider.notifier);
    try {
      final result = await ref.read(apiProvider).checkCatalog(force: !silent);
      if (!ref.mounted) return;
      await _reloadAll(dataChanged: result.status == CheckStatus.applied);
      if (!silent) {
        switch (result.status) {
          case CheckStatus.upToDate:
            toast.ok('The game catalog is up to date.');
          case CheckStatus.alreadyChecking:
            break;
          case CheckStatus.notReady:
            toast.show('A newer catalog is being published. Nothing changed; Wishtory will try again later.');
          case CheckStatus.applied:
            toast.ok('The game catalog was updated. None of your saved pulls changed.');
          case CheckStatus.needsReview:
            toast.show('A catalog update is ready. It would change some of your saved pulls, so please review it.');
        }
      }
    } catch (e) {
      if (ref.mounted && !silent) toast.error(recoveryFor(e).detail);
    } finally {
      if (ref.mounted) state = const CatalogUiState();
    }
  }

  /// Applies the release the person just reviewed. Returns true when it was applied.
  Future<bool> apply(CatalogReview review) async {
    if (state.busy) return false;
    state = const CatalogUiState(applying: true);
    final toast = ref.read(toastProvider.notifier);
    try {
      final result = await ref.read(apiProvider).applyCatalog(version: review.version, fingerprint: review.fingerprint);
      if (!ref.mounted) return false;
      await _reloadAll(dataChanged: result.applied);
      if (result.applied) {
        toast.ok('The game catalog was updated to version ${review.version}.');
      } else if (result.stale) {
        ref.invalidate(catalogReviewProvider);
        toast.show('Your saved pulls changed while you were reviewing, so the list was refreshed. Nothing was applied. Please review it again.');
      }
      return result.applied;
    } catch (e) {
      if (ref.mounted) toast.error(recoveryFor(e).detail);
      return false;
    } finally {
      if (ref.mounted) state = const CatalogUiState();
    }
  }

  /// "Not now": the release stays waiting; the start-up notice stops, the Settings badge stays.
  Future<void> dismiss() async {
    try {
      await ref.read(apiProvider).dismissCatalog();
    } catch (_) {}
    if (ref.mounted) await ref.read(catalogStatusProvider.notifier).reload();
  }

  Future<void> setAutoApply(bool enabled) async {
    try {
      await ref.read(apiProvider).setCatalogAutoApply(enabled);
    } catch (e) {
      if (ref.mounted) ref.read(toastProvider.notifier).error(recoveryFor(e).detail);
    }
    if (ref.mounted) await ref.read(catalogStatusProvider.notifier).reload();
  }
}

final catalogControllerProvider = NotifierProvider<CatalogController, CatalogUiState>(CatalogController.new);

/// The waiting release's change log and affected accounts; loaded when the review page opens.
final catalogReviewProvider = FutureProvider.autoDispose<CatalogReview?>((ref) => ref.watch(apiProvider).getCatalogReview());

/// The applied releases, newest first.
final catalogHistoryProvider = FutureProvider.autoDispose<List<CatalogHistoryEntry>>((ref) => ref.watch(apiProvider).getCatalogHistory());

/// The automatic cloud backup. Does nothing unless the person switched it on and a day has passed,
/// and stays silent when it cannot run: the Settings page shows the state and any problem.
Future<void> runCloudBackupIfDue(WishtoryApi api) async {
  try {
    await api.cloudBackupIfDue();
  } catch (_) {}
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
