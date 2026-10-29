// Tracks the banner schedule that Paimon.moe publishes and drafts what the banner list is missing.
//
//   node scripts/track-banners.mjs [--previous old-catalog.json] [--report banners-report.md]
//
// Only Genshin Impact has a machine-readable public schedule, so the draft covers its character events.
// For every game it also lists 5-star characters that are new in the item catalog (compared with
// --previous) and still need a banner entry. Existing entries are never rewritten: differences are only
// reported, because the list was checked by hand against HoYoverse's notices.
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const PAIMON_BANNERS = 'https://raw.githubusercontent.com/MadeBaruna/paimon-moe/main/src/data/banners.js';
// The first update whose windows were checked against the official schedule. Earlier rows are left alone.
const FROM = '2023-07-05';

const field = (block, key) => {
  const re = new RegExp(String.raw`(?:^|\s)${key}:\s*(?:'((?:\\.|[^'\\])*)'|"((?:\\.|[^"\\])*)")`);
  const m = re.exec(block);
  return m ? (m[1] ?? m[2]).replace(/\\(.)/g, '$1') : null;
};

// Reads the data without running it: Paimon.moe's file is a JavaScript module, and a download must
// never be executed with a write token in reach.
export function parsePaimonCharacters(text) {
  const start = text.indexOf('characters: [');
  if (start < 0) throw new Error('Paimon.moe banner data has no characters list.');
  const end = text.indexOf('\n  ],', start);
  if (end < 0) throw new Error('Paimon.moe banner data changed shape.');
  const body = text.slice(start, end).split('\n').filter((line) => !line.trim().startsWith('//')).join('\n');
  const rows = [];
  for (const [block] of body.matchAll(/\{[^{}]*\}/g)) {
    const featuredList = /(?:^|\s)featured:\s*\[([^\]]*)\]/.exec(block)?.[1] ?? '';
    const row = {
      name: field(block, 'name'),
      start: field(block, 'start'),
      end: field(block, 'end'),
      version: field(block, 'version'),
      featured: (featuredList.match(/'[^']*'|"[^"]*"/g) ?? []).map((s) => s.slice(1, -1)),
    };
    if (row.name && row.start && row.end && row.featured.length) rows.push(row);
  }
  if (!rows.length) throw new Error('Paimon.moe banner data held no character events.');
  return rows;
}

const dayBefore = (date) => {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() - 1);
  return d.toISOString().slice(0, 10);
};

// Paimon.moe opens an update's first phase at 06:00 (the maintenance) and the second at 18:00. The list
// uses the conventions in docs/game-catalog-updates.md: the first phase is listed from 15:00 the day
// before, and every phase ends at 17:59:59 or 14:59:59.
export function toWindow(row) {
  const [startDate, startTime] = row.start.split(' ');
  const [endDate, endTime] = row.end.split(' ');
  const launch = startTime.startsWith('06:00');
  if (!launch && !startTime.startsWith('18:00')) return null;
  if (!/^(14|17):59:/.test(endTime)) return null;
  return {
    start: launch ? `${dayBefore(startDate)} 15:00:00` : `${startDate} 18:00:00`,
    end: `${endDate} ${endTime.slice(0, 6)}59`,
  };
}

export function trackBanners(upstream, ours, items) {
  const idByKey = new Map(
    items.filter((i) => i.game === 'hk4e' && i.kind === 'character' && i.paimon_key).map((i) => [i.paimon_key, i.id]),
  );
  const have = new Map(ours.filter((b) => b.game === 'hk4e' && b.pool === 'character').map((b) => [b.id, b]));
  const lastStart = [...have.values()].map((b) => b.start).sort().at(-1) ?? '';
  const added = [];
  const problems = [];
  for (const row of upstream) {
    const date = row.start.slice(0, 10);
    if (date < FROM) continue;
    const id = `hk4e-character-${date}`;
    const label = `${row.name} (${date})`;
    const window = toWindow(row);
    if (!window) {
      problems.push(`${label}: Paimon.moe lists times this check does not recognise (${row.start} to ${row.end}).`);
      continue;
    }
    const unmapped = row.featured.filter((key) => !idByKey.has(key));
    if (unmapped.length) {
      problems.push(`${label}: the item catalog has no entry for ${unmapped.join(', ')}. Update the item catalog first.`);
      continue;
    }
    const featured = row.featured.map((key) => idByKey.get(key));
    const mine = have.get(id);
    if (!mine) {
      if (window.start > lastStart) {
        added.push({ id, game: 'hk4e', pool: 'character', name: row.name, start: window.start, end: window.end, featured_5: featured });
      } else {
        problems.push(`${label}: Paimon.moe has this event but the banner list does not.`);
      }
      continue;
    }
    const diff = [];
    if ([...mine.featured_5].sort().join() !== [...featured].sort().join()) diff.push(`featured ${mine.featured_5.join('/')} against ${featured.join('/')}`);
    if (mine.start !== window.start) diff.push(`start ${mine.start} against ${window.start}`);
    if (mine.end !== window.end) diff.push(`end ${mine.end} against ${window.end}`);
    if (diff.length) problems.push(`${label}: the banner list differs from Paimon.moe (${diff.join('; ')}).`);
  }
  return { added, problems };
}

export function newFiveStars(previous, next) {
  const known = new Set(previous.map((i) => `${i.game}:${i.id}`));
  return next
    .filter((i) => i.kind === 'character' && i.rarity === 5 && !known.has(`${i.game}:${i.id}`))
    .map((i) => `${i.name} (${i.game})`);
}

const rowText = (row) =>
  `    {${['id', 'game', 'pool', 'name', 'start', 'end'].map((k) => `${JSON.stringify(k)}: ${JSON.stringify(row[k])}`).join(', ')}, "featured_5": [${row.featured_5.map((id) => JSON.stringify(id)).join(', ')}]},`;

// Adds the rows after the last Genshin character row, keeping the file's one-event-per-line layout.
export function insertRows(text, rows) {
  if (!rows.length) return text;
  const lines = text.split('\n');
  let last = -1;
  lines.forEach((line, i) => {
    if (line.includes('"id": "hk4e-character-')) last = i;
  });
  if (last < 0) throw new Error('No Genshin character rows to insert after.');
  lines.splice(last + 1, 0, ...rows.map(rowText));
  return lines.join('\n');
}

export function buildReport({ added, problems, newFives }) {
  const out = [];
  if (added.length) {
    out.push('### Drafted banner events', '', 'Added to `catalog/banners.json` from Paimon.moe. Each needs the second banner\'s title (the draft has only the first) and a check against the official notice before merging.', '');
    for (const row of added) out.push(`- ${row.name}: ${row.start} to ${row.end}`);
    out.push('');
  }
  if (problems.length) {
    out.push('### Needs a look', '', ...problems.map((p) => `- ${p}`), '');
  }
  if (newFives.length) {
    out.push('### New 5-star characters in the item catalog', '', 'Add a banner entry for each when its schedule is announced. Honkai: Star Rail and Zenless Zone Zero have no public schedule to track, so these lists are how new characters get noticed.', '', ...newFives.map((n) => `- ${n}`), '');
  }
  return out.length ? `${out.join('\n')}\n` : '';
}

async function loadText(url) {
  const response = await fetch(url, { headers: { 'user-agent': 'Wishtory catalog maintenance' } });
  if (!response.ok) throw new Error(`Upstream request failed (${response.status}): ${url}`);
  return response.text();
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const args = process.argv.slice(2);
  const option = (name) => (args.includes(name) ? args[args.indexOf(name) + 1] : null);
  const bannersPath = 'catalog/banners.json';
  const bannersText = await readFile(bannersPath, 'utf8');
  const catalog = JSON.parse(await readFile('catalog/catalog.json', 'utf8'));
  const previousPath = option('--previous');
  const previous = previousPath ? JSON.parse(await readFile(previousPath, 'utf8')).items : catalog.items;
  const upstream = parsePaimonCharacters(await loadText(PAIMON_BANNERS));
  const { added, problems } = trackBanners(upstream, JSON.parse(bannersText).banners, catalog.items);
  if (added.length) await writeFile(bannersPath, insertRows(bannersText, added));
  const report = buildReport({ added, problems, newFives: newFiveStars(previous, catalog.items) });
  const reportPath = option('--report');
  if (reportPath) await writeFile(reportPath, report);
  console.log(`${added.length} drafted, ${problems.length} to look at.`);
}
