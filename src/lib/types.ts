export type GameId = 'hk4e' | 'hkrpg' | 'nap';

export interface GameInstall { game: GameId; found: boolean; log_path: string | null; data_dir: string | null; }
export interface Account { game: GameId; uid: string; nickname: string; timezone: number; pull_count: number; }
export interface Pull {
  id: string; game: string; uid: string; gacha_type: string; gacha_id: string; item_id: string;
  name: string; item_type: string; rank_type: number; time: string; lang: string; count: number;
  banner_name: string | null; pity: number | null;
  rank: number; // 3 / 4 / 5 for every game; the backend maps Zenless B / A / S
  pool: string; // short pool name for the table, e.g. "Character"
}

export interface CatalogItem {
  game: string;
  id: string;
  name: string;
  aliases: string[];
  rarity: number;
  kind: string;
  traits: Record<string, string>;
  paimon_key: string | null;
}
export interface GameCatalog {
  schema_version: number;
  catalog_version: number;
  generated_at: string;
  items: CatalogItem[];
}
export interface BannerEntry {
  id: string; game: GameId; pool: string; name: string; gacha_ids: string[];
  start: string; end: string; featured_5: string[]; featured_4: string[];
}
export interface BannerSet { schema_version: number; banners_version: number; generated_at: string; banners: BannerEntry[]; }
export interface BannerUpdate {
  status: 'updated' | 'up-to-date' | 'already-checking'; banners_version: number; installed_at: number;
  added: string[]; changed: string[]; removed: string[];
}
export interface Credit { name: string; role: string; url: string; license: string; }
export interface CreditGroup { title: string; note: string; credits: Credit[]; }
export interface AboutInfo {
  name: string; version: string; description: string; copyright: string; license: string; disclaimers: string[];
  repository: string; releases: string; issues: string; groups: CreditGroup[];
}
export interface AppUpdate { current: string; latest: string; available: boolean; url: string; prerelease: boolean; published_at: string; }
export interface FiveStar { name: string; pity: number; time: string; off_banner: boolean | null; outcome: 'win' | 'loss' | 'guaranteed' | null; }
export interface FourStar { name: string; pity: number; time: string; }
export interface BannerPity {
  banner: string; label: string; pity: number; hard_pity: number; soft_pity: number | null;
  guaranteed: boolean | null; guarantee_odds: number | null; total: number; five_stars: number; four_stars: number;
  avg_five_pity: number | null; avg_four_pity: number | null;
  recent_fives: FiveStar[]; recent_fours: FourStar[];
}
export interface FetchSummary { game: string; uid: string; fetched: number; added: number; warning: string | null; }
export interface ImportSummary { format: string; accounts: number; added: number; skipped: number; }

/** Words that differ per game. One place, so every page says "S-rank" for Zenless and "5★" elsewhere. */
export interface GameInfo {
  name: string; short: string; abbr: string; howTo: string;
  historyPage: string; // where the in-game history page lives, for recovery steps
  r5: string; r4: string; // short labels: "5★" / "S-rank"
  five: string; fives: string; // "five-star" / "five-stars", "S-rank" / "S-rank pulls"
  currency: string; pullCost: number; // what one pull is worth in the premium currency, for comparisons only
}
export const GAMES: Record<GameId, GameInfo> = {
  hk4e: { name: 'Genshin Impact', short: 'Genshin', abbr: 'GI', howTo: 'Open the Paimon menu, then Wish, then History, and flick through a banner tab.', historyPage: 'Wish → History', r5: '5★', r4: '4★', five: 'five-star', fives: 'five-stars', currency: 'Primogems', pullCost: 160 },
  hkrpg: { name: 'Honkai: Star Rail', short: 'Star Rail', abbr: 'HSR', howTo: 'Open Warp → Records (Warp History) and let the list load.', historyPage: 'Warp → Records', r5: '5★', r4: '4★', five: 'five-star', fives: 'five-stars', currency: 'Stellar Jade', pullCost: 160 },
  nap: { name: 'Zenless Zone Zero', short: 'Zenless Zone Zero', abbr: 'ZZZ', howTo: 'Open Signal Search → History and let the list load.', historyPage: 'Signal Search → History', r5: 'S-rank', r4: 'A-rank', five: 'S-rank', fives: 'S-rank pulls', currency: 'Polychromes', pullCost: 160 }
};

/** "5★" or "S-rank": the rarity of a pull as the game words it. */
export function rankLabel(game: GameId, rank: number): string {
  const g = GAMES[game];
  return rank === 5 ? g.r5 : rank === 4 ? g.r4 : game === 'nap' ? 'B-rank' : '3★';
}

/** Static per-game UI naming owned by the backend (history-page banner filter). */
export interface GameMeta { game: GameId; banners: { key: string; label: string }[]; }

export interface LogEntry { seq: number; time: string; level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR'; source: string; message: string; }
