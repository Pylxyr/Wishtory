// A stand-in for the Rust commands, used only when the UI is opened in a plain browser with VITE_MOCK=1
// (see e2e/shots.mjs). It answers with real catalog and banner data from this repository and a
// deterministic made-up wish history, so every screen can be looked at without the game.
import catalog from '../../catalog/catalog.json';
import banners from '../../catalog/banners.json';
import type * as T from './types';
import type { GameId } from './types';

/** The real backend sends no banner key; the mock keeps one only to filter by banner. */
type MockPull = T.HistoryPull & { banner_key: string };

const KEY = { genshin: 'hk4e', star_rail: 'hkrpg', zenless: 'nap' } as const;
const items = (catalog as unknown as T.GameCatalog).items;
const GAME_OF = { hk4e: 'genshin', hkrpg: 'star_rail', nap: 'zenless' } as const;
// The real API serialises the game as genshin / star_rail / zenless; the bundled file uses the saved-record keys.
const bannerList: T.BannerList = {
  ...(banners as unknown as T.BannerList),
  banners: (banners as unknown as { banners: Record<string, any>[] }).banners.map((b) => ({ featured_4: [], gacha_ids: [], ...b, game: GAME_OF[b.game as keyof typeof GAME_OF] })) as unknown as T.BannerEntry[]
};

let s = 1234567;
const rnd = () => ((s = (s * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
const pick = <A>(a: A[]) => a[Math.floor(rnd() * a.length)];

const accounts: T.Account[] = [
  { game: 'genshin', uid: '812345678', nickname: 'Lumine', timezone: 8, pull_count: 0 },
  { game: 'star_rail', uid: '702468135', nickname: 'Stelle', timezone: 8, pull_count: 0 },
  { game: 'zenless', uid: '1300975312', nickname: 'Belle', timezone: 8, pull_count: 0 }
];

const FILTERS: Record<GameId, T.BannerFilter[]> = {
  genshin: [{ key: 'character', label: 'Character Event' }, { key: 'weapon', label: 'Weapon Event' }, { key: 'standard', label: 'Standard' }, { key: 'chronicled', label: 'Chronicled' }],
  star_rail: [{ key: 'character', label: 'Character Warp' }, { key: 'weapon', label: 'Light Cone Warp' }, { key: 'standard', label: 'Stellar Warp' }, { key: 'collab', label: 'Collaboration Warp' }, { key: 'collab_weapon', label: 'Collaboration Light Cone Warp' }],
  zenless: [{ key: 'character', label: 'Exclusive Channel' }, { key: 'weapon', label: 'W-Engine Channel' }, { key: 'standard', label: 'Standard Channel' }, { key: 'bangboo', label: 'Bangboo Channel' }, { key: 'rescreening', label: 'Exclusive Rescreening' }, { key: 'reverberation', label: 'W-Engine Reverberation' }]
};

const names = (game: GameId, rarity: number, kind: 'character' | 'weapon') =>
  items.filter((i) => i.game === KEY[game] && i.rarity === rarity && (kind === 'character' ? i.kind === 'character' : i.kind !== 'character')).map((i) => i.name);

const history: Record<string, MockPull[]> = {};
function build(game: GameId, uid: string) {
  const out: MockPull[] = [];
  const keys = FILTERS[game].slice(0, game === 'zenless' ? 3 : 3).map((f) => f.key);
  let t = Date.UTC(2026, 10, 24, 19, 0, 0);
  let n = 0;
  for (const banner of keys) {
    const count = banner === 'character' ? 120 : banner === 'weapon' ? 60 : 40;
    const kind = banner === 'weapon' ? 'weapon' : 'character';
    let since5 = 0, since4 = 0;
    for (let i = 0; i < count; i++) {
      since5++; since4++;
      let rank = 3;
      const hard = banner === 'weapon' ? 80 : 90;
      if (since5 >= hard || rnd() < (since5 > 72 ? 0.25 : 0.012)) rank = 5;
      else if (since4 >= 10 || rnd() < 0.09) rank = 4;
      const nm = rank === 5 ? pick(names(game, 5, kind)) : rank === 4 ? pick(names(game, 4, rnd() < 0.5 ? 'character' : 'weapon')) : pick(items.filter((x) => x.game === KEY[game] && x.rarity === 3).map((x) => x.name));
      const pity = rank === 5 ? since5 : rank === 4 ? since4 : null;
      if (rank === 5) since5 = 0;
      if (rank >= 4) since4 = 0;
      t -= 1000 * (30 + Math.floor(rnd() * 3600 * (i % 10 === 0 ? 40 : 0.2)));
      const iso = new Date(t + 8 * 3_600_000).toISOString().slice(0, 19).replace('T', ' ');
      out.push({
        id: `${uid}-${banner}-${n++}`, gacha_type: banner, gacha_id: '', item_id: '', name: nm, item_type: kind === 'weapon' ? 'Weapon' : 'Character',
        time: iso, rank, pool: FILTERS[game].find((f) => f.key === banner)!.label.split(' ')[0], banner_key: banner,
        banner_name: rank >= 4 ? `Event · ${nm}` : null, pity
      });
    }
  }
  out.sort((a, b) => (a.time < b.time ? 1 : -1));
  return out;
}
for (const a of accounts) { history[`${a.game}|${a.uid}`] = build(a.game, a.uid); a.pull_count = history[`${a.game}|${a.uid}`].length; }

function pityFor(game: GameId, uid: string): T.BannerPity[] {
  const pulls = history[`${game}|${uid}`] ?? [];
  const rules: Record<string, [number, number | null, boolean | null, number | null]> = {
    character: [90, 74, game === 'genshin' ? true : false, 50], weapon: [80, game === 'genshin' ? 63 : 65, null, null], standard: [90, 74, null, null]
  };
  const now: Record<GameId, Record<string, number>> = {
    genshin: { character: 61, weapon: 33, standard: 12 }, star_rail: { character: 78, weapon: 29, standard: 7 }, zenless: { character: 12, weapon: 41, standard: 3 }
  };
  return FILTERS[game].slice(0, 3).map((f) => {
    const mine = pulls.filter((p) => p.banner_key === f.key);
    const fives = mine.filter((p) => p.rank === 5).reverse();
    const fours = mine.filter((p) => p.rank === 4).reverse();
    const r = rules[f.key];
    return {
      banner: f.key, label: f.label, pity: now[game][f.key], hard_pity: r[0], soft_pity: r[1], guaranteed: f.key === 'character' ? (game === 'star_rail' ? false : r[2]) : r[2],
      guarantee_odds: r[3], total: mine.length, five_stars: fives.length, four_stars: fours.length,
      avg_five_pity: fives.length ? fives.reduce((a, p) => a + (p.pity ?? 0), 0) / fives.length : null,
      avg_four_pity: fours.length ? fours.reduce((a, p) => a + (p.pity ?? 0), 0) / fours.length : null,
      recent_fives: fives.slice(-6).map((p, i) => ({ name: p.name, pity: p.pity ?? 0, time: p.time, off_banner: f.key === 'character' ? i % 3 === 1 : null, outcome: f.key === 'character' ? (['win', 'loss', 'guaranteed', 'win', 'win', 'loss'] as T.PullOutcome[])[i % 6] : null })),
      recent_fours: fours.slice(-6).map((p) => ({ name: p.name, pity: p.pity ?? 0, time: p.time, is_character: p.item_type === 'Character' }))
    };
  });
}

export class MockChannel<M> { onmessage: (m: M) => void = () => {}; }
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
let cloud: T.CloudStatus = { available: true, signed_in: false, account: null, auto_backup: false, last_backup: null };
const status: T.CatalogStatus = {
  version: 5, components: { catalog: 5, banners: 5, rules: 1 }, generated_at: '2026-11-14T03:33:41Z', last_check: Math.floor(Date.now() / 1000) - 3600 /* Unix seconds, like the real backend */, auto_apply: true,
  waiting: { version: 6, generated_at: '2026-11-24T03:00:00Z', major: 0, minor: 2, info: 5, dismissed: false, affected_pulls: 3, affected_accounts: 1 }
};

export async function invoke<R>(cmd: string, args: Record<string, any> = {}): Promise<R> {
  await wait(30);
  const r = (v: unknown) => v as R;
  // Tests can put edge-case answers in `window.__wishtoryMock` (by command) before the page loads, and
  // count the calls the window makes in `window.__wishtoryCalls`.
  const test = globalThis as { __wishtoryMock?: Record<string, (args: Record<string, any>, real: () => unknown) => unknown>; __wishtoryCalls?: { cmd: string; args: Record<string, any> }[] };
  (test.__wishtoryCalls ??= []).push({ cmd, args });
  const hook = test.__wishtoryMock?.[cmd];
  if (hook) return r(await hook(args, () => invokeReal<R>(cmd, args)));
  return invokeReal<R>(cmd, args);
}

// Small stateful stand-ins so the Settings and Accounts screens can be exercised end to end.
let locations = ['/home/you/Games/genshin'];
let cloudBackups: T.CloudBackup[] = [];

async function invokeReal<R>(cmd: string, args: Record<string, any>): Promise<R> {
  const r = (v: unknown) => v as R;
  switch (cmd) {
    case 'startup_error': return r(null);
    case 'discover_games': return r(accounts.map((a) => ({ game: a.game, found: true, log_path: '/mock', data_dir: '/mock' })));
    case 'get_game_locations': return r({ supported: true, locations, detected: [{ path: '/home/you/.local/share/lutris/prefixes/genshin', games: ['genshin'] }, { path: '/home/you/.steam/steamapps/compatdata/1', games: ['star_rail', 'zenless'] }] });
    case 'set_game_locations': locations = args.locations; return r({ supported: true, locations, detected: [] });
    case 'clear_all_history': { const n = accounts.reduce((t, a) => t + a.pull_count, 0); accounts.splice(0); for (const k of Object.keys(history)) delete history[k]; return r(n); }
    case 'clear_logs': return r(undefined);
    case 'set_logging_enabled': return r(undefined);
    case 'cloud_sign_in': if ([...String(args.passphrase)].length < 10) throw { code: 'other', message: 'Choose a backup passphrase of at least 10 characters.' }; await wait(400); cloud = { ...cloud, signed_in: true, account: 'lumine@example.com' }; return r(cloud);
    case 'cloud_cancel_sign_in': return r(undefined);
    case 'cloud_sign_out': cloud = { ...cloud, signed_in: false, account: null }; if (args.deleteBackups) cloudBackups = []; return r(cloud);
    case 'cloud_backup_now': { const b = { id: 'b' + (cloudBackups.length + 1), created: new Date().toISOString(), size: 48213 }; cloudBackups = [b, ...cloudBackups].slice(0, 5); cloud = { ...cloud, last_backup: b.created }; return r(b); }
    case 'cloud_backup_if_due': return r(null);
    case 'cloud_restore': return r({ format: 'UIGF v4.2', accounts: 3, added: 0, skipped: 660 });
    case 'game_meta': return r((Object.keys(FILTERS) as GameId[]).map((g) => ({ game: g, banners: FILTERS[g] })));
    case 'list_accounts': return r(accounts.map((a) => ({ ...a })));
    case 'get_history': return r((history[`${args.game}|${args.uid}`] ?? []).filter((p) => (!args.banner || p.banner_key === args.banner) && (!args.minRank || p.rank >= args.minRank)));
    case 'get_pity': return r(pityFor(args.game, args.uid));
    case 'get_catalog': return r(catalog);
    case 'get_banners': return r(bannerList);
    case 'get_catalog_status': return r(status);
    case 'check_catalog': return r({ status: 'needs_review', message: null });
    case 'get_catalog_review': return r({
      version: 6, current_version: 5, generated_at: status.waiting!.generated_at, fingerprint: 'mock', total_affected: 3, not_listed: 0, dismissed: false,
      changes: [
        { id: 'items:hk4e:rename', scope: 'items', game: 'genshin', ids: ['1'], summary: 'Item renamed: "Mitya (Mock)" is now "Mitya"', severity: 'minor', note: null },
        { id: 'banners:nap:3.4', scope: 'banners', game: 'zenless', ids: [], summary: 'Added banners for Version 3.4', severity: 'info', note: null }
      ],
      accounts: [{ game: 'genshin', uid: '812345678', nickname: 'Lumine', pulls: 220, affected: 3, listed: 3 }]
    });
    case 'get_catalog_history': return r([]);
    case 'set_catalog_auto_apply': return r(args.enabled);
    case 'cloud_status': return r(cloud);
    case 'cloud_set_auto_backup': cloud = { ...cloud, auto_backup: args.enabled }; return r(cloud);
    case 'cloud_list_backups': return r(cloudBackups);
    case 'get_about': return r({
      name: 'Wishtory', version: '0.6.0-rc2', description: 'Track your wish history, pity and 50/50 results.', copyright: '© Wishtory contributors', license: 'MIT',
      disclaimers: ['Wishtory is an independent client and is not affiliated with HoYoverse.'], repository: 'https://github.com/Pylxyr/Wishtory', releases: '', issues: '',
      groups: [{ title: 'Data', note: 'Catalog data and pictures come from community projects.', credits: [{ name: 'Enka.Network', role: 'Item data and pictures', url: 'https://enka.network', license: 'Public API' }] }]
    });
    case 'check_app_update': return r({ current: '0.6.0-rc2', latest: '0.6.0-rc2', available: false, url: '', prerelease: true, published_at: '' });
    case 'get_logs': return r([{ seq: 1, time: '12:00:01', level: 'info', source: 'app', message: 'Mock backend: nothing is really read or sent.' }]);
    case 'refresh_from_game':
    case 'fetch_history': {
      const ch = args.onEvent as MockChannel<T.RefreshEvent>;
      for (const m of ['Found the history link in the game cache…', 'Downloading Character Event: page 1…', 'Downloading Character Event: page 2…', 'Downloading Weapon Event: page 1…']) {
        ch.onmessage({ kind: 'progress', message: m, summary: null, error: null });
        await wait(500);
      }
      const acc = accounts.find((a) => a.game === args.game)!;
      ch.onmessage({ kind: 'finished', message: '', summary: { game: args.game, uid: acc.uid, fetched: 214, added: 9, warning: null }, error: null });
      return r(undefined);
    }
    case 'import_file': return r({ format: 'UIGF v4.0', accounts: 1, added: 120, skipped: 4 });
    default: return r(undefined);
  }
}
