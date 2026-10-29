import { invoke } from '@tauri-apps/api/core';
import type { AboutInfo, Account, AppUpdate, BannerPity, BannerSet, BannerUpdate, FetchSummary, GameCatalog, GameId, GameInstall, GameMeta, ImportSummary, LogEntry, Pull } from './types';

export const errMsg = (e: unknown): string => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Something went wrong.');

export const api = {
  discoverGames: () => invoke<GameInstall[]>('discover_games'),
  extractAuthkey: (game: GameId) => invoke<string>('extract_authkey', { game }),
  // `full` downloads everything the server still keeps; by default a fetch stops at the first page that is already saved.
  refreshFromGame: (game: GameId, full = false) => invoke<FetchSummary>('refresh_from_game', { game, full }),
  fetchHistory: (game: GameId, url: string, full = false) => invoke<FetchSummary>('fetch_history', { game, url, full }),
  listAccounts: () => invoke<Account[]>('list_accounts'),
  deleteAccount: (game: GameId, uid: string) => invoke<void>('delete_account', { game, uid }),
  clearAllHistory: () => invoke<number>('clear_all_history'),
  getHistory: (game: GameId, uid: string, banner: string | null) => invoke<Pull[]>('get_history', { game, uid, banner }),
  getPity: (game: GameId, uid: string) => invoke<BannerPity[]>('get_pity', { game, uid }),
  getGameMeta: () => invoke<GameMeta[]>('get_game_meta'),
  getCatalog: () => invoke<GameCatalog>('get_catalog'),
  refreshCatalog: (force: boolean) => invoke<string>('refresh_catalog', { force }),
  getBanners: () => invoke<BannerSet>('get_banners'),
  getBannerUpdate: () => invoke<BannerUpdate | null>('get_banner_update'),
  refreshBanners: (force: boolean) => invoke<BannerUpdate>('refresh_banners', { force }),
  importFile: (path: string) => invoke<ImportSummary>('import_file', { path }),
  exportFile: (path: string, format: string, game: GameId | null, uid: string | null) =>
    invoke<void>('export_file', { path, format, game, uid }),
  getAbout: () => invoke<AboutInfo>('get_about'),
  openExternal: (url: string) => invoke<void>('open_external', { url }),
  checkAppUpdate: () => invoke<AppUpdate>('check_app_update'),
  setLoggingEnabled: (enabled: boolean) => invoke<void>('set_logging_enabled', { enabled }),
  getLogs: () => invoke<LogEntry[]>('get_logs'),
  clearLogs: () => invoke<void>('clear_logs')
};
