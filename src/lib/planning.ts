import type { BannerEntry, BannerPity, BannerSet, GameCatalog, GameId } from './types';

/**
 * Worst case under the rules Wishtory tracks: how many more pulls guarantee a five-star, and, on
 * banners that track the 50/50 or 75/25, a guaranteed featured one. Only banners with a soft-pity
 * ladder qualify (beginner and collab banners work differently). Estimates, not promises.
 */
export function pullsToGuarantee(p: BannerPity): { fiveStar: number; featured: number | null } | null {
  if (p.soft_pity === null || p.hard_pity <= 0) return null;
  const toFive = Math.max(p.hard_pity - p.pity, 0);
  if (p.guarantee_odds === null) return { fiveStar: toFive, featured: null };
  // After a lost 50/50 the next five-star is the featured one; otherwise a full second ladder may be needed.
  return { fiveStar: toFive, featured: p.guaranteed === true ? toFive : toFive + p.hard_pity };
}

/** "YYYY-MM-DD HH:MM:SS" (a server wall-clock time, no zone) as milliseconds on a plain UTC axis. */
export function clockMs(s: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})$/.exec(s);
  if (!m) return null;
  return Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]);
}

/** The server's wall clock right now, on the same axis as clockMs, for an account at the given UTC offset in hours. */
export const serverClockMs = (tzHours: number, now = Date.now()): number => now + tzHours * 3_600_000;

export interface LiveBanner { id: string; name: string; pool: string; featured: string[]; msLeft: number; }

/** Event banners running right now. Start and end are inclusive server-local times, like each pull's time. */
export function liveBanners(set: BannerSet | null, game: GameId, tzHours: number, catalog: GameCatalog | null, now = Date.now()): LiveBanner[] {
  if (!set) return [];
  const clock = serverClockMs(tzHours, now);
  const names = new Map<string, string>();
  for (const item of catalog?.items ?? []) if (item.game === game) names.set(item.id, item.name);
  const out: LiveBanner[] = [];
  for (const b of set.banners as BannerEntry[]) {
    if (b.game !== game) continue;
    const start = clockMs(b.start);
    const end = clockMs(b.end);
    if (start === null || end === null || clock < start || clock > end + 999) continue;
    out.push({ id: b.id, name: b.name, pool: b.pool, featured: b.featured_5.map((id) => names.get(id) ?? '').filter(Boolean), msLeft: end + 1000 - clock });
  }
  return out.sort((a, b) => a.msLeft - b.msLeft);
}

/** "6 d 4 h left", "5 h 12 min left", "9 min left". */
export function timeLeft(ms: number): string {
  const min = Math.max(1, Math.floor(ms / 60_000));
  const d = Math.floor(min / 1440);
  const h = Math.floor((min % 1440) / 60);
  if (d >= 1) return `${d} d ${h} h left`;
  if (h >= 1) return `${h} h ${min % 60} min left`;
  return `${min} min left`;
}
