import type { BannerEntry, BannerPity, CatalogItem, GameId, BannerList } from './types';
import { daysLeft, serverNow } from './gameinfo';

export interface LiveCard { banner: BannerEntry; title: string; items: CatalogItem[]; key: string; days: number; hours: number }

const order = (pool: string) => (pool === 'character' ? 0 : pool === 'weapon' ? 1 : 2);

/** The pity-engine key a banner entry belongs to (rerun periods are told apart by their id). */
export function pityKey(b: BannerEntry): string {
  if (b.id.endsWith('-rescreening')) return 'rescreening';
  if (b.id.endsWith('-reverberation')) return 'reverberation';
  return b.pool;
}

/** Banners running now on the account's server clock, as cards: one per featured character, one per weapon banner. */
export function liveCards(list: BannerList | null, game: GameId, tz: number, find: (g: string, id: string) => CatalogItem | undefined, gkey: string): LiveCard[] {
  if (!list) return [];
  const now = serverNow(tz);
  const live = list.banners.filter((b) => b.game === game && b.start <= now && now <= b.end);
  live.sort((a, b) => order(a.pool) - order(b.pool) || (a.end < b.end ? -1 : 1));
  const out: LiveCard[] = [];
  for (const b of live) {
    const items = b.featured_5.map((id) => find(gkey, id)).filter((x): x is CatalogItem => !!x);
    const names = b.name.split(' / ');
    const msLeft = Date.parse(b.end.replace(' ', 'T') + 'Z') - Date.parse(now.replace(' ', 'T') + 'Z');
    const base = { banner: b, days: daysLeft(b.end, now), hours: Math.max(0, Math.floor(msLeft / 3_600_000)) };
    if (b.pool === 'character' && items.length > 1) {
      items.forEach((it, i) => out.push({ ...base, title: names.length === items.length ? names[i] : it.name, items: [it], key: `${b.id}-${it.id}` }));
    } else out.push({ ...base, title: names[0] || b.name, items, key: b.id });
  }
  return out;
}

export const pityOf = (pity: BannerPity[], b: BannerEntry) => pity.find((p) => p.banner === pityKey(b));
