import 'generated/api/core_api.dart' as rs;
import 'generated/api/core_api.dart' show WishtoryCore, RefreshEvent;
import 'generated/api/types.dart';

export 'generated/api/core_api.dart' show RefreshEvent, RefreshEventKind;
export 'generated/api/error.dart' show ApiError, ErrorCode;
export 'generated/api/types.dart';

/// Everything the UI can ask of the Rust core. Widgets and providers depend on this interface only,
/// so tests can substitute a fake without loading the native library.
abstract class WishtoryApi {
  Future<List<GameInstall>> discoverGames();
  Future<GameLocations> getGameLocations();
  Future<GameLocations> setGameLocations(List<String> locations);

  /// Reads the game's cache (read-only) and downloads. The stream ends with a finished or failed event.
  Stream<RefreshEvent> refreshFromGame(GameId game, {bool full = false});
  Stream<RefreshEvent> fetchHistory(GameId game, String url, {bool full = false});

  Future<List<Account>> listAccounts();
  Future<void> deleteAccount(GameId game, String uid);
  Future<int> clearAllHistory();

  /// Newest first.
  Future<List<HistoryPull>> getHistory(GameId game, String uid);
  Future<List<BannerPity>> getPity(GameId game, String uid);

  Future<GameCatalog> getCatalog();
  Future<UpdateStatus> refreshCatalog({bool force = false});
  Future<BannerList> getBanners();
  Future<BannerUpdate?> getBannerUpdate();
  Future<BannerUpdate> refreshBanners({bool force = false});

  Future<ImportSummary> importFile(String path);
  Future<void> exportFile(String path, String format, {GameId? game, String? uid});

  /// Cloud backup to the person's own Google Drive, encrypted on this PC.
  Future<CloudStatus> cloudStatus();

  /// Opens Google's sign-in in the browser and completes when the person is done (or gives up).
  Future<CloudStatus> cloudSignIn(String passphrase);
  Future<void> cloudCancelSignIn();
  Future<CloudStatus> cloudSignOut({required bool deleteBackups});
  Future<CloudStatus> cloudSetAutoBackup(bool enabled);
  Future<CloudBackup> cloudBackupNow();

  /// The automatic backup: does nothing unless it is switched on and a day has passed.
  Future<CloudBackup?> cloudBackupIfDue();

  /// Newest first.
  Future<List<CloudBackup>> cloudListBackups();
  Future<ImportSummary> cloudRestore(String id, {String? passphrase});

  Future<List<GameMeta>> gameMeta();
  Future<AboutInfo> getAbout();
  Future<void> openExternal(String url);
  Future<AppUpdate> checkAppUpdate();

  Future<void> setLoggingEnabled(bool enabled);
  Future<List<LogEntry>> getLogs();
  Future<void> clearLogs();
}

class RustWishtoryApi implements WishtoryApi {
  RustWishtoryApi(this._core);

  final WishtoryCore _core;

  static Future<RustWishtoryApi> open({String? dataDir}) async => RustWishtoryApi(await WishtoryCore.open(dataDir: dataDir));

  @override
  Future<List<GameInstall>> discoverGames() => _core.discoverGames();
  @override
  Future<GameLocations> getGameLocations() => _core.getGameLocations();
  @override
  Future<GameLocations> setGameLocations(List<String> locations) => _core.setGameLocations(locations: locations);

  @override
  Stream<RefreshEvent> refreshFromGame(GameId game, {bool full = false}) => _core.refreshFromGame(game: game, full: full);
  @override
  Stream<RefreshEvent> fetchHistory(GameId game, String url, {bool full = false}) =>
      _core.fetchHistory(game: game, url: url, full: full);

  @override
  Future<List<Account>> listAccounts() => _core.listAccounts();
  @override
  Future<void> deleteAccount(GameId game, String uid) => _core.deleteAccount(game: game, uid: uid);
  @override
  Future<int> clearAllHistory() async => (await _core.clearAllHistory()).toInt();

  @override
  Future<List<HistoryPull>> getHistory(GameId game, String uid) => _core.getHistory(game: game, uid: uid);
  @override
  Future<List<BannerPity>> getPity(GameId game, String uid) => _core.getPity(game: game, uid: uid);

  @override
  Future<GameCatalog> getCatalog() => _core.getCatalog();
  @override
  Future<UpdateStatus> refreshCatalog({bool force = false}) => _core.refreshCatalog(force: force);
  @override
  Future<BannerList> getBanners() => _core.getBanners();
  @override
  Future<BannerUpdate?> getBannerUpdate() => _core.getBannerUpdate();
  @override
  Future<BannerUpdate> refreshBanners({bool force = false}) => _core.refreshBanners(force: force);

  @override
  Future<ImportSummary> importFile(String path) => _core.importFile(path: path);
  @override
  Future<void> exportFile(String path, String format, {GameId? game, String? uid}) =>
      _core.exportFile(path: path, format: format, game: game, uid: uid);

  @override
  Future<CloudStatus> cloudStatus() => _core.cloudStatus();
  @override
  Future<CloudStatus> cloudSignIn(String passphrase) => _core.cloudSignIn(passphrase: passphrase);
  @override
  Future<void> cloudCancelSignIn() => _core.cloudCancelSignIn();
  @override
  Future<CloudStatus> cloudSignOut({required bool deleteBackups}) => _core.cloudSignOut(deleteBackups: deleteBackups);
  @override
  Future<CloudStatus> cloudSetAutoBackup(bool enabled) => _core.cloudSetAutoBackup(enabled: enabled);
  @override
  Future<CloudBackup> cloudBackupNow() => _core.cloudBackupNow();
  @override
  Future<CloudBackup?> cloudBackupIfDue() => _core.cloudBackupIfDue();
  @override
  Future<List<CloudBackup>> cloudListBackups() => _core.cloudListBackups();
  @override
  Future<ImportSummary> cloudRestore(String id, {String? passphrase}) => _core.cloudRestore(id: id, passphrase: passphrase);

  @override
  Future<List<GameMeta>> gameMeta() => rs.gameMeta();
  @override
  Future<AboutInfo> getAbout() => rs.getAbout();
  @override
  Future<void> openExternal(String url) => rs.openExternal(url: url);
  @override
  Future<AppUpdate> checkAppUpdate() => rs.checkAppUpdate();

  @override
  Future<void> setLoggingEnabled(bool enabled) => rs.setLoggingEnabled(enabled: enabled);
  @override
  Future<List<LogEntry>> getLogs() => rs.getLogs();
  @override
  Future<void> clearLogs() => rs.clearLogs();
}
