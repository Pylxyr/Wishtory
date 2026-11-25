import { api, asError } from './api';
import { INFO, gameFromKey } from './gameinfo';
import { applyPalette } from './themes';
import { loadPrefs, savePrefs, type Prefs } from './prefs';
import type * as T from './types';
import type { GameId } from './types';

export type Page = 'overview' | 'history' | 'import' | 'accounts' | 'settings' | 'about';
export interface Toast { id: number; text: string; kind: 'ok' | 'err' | 'info' }
export interface RefreshState { game: GameId; phase: 'running' | 'done' | 'failed'; message: string; summary?: T.FetchSummary; error?: T.ApiError }

class App {
  game = $state<GameId>('genshin');
  page = $state<Page>('overview');
  prefs = $state<Prefs>(loadPrefs());
  ready = $state(false);
  startupError = $state<T.ApiError | null>(null);

  accounts = $state<T.Account[]>([]);
  uids = $state<Partial<Record<GameId, string>>>({});
  meta = $state<T.GameMeta[]>([]);
  catalog = $state<T.GameCatalog | null>(null);
  banners = $state<T.BannerList | null>(null);
  pity = $state<T.BannerPity[]>([]);
  pityLoading = $state(false);
  selectedBanner = $state<string | null>(null);
  catalogStatus = $state<T.CatalogStatus | null>(null);

  refresh = $state<RefreshState | null>(null);
  toasts = $state<Toast[]>([]);
  prefersDark = $state(true);
  /** Bumped on every page or game change; keys the page transition so each game can animate it its own way. */
  navTick = $state(0);
  private seq = 0;
  private pityToken = 0;

  info = $derived(INFO[this.game]);
  dark = $derived(this.prefs.mode === 'system' ? this.prefersDark : this.prefs.mode === 'dark');
  gameAccounts = $derived(this.accounts.filter((a) => a.game === this.game));
  account = $derived(this.gameAccounts.find((a) => a.uid === this.uids[this.game]) ?? this.gameAccounts[0] ?? null);
  index = $derived.by(() => {
    const byName = new Map<string, T.CatalogItem>();
    const byId = new Map<string, T.CatalogItem>();
    for (const i of this.catalog?.items ?? []) {
      byId.set(`${i.game}|${i.id}`, i);
      const k = `${i.game}|${i.name.toLowerCase()}`;
      if (!byName.has(k)) byName.set(k, i);
      for (const a of i.aliases) if (!byName.has(`${i.game}|${a.toLowerCase()}`)) byName.set(`${i.game}|${a.toLowerCase()}`, i);
    }
    return { byName, byId };
  });

  constructor() {
    if (typeof window !== 'undefined') {
      const q = window.matchMedia('(prefers-color-scheme: dark)');
      this.prefersDark = q.matches;
      q.addEventListener('change', (e) => (this.prefersDark = e.matches));
      const g = localStorage.getItem('wishtory.game');
      const found = g ? (Object.values(INFO).find((i) => i.id === g)?.id ?? gameFromKey(g)) : null;
      if (found) this.game = found;
      const p = localStorage.getItem('wishtory.page') as Page | null;
      if (p && ['overview', 'history', 'import', 'accounts', 'settings', 'about'].includes(p)) this.page = p;
    }
  }

  /** Writes theme variables and the data-* hooks the stylesheet's per-game skins and animations key on. */
  paint() {
    applyPalette(this.prefs.themeId, this.dark, this.game);
    const r = document.documentElement;
    r.dataset.game = this.game;
    r.dataset.motion = this.prefs.motion ? 'on' : 'off';
    r.dataset.density = this.prefs.comfortable ? 'comfortable' : 'compact';
  }

  setPrefs(patch: Partial<Prefs>) {
    this.prefs = { ...this.prefs, ...patch };
    savePrefs(this.prefs);
  }

  toast(text: string, kind: Toast['kind'] = 'info') {
    const id = ++this.seq;
    this.toasts = [...this.toasts, { id, text, kind }];
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), kind === 'err' ? 7000 : 4000);
  }

  go(page: Page) {
    this.page = page;
    this.navTick++;
    localStorage.setItem('wishtory.page', page);
  }
  openGame(game: GameId) {
    const page = this.page === 'overview' || this.page === 'history' ? this.page : 'overview';
    this.game = game;
    this.go(page);
    localStorage.setItem('wishtory.game', game);
    this.selectedBanner = null;
    void this.loadPity();
  }
  pickAccount(uid: string) {
    this.uids = { ...this.uids, [this.game]: uid };
    this.selectedBanner = null;
    void this.loadPity();
  }

  async start() {
    try {
      this.startupError = await api.startupError();
      if (this.startupError) return;
      const [meta, accounts, catalog, banners, status] = await Promise.all([
        api.gameMeta(), api.listAccounts(), api.getCatalog(), api.getBanners(), api.getCatalogStatus()
      ]);
      this.meta = meta; this.accounts = accounts; this.catalog = catalog; this.banners = banners; this.catalogStatus = status;
      await this.loadPity();
    } catch (e) {
      this.startupError = asError(e);
    } finally {
      this.ready = true;
    }
  }

  async reloadAccounts() {
    this.accounts = await api.listAccounts();
    await this.loadPity();
  }

  async loadPity() {
    const acc = this.account;
    const token = ++this.pityToken;
    if (!acc) { this.pity = []; return; }
    this.pityLoading = true;
    try {
      const pity = await api.getPity(acc.game, acc.uid);
      if (token === this.pityToken) this.pity = pity;
    } catch (e) {
      this.toast(asError(e).message, 'err');
    } finally {
      if (token === this.pityToken) this.pityLoading = false;
    }
  }

  async refreshGame(full = false, url?: string) {
    const game = this.game;
    this.refresh = { game, phase: 'running', message: 'Looking for the history page you opened in the game…' };
    const onEvent = (e: T.RefreshEvent) => {
      if (e.kind === 'progress') this.refresh = { game, phase: 'running', message: e.message };
      else if (e.kind === 'finished') {
        this.refresh = { game, phase: 'done', message: '', summary: e.summary ?? undefined };
        if (e.summary) this.setPrefs({ lastRefresh: { ...this.prefs.lastRefresh, [`${e.summary.game}|${e.summary.uid}`]: Date.now() } });
        void this.reloadAccounts();
      } else this.refresh = { game, phase: 'failed', message: e.message, error: e.error ?? undefined };
    };
    try {
      if (url) await api.fetchHistory(game, url, full, onEvent);
      else await api.refreshFromGame(game, full, onEvent);
    } catch (e) {
      this.refresh = { game, phase: 'failed', message: asError(e).message, error: asError(e) };
    }
  }

  nameOf(a: T.Account) { return this.prefs.aliases[`${a.game}|${a.uid}`] || a.nickname || a.uid; }
  itemByName(name: string) { return this.index.byName.get(`${INFO[this.game].key}|${name.toLowerCase()}`); }
}

export const app = new App();
