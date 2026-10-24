export type GameId = 'hk4e' | 'hkrpg' | 'nap';

export interface GameInstall { game: GameId; found: boolean; log_path: string | null; data_dir: string | null; }
export interface Account { game: GameId; uid: string; nickname: string; timezone: number; pull_count: number; }
export interface Pull {
  id: string; game: string; uid: string; gacha_type: string; gacha_id: string; item_id: string;
  name: string; item_type: string; rank_type: number; time: string; lang: string; count: number;
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
export interface FiveStar { name: string; pity: number; time: string; off_banner: boolean | null; outcome: 'win' | 'loss' | 'guaranteed' | null; }
export interface FourStar { name: string; pity: number; time: string; }
export interface BannerPity {
  banner: string; label: string; pity: number; hard_pity: number; soft_pity: number | null;
  guaranteed: boolean | null; total: number; five_stars: number; four_stars: number;
  avg_five_pity: number | null; avg_four_pity: number | null;
  recent_fives: FiveStar[]; recent_fours: FourStar[];
}
export interface FetchSummary { game: string; uid: string; fetched: number; added: number; warning: string | null; }
export interface ImportSummary { format: string; accounts: number; added: number; skipped: number; }

export const GAMES: Record<GameId, { name: string; short: string; howTo: string }> = {
  hk4e: { name: 'Genshin Impact', short: 'Genshin', howTo: 'Open the Paimon menu, then Wish, then History, and flick through a banner tab.' },
  hkrpg: { name: 'Honkai: Star Rail', short: 'Star Rail', howTo: 'Open Warp → Records (Warp History) and let the list load.' },
  nap: { name: 'Zenless Zone Zero', short: 'Zenless Zone Zero', howTo: 'Open Signal Search → History and let the list load.' }
};

export const BANNERS: Record<GameId, { key: string; label: string }[]> = {
  hk4e: [
    { key: 'character', label: 'Character Event' }, { key: 'weapon', label: 'Weapon Event' },
    { key: 'standard', label: 'Standard' }, { key: 'chronicled', label: 'Chronicled' }, { key: 'beginner', label: "Beginners'" }
  ],
  hkrpg: [
    { key: 'character', label: 'Character Warp' }, { key: 'weapon', label: 'Light Cone Warp' },
    { key: 'standard', label: 'Stellar Warp' }, { key: 'beginner', label: 'Departure Warp' }
  ],
  nap: [
    { key: 'character', label: 'Exclusive Channel' }, { key: 'weapon', label: 'W-Engine Channel' },
    { key: 'standard', label: 'Standard Channel' }, { key: 'bangboo', label: 'Bangboo Channel' }
  ]
};

export const GACHA_LABELS: Record<GameId, Record<string, string>> = {
  hk4e: { '100': "Beginners'", '200': 'Standard', '301': 'Character', '400': 'Character', '302': 'Weapon', '500': 'Chronicled' },
  hkrpg: { '1': 'Stellar', '2': 'Departure', '11': 'Character', '12': 'Light Cone', '21': 'Character Event', '22': 'Light Cone Event' },
  nap: { '1': 'Standard', '2': 'Exclusive', '3': 'W-Engine', '5': 'Bangboo' }
};

export interface LogEntry { seq: number; time: string; level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR'; source: string; message: string; }
