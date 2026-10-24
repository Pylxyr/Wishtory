import { invoke } from '@tauri-apps/api/core';
import type { Account, BannerPity, FetchSummary, GameCatalog, GameId, GameInstall, ImportSummary, LogEntry, Pull } from './types';

export const errMsg = (e: unknown): string => (typeof e === 'string' ? e : e instanceof Error ? e.message : 'Something went wrong.');

export const api = {
  discoverGames: () => invoke<GameInstall[]>('discover_games'),
  extractAuthkey: (game: GameId) => invoke<string>('extract_authkey', { game }),
  refreshFromGame: (game: GameId) => invoke<FetchSummary>('refresh_from_game', { game }),
  fetchHistory: (game: GameId, url: string) => invoke<FetchSummary>('fetch_history', { game, url }),
  listAccounts: () => invoke<Account[]>('list_accounts'),
  deleteAccount: (game: GameId, uid: string) => invoke<void>('delete_account', { game, uid }),
  clearAllHistory: () => invoke<number>('clear_all_history'),
  getHistory: (game: GameId, uid: string, banner: string | null) => invoke<Pull[]>('get_history', { game, uid, banner }),
  getPity: (game: GameId, uid: string) => invoke<BannerPity[]>('get_pity', { game, uid }),
  getCatalog: () => invoke<GameCatalog>('get_catalog'),
  refreshCatalog: () => invoke<string>('refresh_catalog'),
  importFile: (path: string) => invoke<ImportSummary>('import_file', { path }),
  exportFile: (path: string, format: string, game: GameId | null, uid: string | null) =>
    invoke<void>('export_file', { path, format, game, uid }),
  setLoggingEnabled: (enabled: boolean) => invoke<void>('set_logging_enabled', { enabled }),
  getLogs: () => invoke<LogEntry[]>('get_logs'),
  clearLogs: () => invoke<void>('clear_logs')
};
