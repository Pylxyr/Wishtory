import { listen } from '@tauri-apps/api/event';
import { api, errMsg } from './api';
import type { Account, FetchSummary, GameCatalog, GameId, GameInstall } from './types';

export type Page = 'dashboard' | 'history' | 'accounts' | 'importexport' | 'help' | 'settings' | 'logging';
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
  theme: 'dark' as 'light' | 'dark',
  status: '', // latest refresh progress line, shown while busy
  steps: [] as string[], // one line per refresh step, kept after the run so the outcome stays visible
  result: null as { kind: 'ok' | 'warn' | 'err'; text: string } | null,
  loggingEnabled: false // off by default; persisted in localStorage next to the theme
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
  setTheme(saved ?? 'dark'); // dark-first; the sidebar toggle switches to light and is remembered
}

/** Restores the saved logging switch and tells the backend. Off (the default) leaves the backend silent. */
export async function initLogging() {
  if (localStorage.getItem('wishtory.logging') !== '1') return;
  try {
    await api.setLoggingEnabled(true);
    app.loggingEnabled = true;
  } catch { /* backend unavailable: stay off */ }
}
export async function setLogging(on: boolean) {
  try {
    await api.setLoggingEnabled(on);
    app.loggingEnabled = on;
    if (on) localStorage.setItem('wishtory.logging', '1'); else localStorage.removeItem('wishtory.logging');
    if (!on && app.page === 'logging') app.page = 'settings';
  } catch (e) {
    toast('err', errMsg(e));
  }
}

/** Status lines from the backend while a refresh runs ("Downloading banner 2 of 5…"). */
export function initProgress() {
  void listen<string>('refresh-progress', (e) => {
    if (!app.busy) return;
    app.status = e.payload;
    logStep(e.payload);
  }).catch(() => {});
}

/** One line per banner: a message that starts with the same banner name (the text before the first colon) replaces the last line. */
function logStep(text: string) {
  const colon = text.indexOf(':');
  const last = app.steps.at(-1);
  if (colon !== -1 && last !== undefined && last.startsWith(`${text.slice(0, colon)}:`)) app.steps[app.steps.length - 1] = text;
  else app.steps.push(text);
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
  app.status = 'Starting…';
  app.steps = ['Starting…'];
  app.result = null;
  try {
    const r = await action();
    await reloadAccounts();
    app.uid = r.uid;
    app.refreshKey++;
    toast('ok', r.added > 0 ? `Added ${r.added} new pulls for UID ${r.uid}.` : `Already up to date for UID ${r.uid}.`);
    if (r.warning) toast('warn', r.warning);
    app.result = {
      kind: r.warning ? 'warn' : 'ok',
      text: `Downloaded ${r.fetched.toLocaleString()} pulls for UID ${r.uid}: ${r.added.toLocaleString()} new, ${(r.fetched - r.added).toLocaleString()} already saved.${r.warning ? ` ${r.warning}` : ''}`
    };
  } catch (e) {
    toast('err', errMsg(e));
    app.result = { kind: 'err', text: errMsg(e) };
  } finally {
    app.busy = false;
    app.status = '';
  }
}

/** Read-only: finds the link the game already cached, then fetches. */
export const refreshFromGame = () => {
  return run(() => api.refreshFromGame(app.game));
};
export const fetchFromUrl = (url: string) => run(() => api.fetchHistory(app.game, url));
