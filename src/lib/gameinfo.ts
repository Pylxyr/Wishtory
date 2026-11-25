import type { GameId } from './types';

/** Words that differ per game, in one place so every screen says "S-rank" for Zenless and "5★" elsewhere. */
export interface GameInfo {
  id: GameId; key: 'hk4e' | 'hkrpg' | 'nap'; name: string; short: string; abbr: string; howTo: string; historyPage: string;
  r5: string; r4: string; fives: string; currency: string; pullCost: number; themeId: string; liveTitle: string; bannersTitle: string;
}

export const INFO: Record<GameId, GameInfo> = {
  genshin: {
    id: 'genshin', key: 'hk4e', name: 'Genshin Impact', short: 'Genshin', abbr: 'GI',
    howTo: 'Open the Paimon menu, then Wish, then History, and flick through a banner tab.', historyPage: 'Wish → History',
    r5: '5★', r4: '4★', fives: 'five-stars', currency: 'Primogems', pullCost: 160, themeId: 'teyvat',
    liveTitle: 'Wishing now', bannersTitle: 'Your wishes'
  },
  star_rail: {
    id: 'star_rail', key: 'hkrpg', name: 'Honkai: Star Rail', short: 'Star Rail', abbr: 'HSR',
    howTo: 'Open Warp → Records (Warp History) and let the list load.', historyPage: 'Warp → Records',
    r5: '5★', r4: '4★', fives: 'five-stars', currency: 'Stellar Jade', pullCost: 160, themeId: 'astral',
    liveTitle: 'Now boarding', bannersTitle: 'Warp route'
  },
  zenless: {
    id: 'zenless', key: 'nap', name: 'Zenless Zone Zero', short: 'Zenless Zone Zero', abbr: 'ZZZ',
    howTo: 'Open Signal Search → History and let the list load.', historyPage: 'Signal Search → History',
    r5: 'S-rank', r4: 'A-rank', fives: 'S-rank pulls', currency: 'Polychromes', pullCost: 160, themeId: 'eridu',
    liveTitle: 'On air', bannersTitle: 'Channels'
  }
};

export const gameFromKey = (key: string): GameId | null => (Object.values(INFO).find((i) => i.key === key)?.id ?? null);
export const rankLabel = (g: GameId, rank: number) => (rank === 5 ? INFO[g].r5 : rank === 4 ? INFO[g].r4 : g === 'zenless' ? 'B-rank' : '3★');
export const utcLabel = (h: number) => `UTC${h >= 0 ? '+' : ''}${h}`;
export const groupDigits = (n: number) => n.toLocaleString('en-US');
export const plural = (n: number, w: string, many?: string) => `${groupDigits(n)} ${n === 1 ? w : (many ?? w + 's')}`;

export function ago(then: number, now = Date.now()): string {
  const s = Math.round((now - then) / 1000);
  if (s < 45) return 'just now';
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 48) return `${h} h ago`;
  return `${Math.round(h / 24)} d ago`;
}

/** "2026-11-24 18:00:00" is server-local; compare as plain strings against another server-local time. */
export const shortDate = (t: string) => t.slice(0, 10);
export function daysLeft(end: string, nowLocal: string): number {
  const e = Date.parse(end.replace(' ', 'T') + 'Z');
  const n = Date.parse(nowLocal.replace(' ', 'T') + 'Z');
  return Math.max(0, Math.ceil((e - n) / 86_400_000));
}
export const serverNow = (tz: number) => new Date(Date.now() + tz * 3_600_000).toISOString().slice(0, 19).replace('T', ' ');
