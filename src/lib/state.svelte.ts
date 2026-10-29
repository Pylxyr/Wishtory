import { listen } from '@tauri-apps/api/event';
import { api, errMsg } from './api';
import type { AboutInfo, Account, AppUpdate, BannerSet, BannerUpdate, FetchSummary, GameCatalog, GameId, GameInstall, GameMeta } from './types';

export type Page = 'dashboard' | 'history' | 'accounts' | 'importexport' | 'help' | 'settings' | 'about' | 'logging';
interface Toast { id: number; kind: 'ok' | 'err' | 'info' | 'warn'; text: string; }

export const app = $state({
  page: 'dashboard' as Page,
  game: 'hk4e' as GameId,
  uid: null as string | null,
  accounts: [] as Account[],
  installs: [] as GameInstall[],
  catalog: null as GameCatalog | null,
  meta: [] as GameMeta[], // static per-game UI naming from the backend
  banners: null as BannerSet | null,
  bannerUpdate: null as BannerUpdate | null, // the last installed banner update, kept across restarts
  about: null as AboutInfo | null,
  update: null as AppUpdate | null, // the last app update check this session
  updateChecking: false,
  busy: false,
  refreshKey: 0, // bump to make pages reload their data
  toasts: [] as Toast[],
  tutorial: false,
  theme: 'dark' as 'light' | 'dark',
  status: '', // latest refresh progress line, shown while busy
  steps: [] as string[], // one line per refresh step, kept after the run so the outcome stays visible
  result: null as { kind: 'ok' | 'warn' | 'err'; text: string } | null,
  runView: null as { game: GameId; page: Page } | null, // where the last refresh was started; its progress panel is only shown there
  loggingEnabled: false // off by default; persisted in localStorage next to the theme
});

let nextToast = 1;
export function toast(kind: Toast['kind'], text: string) {
  const id = nextToast++;
  app.toasts.push({ id, kind, text });
  const ms = kind === 'warn' || kind === 'err' ? 8000 : 4500;
  setTimeout(() => (app.toasts = app.toasts.filter((t) => t.id !== id)), ms);
}

const UPDATE_CHECK_KEY = 'wishtory.updateCheckedAt';
const DAY_MS = 24 * 60 * 60 * 1000;

/** Opens one of the links Wishtory shows in the default browser. The backend refuses any other link. */
export async function openLink(url: string) {
  try {
    await api.openExternal(url);
  } catch (e) {
    toast('warn', errMsg(e));
  }
}

/** Looks for a newer Wishtory release. Automatic checks run at most once a day; a manual one always runs. */
export async function checkAppUpdate(manual = false) {
  if (app.updateChecking) return;
  if (!manual && Date.now() - Number(localStorage.getItem(UPDATE_CHECK_KEY) ?? 0) < DAY_MS) return;
  app.updateChecking = true;
  try {
    const update = await api.checkAppUpdate();
    localStorage.setItem(UPDATE_CHECK_KEY, String(Date.now()));
    app.update = update;
    if (update.available) toast('info', `Wishtory ${update.latest} is available. Open Settings or About to view the release.`);
    else if (manual) toast('ok', `Wishtory ${update.current} is up to date.`);
  } catch (e) {
    if (manual) toast('warn', `Could not check for app updates: ${errMsg(e)}`);
  } finally {
    app.updateChecking = false;
  }
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
  const game = app.game;
  app.busy = true;
  app.status = 'Starting…';
  app.steps = ['Starting…'];
  app.result = null;
  app.runView = { game, page: app.page };
  try {
    const r = await action();
    await reloadAccounts();
    // Only follow the new account when the user is still on that game; they may have switched meanwhile.
    if (app.game === game) app.uid = r.uid;
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
export const refreshFromGame = () => run(() => api.refreshFromGame(app.game));
/** Same, but downloads everything the server still keeps instead of stopping at what is already saved. */
export const resyncFromGame = () => run(() => api.refreshFromGame(app.game, true));
export const fetchFromUrl = (url: string) => run(() => api.fetchHistory(app.game, url));
