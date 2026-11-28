// UI preferences kept in the webview's own storage (they are not wish data and never leave this PC).
export type Mode = 'system' | 'dark' | 'light';
export type PullView = 'icons_names' | 'names' | 'icons';
/** Which pulls the History table lists: only the top rank, the top two ranks, or everything. */
export type HistoryRank = 'five' | 'four' | 'all';

export interface Prefs {
  mode: Mode; themeId: string; comfortable: boolean; historyRank: HistoryRank; groupByDay: boolean; pullView: PullView; newestFirst: boolean;
  motion: boolean; aliases: Record<string, string>; lastRefresh: Record<string, number>;
}
export const DEFAULT_PREFS: Prefs = {
  mode: 'system', themeId: 'game', comfortable: false, historyRank: 'four', groupByDay: false, pullView: 'icons_names', newestFirst: true,
  motion: true, aliases: {}, lastRefresh: {}
};

const KEY = 'wishtory.prefs.v1';
export function loadPrefs(): Prefs {
  try {
    return { ...DEFAULT_PREFS, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}
export function savePrefs(p: Prefs) {
  try {
    localStorage.setItem(KEY, JSON.stringify(p));
  } catch {
    /* storage can be unavailable; the choices then last until the window closes */
  }
}
