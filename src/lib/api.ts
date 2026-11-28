import type * as T from './types';
import type { GameId } from './types';

// In the app every call goes to a Rust command (src-tauri/src/commands.rs). In a plain browser, started
// with VITE_MOCK=1, the same calls are answered by src/lib/mock.ts, which is how the UI is looked at
// without the game or the Rust build.
type Invoke = <R>(cmd: string, args?: Record<string, unknown>) => Promise<R>;
type ChannelCtor = new <M>(onmessage?: (m: M) => void) => { onmessage: (m: M) => void };

let backend: Promise<{ invoke: Invoke; Channel: ChannelCtor; save: (o: object) => Promise<string | null>; open: (o: object) => Promise<string | null> }> | null = null;

function load() {
  backend ??= (async () => {
    if (import.meta.env.VITE_MOCK === '1') {
      const m = await import('./mock');
      return { invoke: m.invoke as Invoke, Channel: m.MockChannel as unknown as ChannelCtor, save: async () => '/tmp/export.json', open: async () => '/tmp/import.json' };
    }
    const core = await import('@tauri-apps/api/core');
    const dialog = await import('@tauri-apps/plugin-dialog');
    return {
      invoke: core.invoke as Invoke, Channel: core.Channel as unknown as ChannelCtor,
      save: async (o) => (await dialog.save(o)) as string | null, open: async (o) => (await dialog.open(o)) as string | null
    };
  })();
  return backend;
}
const call = async <R>(cmd: string, args?: Record<string, unknown>) => (await load()).invoke<R>(cmd, args);

/** Every command throws an `ApiError` (`{ code, message }`); anything else is wrapped into one. */
export function asError(e: unknown): T.ApiError {
  if (e && typeof e === 'object' && 'message' in e && 'code' in e) return e as T.ApiError;
  return { code: 'other', message: typeof e === 'string' ? e : e instanceof Error ? e.message : 'Something went wrong.' };
}

async function refresh(cmd: string, args: Record<string, unknown>, onEvent: (e: T.RefreshEvent) => void) {
  const b = await load();
  const channel = new b.Channel<T.RefreshEvent>();
  channel.onmessage = onEvent;
  await b.invoke(cmd, { ...args, onEvent: channel });
}

export const api = {
  startupError: () => call<T.ApiError | null>('startup_error'),
  discoverGames: () => call<T.GameInstall[]>('discover_games'),
  getGameLocations: () => call<T.GameLocations>('get_game_locations'),
  setGameLocations: (locations: string[]) => call<T.GameLocations>('set_game_locations', { locations }),
  refreshFromGame: (game: GameId, full: boolean, onEvent: (e: T.RefreshEvent) => void) => refresh('refresh_from_game', { game, full }, onEvent),
  fetchHistory: (game: GameId, url: string, full: boolean, onEvent: (e: T.RefreshEvent) => void) => refresh('fetch_history', { game, url, full }, onEvent),
  listAccounts: () => call<T.Account[]>('list_accounts'),
  deleteAccount: (game: GameId, uid: string) => call<void>('delete_account', { game, uid }),
  clearAllHistory: () => call<number>('clear_all_history'),
  /** `minRank` 4 leaves out the 3★ pulls and 5 keeps only the 5★ ones, so they are never sent over. */
  getHistory: (game: GameId, uid: string, banner: string | null, minRank: number | null = null) =>
    call<T.HistoryPull[]>('get_history', { game, uid, banner, minRank }),
  getPity: (game: GameId, uid: string) => call<T.BannerPity[]>('get_pity', { game, uid }),
  getCatalog: () => call<T.GameCatalog>('get_catalog'),
  getBanners: () => call<T.BannerList>('get_banners'),
  getCatalogStatus: () => call<T.CatalogStatus>('get_catalog_status'),
  checkCatalog: (force: boolean) => call<T.CatalogCheck>('check_catalog', { force }),
  getCatalogReview: () => call<T.CatalogReview | null>('get_catalog_review'),
  getImpactPage: (game: GameId, uid: string, offset: number, limit: number) => call<T.AffectedPull[]>('get_impact_page', { game, uid, offset, limit }),
  applyCatalog: (version: number, fingerprint: string) => call<T.ApplyResult>('apply_catalog', { version, fingerprint }),
  dismissCatalog: () => call<void>('dismiss_catalog'),
  getCatalogHistory: () => call<T.CatalogHistoryEntry[]>('get_catalog_history'),
  setCatalogAutoApply: (enabled: boolean) => call<boolean>('set_catalog_auto_apply', { enabled }),
  cloudStatus: () => call<T.CloudStatus>('cloud_status'),
  cloudSignIn: (passphrase: string) => call<T.CloudStatus>('cloud_sign_in', { passphrase }),
  cloudCancelSignIn: () => call<void>('cloud_cancel_sign_in'),
  cloudSignOut: (deleteBackups: boolean) => call<T.CloudStatus>('cloud_sign_out', { deleteBackups }),
  cloudSetAutoBackup: (enabled: boolean) => call<T.CloudStatus>('cloud_set_auto_backup', { enabled }),
  cloudBackupNow: () => call<T.CloudBackup>('cloud_backup_now'),
  cloudBackupIfDue: () => call<T.CloudBackup | null>('cloud_backup_if_due'),
  cloudListBackups: () => call<T.CloudBackup[]>('cloud_list_backups'),
  cloudRestore: (id: string, passphrase: string | null) => call<T.ImportSummary>('cloud_restore', { id, passphrase }),
  importFile: (path: string) => call<T.ImportSummary>('import_file', { path }),
  exportFile: (path: string, format: string, game: GameId | null, uid: string | null) => call<void>('export_file', { path, format, game, uid }),
  gameMeta: () => call<T.GameMeta[]>('game_meta'),
  getAbout: () => call<T.AboutInfo>('get_about'),
  openExternal: (url: string) => call<void>('open_external', { url }),
  checkAppUpdate: () => call<T.AppUpdate>('check_app_update'),
  setLoggingEnabled: (enabled: boolean) => call<void>('set_logging_enabled', { enabled }),
  getLogs: () => call<T.LogEntry[]>('get_logs'),
  clearLogs: () => call<void>('clear_logs'),
  pickSavePath: async (name: string, ext: string) => (await load()).save({ defaultPath: name, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] }),
  pickOpenPath: async () => (await load()).open({ multiple: false, filters: [{ name: 'Wish history', extensions: ['json', 'xlsx'] }] })
};
