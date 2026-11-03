// Mock of the Tauri bridge for read-only UI rendering. Synthetic data; not real pulls.
(function () {
  const q = new URLSearchParams(location.search);
  const scenario = q.get('s') || 'normal';
  let seed = 424242;
  const rnd = () => (seed = (seed * 1664525 + 1013904223) % 4294967296) / 4294967296;
  const pick = (a) => a[Math.floor(rnd() * a.length)];

  const NAMES = {
    hk4e: {
      c5: ['Venti', 'Diluc', 'Mona', 'Jean', 'Qiqi', 'Keqing', 'Tighnari', 'Dehya', 'Wanderer', 'Nahida', 'Kazuha', 'Yelan'],
      w5: ["Skyward Harp", "Amos' Bow", 'Aquila Favonia', 'Primordial Jade Winged-Spear', "Wolf's Gravestone", 'Skyward Blade'],
      c4: ['Bennett', 'Xiangling', 'Fischl', 'Sucrose', 'Noelle', 'Rosaria', 'Dori', 'Layla'],
      w4: ['The Flute', 'Favonius Sword', 'Sacrificial Bow', 'Rainslasher', 'Lion\u2019s Roar'],
      w3: ['Slingshot', 'Debate Club', 'Black Tassel', 'Cool Steel', 'Harbinger of Dawn', 'Raven Bow'],
      el: { Venti: 'anemo', Diluc: 'pyro', Mona: 'hydro', Jean: 'anemo', Qiqi: 'cryo', Keqing: 'electro', Tighnari: 'dendro', Dehya: 'pyro', Wanderer: 'anemo', Nahida: 'dendro', Kazuha: 'anemo', Yelan: 'hydro' }
    },
    hkrpg: {
      c5: ['Seele', 'Bronya', 'Clara', 'Gepard', 'Bailu', 'Himeko', 'Welt', 'Kafka', 'Jingliu'],
      w5: ['Night on the Milky Way', 'In the Name of the World', 'Before Dawn', 'Time Waits for No One'],
      c4: ['Natasha', 'Pela', 'Serval', 'Tingyun', 'Hook', 'Qingque', 'Sampo'],
      w4: ['Post-Op Conversation', 'Perfect Timing', 'Make the World Clamor', 'Good Night and Sleep Well'],
      w3: ['Arrows', 'Cornucopia', 'Defense', 'Multiplication', 'Mutual Demise', 'Pioneering'],
      el: { Seele: 'quantum', Bronya: 'wind', Clara: 'physical', Gepard: 'ice', Bailu: 'lightning', Himeko: 'fire', Welt: 'imaginary', Kafka: 'lightning', Jingliu: 'ice' }
    },
    nap: {
      c5: ['Ellen Joe', 'Zhu Yuan', 'Qingyi', 'Jane Doe', 'Burnice White', 'Soukaku'],
      w5: ['Deep Sea Visitor', 'Steel Cushion', 'Original Transmorphic', 'The Brimstone'],
      c4: ['Nicole Demara', 'Billy Kid', 'Anton Ivanov', 'Soldier 11', 'Ben Bigger'],
      w4: ['Starlight Engine', 'Drill Rig - Red Axis', 'Cannon Rotor', 'Bashful Demon'],
      w3: ['Housekeeper', 'Slice of Time'],
      el: {}
    }
  };
  const POOLS = {
    hk4e: [
      { key: 'character', label: 'Character Event', gt: '301', pool: 'Character', hard: 90, soft: 74, odds: 50, n: 520, end: 17, kind: 'c', banners: ['Ballad in Goblets', 'Gentry of Hermitage', 'Invitation to Mundane Life', 'Sprouting Dawn', 'Song of the Welkin Moon', 'Fleeting Colors in Flight', 'Kitsune Fireworks'] },
      { key: 'weapon', label: 'Weapon Event', gt: '302', pool: 'Weapon', hard: 80, soft: 63, odds: 75, n: 190, end: 66, kind: 'w', banners: ['Epitome Invocation'] },
      { key: 'standard', label: 'Standard', gt: '200', pool: 'Standard', hard: 90, soft: 74, odds: null, n: 240, end: 31, kind: 'cw', banners: ['Wanderlust Invocation'] },
      { key: 'chronicled', label: 'Chronicled', gt: '500', pool: 'Chronicled', hard: 90, soft: 74, odds: 50, n: 30, end: 9, kind: 'cw', banners: ['Chronicled Wish'] },
      { key: 'beginner', label: "Beginners'", gt: '100', pool: "Beginners'", hard: 20, soft: null, odds: null, n: 20, end: 0, kind: 'c', banners: ["Beginners' Wish"], no5: true }
    ],
    hkrpg: [
      { key: 'character', label: 'Character Warp', gt: '11', pool: 'Character', hard: 90, soft: 74, odds: 50, n: 210, end: 52, kind: 'c', banners: ['Star Rail Special Warp', 'Jade in the Wind', 'Moonlit Cities'] },
      { key: 'weapon', label: 'Light Cone Warp', gt: '12', pool: 'Light Cone', hard: 80, soft: 66, odds: 75, n: 80, end: 12, kind: 'w', banners: ['Brilliant Fixation'] },
      { key: 'standard', label: 'Stellar Warp', gt: '1', pool: 'Stellar', hard: 90, soft: 74, odds: null, n: 90, end: 40, kind: 'cw', banners: ['Stellar Warp'] }
    ],
    nap: [
      { key: 'character', label: 'Exclusive Channel', gt: '2', pool: 'Exclusive', hard: 90, soft: 74, odds: 50, n: 120, end: 21, kind: 'c', banners: ['Hot Pursuit', 'Rewind of Time'] },
      { key: 'weapon', label: 'W-Engine Channel', gt: '3', pool: 'W-Engine', hard: 80, soft: 64, odds: 75, n: 60, end: 5, kind: 'w', banners: ['Steel Resolve'] },
      { key: 'standard', label: 'Standard Channel', gt: '1', pool: 'Standard', hard: 90, soft: 74, odds: null, n: 50, end: 0, kind: 'cw', banners: ['Stable Channel'] },
      { key: 'bangboo', label: 'Bangboo Channel', gt: '5', pool: 'Bangboo', hard: 80, soft: null, odds: null, n: 20, end: 3, kind: 'c', banners: ['Bangboo Channel'] },
      { key: 'rescreening', label: 'Exclusive Rescreening', gt: '102', pool: 'Rescreening', hard: 90, soft: 74, odds: null, n: 10, end: 10, kind: 'c', no5: true, banners: ['Rescreening'] },
      { key: 'reverberation', label: 'W-Engine Reverberation', gt: '103', pool: 'Reverberation', hard: 80, soft: 64, odds: null, n: 0, end: 0, kind: 'w', banners: ['Reverberation'] }
    ]
  };

  function fmt(ms) {
    const d = new Date(ms); const p = (n) => String(n).padStart(2, '0');
    return `${d.getUTCFullYear()}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())} ${p(d.getUTCHours())}:${p(d.getUTCMinutes())}:${p(d.getUTCSeconds())}`;
  }

  const store = {}; // key game:uid -> { pulls (oldest..newest), pity[] }
  function build(game, uid, scale) {
    const N = NAMES[game]; const out = []; const pity = [];
    let idc = 1000000000000000000n + BigInt(Math.floor(rnd() * 1e9));
    const t0 = Date.UTC(2025, 7, 1, 12), t1 = Date.UTC(2026, 9, 29, 20);
    for (const P of POOLS[game]) {
      const n = Math.round(P.n * scale); let p5 = 0, p4 = 0, guaranteed = false, fives = [], fours = [], pulls = [];
      const forceAt = P.end > 0 && n > P.end + 5 ? n - P.end - 1 : -1;
      for (let i = 0; i < n; i++) {
        p5++; p4++;
        let r5 = 0;
        if (!P.no5) {
          let pr = 0.006;
          if (P.soft && p5 >= P.soft) pr += (p5 - P.soft + 1) * 0.06;
          if (p5 >= P.hard) pr = 1;
          const inTail = forceAt >= 0 && i > forceAt;
          if (!inTail && (rnd() < pr || i === forceAt)) r5 = 1;
          if (i === forceAt) r5 = 1;
        }
        let rank = 3;
        if (r5) rank = 5; else if (p4 >= 10 || rnd() < 0.051) rank = 4;
        const when = t0 + (t1 - t0) * (i / Math.max(n, 1)) + Math.floor(i / 10) * 0 + (i % 10) * 1000;
        const bi = Math.min(P.banners.length - 1, Math.floor((i / n) * P.banners.length));
        let name, type, itemId;
        const wpn = P.kind === 'w' || (P.kind === 'cw' && rnd() < 0.5);
        const wType = game === 'hkrpg' ? 'Light Cone' : game === 'nap' ? 'W-Engine' : 'Weapon';
        const cType = game === 'nap' && P.key === 'bangboo' ? 'Bangboo' : 'Character';
        if (rank === 5) { name = pick(wpn ? N.w5 : N.c5); type = wpn ? wType : cType; }
        else if (rank === 4) { name = pick(wpn ? N.w4 : (rnd() < 0.5 ? N.c4 : N.w4)); type = N.w4.includes(name) ? wType : cType; }
        else { name = pick(N.w3); type = wType; }
        let outcome = null, off = null, pityVal = null;
        if (rank === 5) {
          pityVal = p5;
          if (P.odds !== null) {
            if (guaranteed) { outcome = 'guaranteed'; off = false; guaranteed = false; }
            else if (rnd() * 100 < P.odds) { outcome = 'win'; off = false; }
            else { outcome = 'loss'; off = true; guaranteed = true; }
          }
          fives.push({ name, pity: p5, time: fmt(when), off_banner: off, outcome });
          p5 = 0; p4 = 0;
        } else if (rank === 4) { pityVal = p4; fours.push({ name, pity: p4, time: fmt(when) }); p4 = 0; }
        pulls.push({
          id: String(idc++), game, uid, gacha_type: P.gt, gacha_id: '1', item_id: String(Math.floor(rnd() * 99999)),
          name, item_type: type, rank_type: game === 'nap' ? rank - 1 : rank, time: fmt(when), lang: 'en', count: 1,
          banner_name: P.banners[bi] + (P.key === 'character' && game === 'hk4e' ? ' \u00b7 ' + pick(N.c5) : ''),
          pity: pityVal, rank, pool: P.pool, _key: P.key
        });
      }
      const avg = (a) => (a.length ? a.reduce((s, x) => s + x.pity, 0) / a.length : null);
      pity.push({
        banner: P.key, label: P.label, pity: p5, hard_pity: P.hard, soft_pity: P.soft, guaranteed: P.odds !== null ? (fives.length ? guaranteed : null) : null,
        guarantee_odds: P.odds, total: n, five_stars: fives.length, four_stars: fours.length, avg_five_pity: avg(fives), avg_four_pity: avg(fours),
        recent_fives: fives, recent_fours: fours.slice(-40)
      });
      out.push(...pulls);
    }
    out.sort((a, b) => a.time.localeCompare(b.time));
    if (uid === '600987654' && out.length) out[out.length - 1].name = '=1+1, "test"';
    return { pulls: out, pity };
  }

  const accountsAll = [
    { game: 'hk4e', uid: '800123456', nickname: 'Traveler', timezone: 8, pull_count: 0, scale: scenario === 'big' ? 50 : 1 },
    { game: 'hk4e', uid: '600987654', nickname: '', timezone: -5, pull_count: 0, scale: 0.18 },
    { game: 'hkrpg', uid: '700111222', nickname: 'Trailblazer', timezone: 8, pull_count: 0, scale: 1 },
    { game: 'nap', uid: '1500123456', nickname: 'Proxy', timezone: 8, pull_count: 0, scale: 1 }
  ];
  let accounts = scenario === 'empty' ? [] : accountsAll;
  accounts.forEach((a) => { const d = build(a.game, a.uid, a.scale); store[a.game + ':' + a.uid] = d; a.pull_count = d.pulls.length; });

  const GM = Object.keys(POOLS).map((g) => ({ game: g, banners: POOLS[g].map((p) => ({ key: p.key, label: p.label })) }));
  const catalogItems = [];
  for (const g of Object.keys(NAMES)) for (const n of NAMES[g].c5) catalogItems.push({ game: g, id: 'x-' + n, name: n, aliases: [], rarity: 5, kind: 'character', traits: NAMES[g].el[n] ? { element: NAMES[g].el[n] } : {}, paimon_key: null });
  const banners = [];
  const bannersSet = () => ({ schema_version: 1, banners_version: 42, generated_at: '2026-10-28T00:00:00Z', banners });
  const pad2 = (n) => String(n).padStart(2, '0');
  const naive = (ms) => { const d = new Date(ms); return `${d.getUTCFullYear()}-${pad2(d.getUTCMonth() + 1)}-${pad2(d.getUTCDate())} ${pad2(d.getUTCHours())}:${pad2(d.getUTCMinutes())}:${pad2(d.getUTCSeconds())}`; };
  const serverNow = Date.now() + 8 * 3600e3; // the fixture accounts are UTC+8
  for (const g of Object.keys(POOLS)) POOLS[g].filter((p) => p.key === 'character' || p.key === 'weapon').forEach((p) => p.banners.forEach((b, i) => {
    const live = i === p.banners.length - 1; // the newest banner of each pool is running now
    const start = live ? serverNow - 10 * 86400e3 : serverNow - (60 + 25 * (p.banners.length - i)) * 86400e3;
    const end = live ? serverNow + (6 * 86400e3 + 4 * 3600e3 + (p.key === 'weapon' ? 0 : 7200e3)) : start + 21 * 86400e3;
    banners.push({ id: `${g}-${p.key}-${i}`, game: g, pool: p.key, name: b, gacha_ids: ['1'], start: naive(start), end: naive(end), featured_5: ['x-' + NAMES[g].c5[i % NAMES[g].c5.length]], featured_4: [] });
  }));
  const bannerUpdate = { status: 'updated', banners_version: 42, installed_at: 1793000000, added: banners.slice(0, 4).map((b) => b.id), changed: banners.slice(4, 6).map((b) => b.id), removed: ['hk4e-character-removed'] };

  const about = {
    name: 'Wishtory', version: '0.1.0', description: 'A local-first companion for tracking gacha pull history.', copyright: 'Copyright \u00a9 2026', license: 'MIT License',
    disclaimers: ['Wishtory is an unofficial fan project and is not affiliated with HoYoverse.'], repository: 'https://example.invalid/repo', releases: 'https://example.invalid/releases', issues: 'https://example.invalid/issues',
    groups: [{ title: 'Standards', note: 'Formats Wishtory reads and writes.', credits: [{ name: 'UIGF', role: 'Wish history interchange format', url: 'https://uigf.org/', license: 'Open standard' }] }]
  };
  const logs = [];
  for (let i = 0; i < 40; i++) logs.push({ seq: i, time: '2026-10-31 12:00:' + String(i).padStart(2, '0') + '.123', level: ['DEBUG', 'INFO', 'WARN', 'ERROR'][i % 4], source: ['fetch', 'db', 'catalog'][i % 3], message: 'Synthetic log line ' + i + ' for rendering' });

  let n = 1; const handlers = {};
  window.__TAURI_INTERNALS__ = {
    transformCallback(cb) { const id = n++; window['_' + id] = cb; return id; },
    unregisterCallback() {}, convertFileSrc: (p) => p, metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main', windowLabel: 'main' } },
    async invoke(cmd, args) {
      switch (cmd) {
        case 'plugin:event|listen': handlers[args.event] = args.handler; return n++;
        case 'plugin:event|unlisten': return null;
        case 'plugin:dialog|open': case 'plugin:dialog|save': return null;
        case 'discover_games': return [{ game: 'hk4e', found: true, log_path: 'C:/x', data_dir: 'C:/x' }, { game: 'hkrpg', found: true, log_path: 'C:/x', data_dir: 'C:/x' }, { game: 'nap', found: false, log_path: null, data_dir: null }];
        case 'list_accounts': return accounts.map(({ scale, ...a }) => a);
        case 'get_game_meta': return GM;
        case 'get_catalog': return { schema_version: 1, catalog_version: 17, generated_at: '2026-10-20T00:00:00Z', items: catalogItems };
        case 'refresh_catalog': return 'up-to-date';
        case 'get_banners': return bannersSet();
        case 'get_banner_update': return bannerUpdate;
        case 'refresh_banners': return { ...bannerUpdate, status: 'up-to-date' };
        case 'get_about': return about;
        case 'check_app_update': return { current: '0.1.0', latest: '0.1.0', available: false, url: '', prerelease: false, published_at: '' };
        case 'set_logging_enabled': if (scenario === 'toasterr') throw 'Could not change logging.'; return null;
        case 'get_logs': return logs;
        case 'clear_logs': return null;
        case 'open_external': return null;
        case 'get_pity': {
          if (scenario === 'pityerr') throw 'Database error: disk I/O error'; const d = store[args.game + ':' + args.uid]; return d ? d.pity : []; }
        case 'get_history': {
          const d = store[args.game + ':' + args.uid]; if (!d) return [];
          let p = d.pulls; if (args.banner) p = p.filter((x) => x._key === args.banner);
          return p.slice().reverse().map(({ _key, ...r }) => r);
        }
        case 'refresh_from_game': case 'fetch_history': {
          const say = (t) => { const h = handlers['refresh-progress']; if (h && window['_' + h]) window['_' + h]({ event: 'refresh-progress', id: 1, payload: t }); };
          say('Looking for the history link\u2026'); await new Promise((r) => setTimeout(r, 150));
          say('Character Event: 20 pulls read');
          if (scenario === 'busy') { say('Weapon Event: downloading page 3'); return new Promise(() => {}); }
          if (scenario === 'expired-once' && !window.__failedOnce) { window.__failedOnce = true; throw "This history link has expired. In the game, open the Wish / Warp / Signal Search history page again, wait until the list loads, then click Refresh."; }
          if (scenario === 'rate') throw "The server asked us to slow down. Please wait a minute and try again.";
          if (scenario === 'network') throw "Network problem: error sending request. Check your internet connection (and turn off any VPN or proxy that blocks HoYoverse), then click Refresh again.";
          if (scenario === 'expired') throw "This history link has expired. In the game, open the Wish / Warp / Signal Search history page again, wait until the list loads, then click Refresh.";
          return { game: args.game, uid: '800123456', fetched: 40, added: 12, warning: null };
        }
        default: return null;
      }
    }
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
})();
