import { api, errMsg } from './api';
import type { Account, FetchSummary, GameCatalog, GameId, GameInstall } from './types';

export type Page = 'dashboard' | 'history' | 'accounts' | 'importexport' | 'help';
interface Toast { id: number; kind: 'ok' | 'err' | 'info' | 'warn'; text: string; }

export const app = $state({
  page: 'dashboard' as Page,
  game: 'hk4e' as GameId,
  uid: null as string | null,
  accounts: [] as Account[],
  installs: [] as GameInstall[],
  catalog: null as GameCatalog | null,
  busy: false,
  refreshKey: 0, // bump to make pages reload their data
  toasts: [] as Toast[],
  tutorial: false,
  theme: 'light' as 'light' | 'dark'
});

let nextToast = 1;
export function toast(kind: Toast['kind'], text: string) {
  const id = nextToast++;
  app.toasts.push({ id, kind, text });
  const ms = kind === 'warn' || kind === 'err' ? 8000 : 4500;
  setTimeout(() => (app.toasts = app.toasts.filter((t) => t.id !== id)), ms);
}

export function setTheme(t: 'light' | 'dark') {
  app.theme = t;
  document.documentElement.dataset.theme = t;
  localStorage.setItem('wishtory.theme', t);
}
export function initTheme() {
  const saved = localStorage.getItem('wishtory.theme') as 'light' | 'dark' | null;
  setTheme(saved ?? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'));
}

export async function reloadAccounts() {
  app.accounts = await api.listAccounts();
  const mine = app.accounts.filter((a) => a.game === app.game);
  if (!mine.some((a) => a.uid === app.uid)) app.uid = mine[0]?.uid ?? null;
}
export function selectGame(g: GameId) {
  app.game = g;
  app.uid = app.accounts.find((a) => a.game === g)?.uid ?? null;
}

async function run(action: () => Promise<FetchSummary>) {
  if (app.busy) return;
  app.busy = true;
  try {
    const r = await action();
    await reloadAccounts();
    app.uid = r.uid;
    app.refreshKey++;
    toast('ok', r.added > 0 ? `Added ${r.added} new pulls for UID ${r.uid}.` : `Already up to date for UID ${r.uid}.`);
    if (r.warning) toast('warn', r.warning);
  } catch (e) {
    toast('err', errMsg(e));
  } finally {
    app.busy = false;
  }
}

/** Read-only: finds the link the game already cached, then fetches. */
export const refreshFromGame = () => {
  toast('info', "Reading the game's cache (read-only)...");
  return run(() => api.refreshFromGame(app.game));
};
export const fetchFromUrl = (url: string) => run(() => api.fetchHistory(app.game, url));
