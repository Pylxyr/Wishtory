import { listen } from '@tauri-apps/api/event';
import { api, errMsg } from './api';
import { GAMES, type AboutInfo, type Account, type AppUpdate, type BannerSet, type BannerUpdate, type FetchSummary, type GameCatalog, type GameId, type GameInstall, type GameLocations, type GameMeta } from './types';

export type Page = 'dashboard' | 'history' | 'accounts' | 'importexport' | 'help' | 'settings' | 'about' | 'logging';
export type ThemePref = 'system' | 'dark' | 'light';
/** ttl is how long a toast stays in ms; 0 keeps it until the person dismisses it (errors). */
export interface Toast { id: number; kind: 'ok' | 'err' | 'info' | 'warn'; text: string; ttl: number; }
/** The outcome of the last refresh. It stays visible in the status bar until dismissed or replaced. */
export interface RunResult { kind: 'ok' | 'warn' | 'err'; text: string; game: GameId; at: number; }

export const app = $state({
  page: 'dashboard' as Page,
  game: 'hk4e' as GameId,
  uid: null as string | null,
  accounts: [] as Account[],
  installs: [] as GameInstall[],
  locations: null as GameLocations | null, // where games are searched for; null until loaded
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
  themePref: 'system' as ThemePref,
  theme: 'dark' as 'light' | 'dark', // what is shown: the preference, or the system setting when the preference is 'system'
  status: '', // latest refresh progress line, shown while busy
  steps: [] as string[], // one line per refresh step, kept after the run so the outcome stays visible
  result: null as RunResult | null,
  runGame: null as GameId | null, // the game a running refresh belongs to
  lastRefresh: {} as Record<string, number>, // `${game}:${uid}` → when Wishtory last downloaded that account (this PC only)
  aliases: {} as Record<string, string>, // `${game}:${uid}` → a label the person gave the account (this PC only)
  lastUid: {} as Partial<Record<GameId, string>>, // the account last used per game, so switching games comes back to it
  helpTopic: null as string | null, // a Help fold to open when the Help page shows
  loggingEnabled: false // off by default; persisted in localStorage next to the theme
});

let nextToast = 1;
export function toast(kind: Toast['kind'], text: string) {
  if (app.toasts.some((t) => t.kind === kind && t.text === text)) return; // do not stack identical messages
  // Errors stay until dismissed; the rest time out, and the timer pauses while the toast is hovered or focused.
  const ttl = kind === 'err' ? 0 : kind === 'warn' ? 12000 : 6000;
  app.toasts.push({ id: nextToast++, kind, text, ttl });
}
export function dismissToast(id: number) {
  app.toasts = app.toasts.filter((t) => t.id !== id);
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

const lightQuery = () => matchMedia('(prefers-color-scheme: light)');
function applyTheme() {
  app.theme = app.themePref === 'system' ? (lightQuery().matches ? 'light' : 'dark') : app.themePref;
  document.documentElement.dataset.theme = app.theme;
}
export function setThemePref(pref: ThemePref) {
  app.themePref = pref;
  localStorage.setItem('wishtory.theme', pref);
  applyTheme();
}
/** Runs before the first paint. Follows the system theme until the person picks one. */
export function initTheme() {
  const saved = localStorage.getItem('wishtory.theme');
  app.themePref = saved === 'light' || saved === 'dark' ? saved : 'system';
  applyTheme();
  lightQuery().addEventListener('change', () => { if (app.themePref === 'system') applyTheme(); });
}

// ---- What the person was looking at, remembered between launches (this PC only) ----
const VIEW_KEY = 'wishtory.view';
const REFRESH_KEY = 'wishtory.lastRefresh';
const ALIAS_KEY = 'wishtory.aliases';
const RESTORABLE: Page[] = ['dashboard', 'history', 'accounts', 'importexport', 'help', 'settings', 'about'];
function readJson<T>(key: string, fallback: T): T {
  try { return JSON.parse(localStorage.getItem(key) ?? 'null') ?? fallback; } catch { return fallback; }
}
export function restoreView() {
  const v = readJson<{ page?: Page; game?: GameId; uid?: string } | null>(VIEW_KEY, null);
  if (v?.game && v.game in GAMES) {
    app.game = v.game;
    app.uid = typeof v.uid === 'string' ? v.uid : null; // reloadAccounts drops it if the account is gone
    app.lastUid[v.game] = app.uid ?? undefined;
  }
  if (v?.page && RESTORABLE.includes(v.page)) app.page = v.page;
  app.lastRefresh = readJson(REFRESH_KEY, {});
  app.aliases = readJson(ALIAS_KEY, {});
}
export function saveView() {
  try { localStorage.setItem(VIEW_KEY, JSON.stringify({ page: app.page, game: app.game, uid: app.uid })); } catch { /* storage full or blocked: nothing to remember */ }
}
const key = (game: string, uid: string) => `${game}:${uid}`;
export const lastRefreshOf = (game: string, uid: string): number | null => app.lastRefresh[key(game, uid)] ?? null;
export const accountLabel = (a: Account): string => app.aliases[key(a.game, a.uid)] || a.nickname;
export function setAlias(game: string, uid: string, label: string) {
  const next = { ...app.aliases };
  if (label.trim()) next[key(game, uid)] = label.trim(); else delete next[key(game, uid)];
  app.aliases = next;
  localStorage.setItem(ALIAS_KEY, JSON.stringify(next));
}
/** Drops what is remembered about accounts that were removed. */
export function forgetAccounts(only?: { game: string; uid: string }) {
  const keep = (map: Record<string, unknown>) =>
    Object.fromEntries(Object.entries(map).filter(([k]) => (only ? k !== key(only.game, only.uid) : false)));
  app.lastRefresh = keep(app.lastRefresh) as Record<string, number>;
  app.aliases = keep(app.aliases) as Record<string, string>;
  localStorage.setItem(REFRESH_KEY, JSON.stringify(app.lastRefresh));
  localStorage.setItem(ALIAS_KEY, JSON.stringify(app.aliases));
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
  if (app.uid) app.lastUid[app.game] = app.uid;
  app.game = g;
  const mine = app.accounts.filter((a) => a.game === g);
  app.uid = mine.find((a) => a.uid === app.lastUid[g])?.uid ?? mine[0]?.uid ?? null;
}

/** Saves the extra game folders (Linux), then looks for the games again. */
export async function saveGameLocations(paths: string[]) {
  app.locations = await api.setGameLocations(paths);
  app.installs = await api.discoverGames().catch(() => []);
}

/** Jumps to the Help page, optionally with one troubleshooting entry opened. */
export function openHelp(topic: string | null = null) {
  app.helpTopic = topic;
  app.page = 'help';
}

let lastAction: (() => Promise<FetchSummary>) | null = null;
async function run(action: () => Promise<FetchSummary>) {
  if (app.busy) return;
  lastAction = action;
  const game = app.game;
  app.busy = true;
  app.runGame = game;
  app.status = 'Starting…';
  app.steps = ['Starting…'];
  app.result = null;
  try {
    const r = await action();
    await reloadAccounts();
    // Only follow the new account when the user is still on that game; they may have switched meanwhile.
    if (app.game === game) app.uid = r.uid;
    app.refreshKey++;
    app.lastRefresh = { ...app.lastRefresh, [key(game, r.uid)]: Date.now() };
    localStorage.setItem(REFRESH_KEY, JSON.stringify(app.lastRefresh));
    app.result = {
      kind: r.warning ? 'warn' : 'ok', game, at: Date.now(),
      text: `Downloaded ${r.fetched.toLocaleString()} pulls for UID ${r.uid}: ${r.added.toLocaleString()} new, ${(r.fetched - r.added).toLocaleString()} already saved.${r.warning ? ` ${r.warning}` : ''}`
    };
  } catch (e) {
    app.result = { kind: 'err', game, at: Date.now(), text: errMsg(e) };
  } finally {
    app.busy = false;
    app.runGame = null;
    app.status = '';
  }
}
export function dismissResult() {
  app.result = null;
  app.steps = [];
}
/** Runs the last refresh again (the "Try again" button). */
export const retryLast = () => (lastAction ? run(lastAction) : Promise.resolve());

/** Read-only: finds the link the game already cached, then fetches. */
export const refreshFromGame = () => run(() => api.refreshFromGame(app.game));
/** Same, but downloads everything the server still keeps instead of stopping at what is already saved. */
export const resyncFromGame = () => run(() => api.refreshFromGame(app.game, true));
export const fetchFromUrl = (url: string) => run(() => api.fetchHistory(app.game, url));
