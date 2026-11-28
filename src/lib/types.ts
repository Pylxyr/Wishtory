// Mirrors crates/wishtory-api/src/api/types.rs as it serialises: snake_case fields, snake_case enums.

export type GameId = 'genshin' | 'star_rail' | 'zenless';
export const GAMES: GameId[] = ['genshin', 'star_rail', 'zenless'];
export type PullOutcome = 'win' | 'loss' | 'guaranteed';

export interface GameInstall { game: GameId; found: boolean; log_path: string | null; data_dir: string | null }
export interface DetectedPrefix { path: string; games: GameId[] }
export interface GameLocations { supported: boolean; locations: string[]; detected: DetectedPrefix[] }
export interface Account { game: GameId; uid: string; nickname: string; timezone: number; pull_count: number }

export interface HistoryPull {
  id: string; gacha_type: string; gacha_id: string; item_id: string; name: string; item_type: string;
  time: string; rank: number; pool: string; banner_name: string | null; pity: number | null;
}
export interface FiveStar { name: string; pity: number; time: string; off_banner: boolean | null; outcome: PullOutcome | null }
export interface FourStar { name: string; pity: number; time: string; is_character: boolean }
export interface BannerPity {
  banner: string; label: string; pity: number; hard_pity: number; soft_pity: number | null;
  guaranteed: boolean | null; guarantee_odds: number | null; total: number; five_stars: number; four_stars: number;
  avg_five_pity: number | null; avg_four_pity: number | null; recent_fives: FiveStar[]; recent_fours: FourStar[];
}
export interface BannerFilter { key: string; label: string }
export interface GameMeta { game: GameId; banners: BannerFilter[] }

export interface CatalogItem { game: string; id: string; name: string; aliases: string[]; rarity: number; kind: string; paimon_key: string | null }
export interface GameCatalog { catalog_version: number; generated_at: string; items: CatalogItem[] }
export interface BannerEntry {
  id: string; game: GameId; pool: string; name: string; gacha_ids: string[]; start: string; end: string;
  featured_5: string[]; featured_4: string[];
}
export interface BannerList { banners_version: number; generated_at: string; banners: BannerEntry[] }

export type CheckStatus = 'up_to_date' | 'already_checking' | 'not_ready' | 'applied' | 'needs_review';
export interface CatalogCheck { status: CheckStatus; message: string | null }
export type ChangeSeverity = 'major' | 'minor' | 'info';
export interface ChangeEntry { id: string; scope: 'items' | 'banners' | 'rules'; game: GameId; ids: string[]; summary: string; severity: ChangeSeverity; note: string | null }
export interface WaitingRelease { version: number; generated_at: string; major: number; minor: number; info: number; dismissed: boolean; affected_pulls: number; affected_accounts: number }
export interface CatalogStatus {
  version: number; components: { catalog: number; banners: number; rules: number }; generated_at: string;
  /** Unix seconds (not milliseconds) of the last check; null when never checked. */
  last_check: number | null; auto_apply: boolean; waiting: WaitingRelease | null;
}
export interface ShownItem { name: string; rarity: number }
export interface ImpactReason { text: string; change_ids: string[] }
export interface AffectedPull {
  game: GameId; uid: string; pull_id: string; time: string; item_id: string; name: string; rank: number;
  banner_before: string | null; banner_after: string | null; result_before: PullOutcome | null; result_after: PullOutcome | null;
  pity_before: number | null; pity_after: number | null; shown_before: ShownItem; shown_after: ShownItem; reasons: ImpactReason[];
}
export interface AccountImpact { game: GameId; uid: string; nickname: string; pulls: number; affected: number; listed: number }
export interface CatalogReview {
  version: number; current_version: number; generated_at: string; changes: ChangeEntry[]; fingerprint: string;
  accounts: AccountImpact[]; total_affected: number; not_listed: number; dismissed: boolean;
}
export interface ApplyResult { applied: boolean; stale: boolean }
export interface CatalogHistoryEntry { version: number; applied_at: number; automatic: boolean; affected_pulls: number; changes_total: number; changes: ChangeEntry[] }

export interface FetchSummary { game: GameId; uid: string; fetched: number; added: number; warning: string | null }
export interface ImportSummary { format: string; accounts: number; added: number; skipped: number }
export interface CloudStatus { available: boolean; signed_in: boolean; account: string | null; auto_backup: boolean; last_backup: string | null }
export interface CloudBackup { id: string; created: string; size: number }
export interface Credit { name: string; role: string; url: string; license: string }
export interface CreditGroup { title: string; note: string; credits: Credit[] }
export interface AboutInfo {
  name: string; version: string; description: string; copyright: string; license: string; disclaimers: string[];
  repository: string; releases: string; issues: string; groups: CreditGroup[];
}
export interface AppUpdate { current: string; latest: string; available: boolean; url: string; prerelease: boolean; published_at: string }
export interface LogEntry { seq: number; time: string; level: string; source: string; message: string }

export type ErrorCode = 'game_not_found' | 'history_page_not_opened' | 'auth_expired' | 'invalid_url' | 'rate_limited' | 'api' | 'network' | 'database' | 'file' | 'format' | 'other';
export interface ApiError { code: ErrorCode; message: string }
export interface RefreshEvent { kind: 'progress' | 'finished' | 'failed'; message: string; summary: FetchSummary | null; error: ApiError | null }
