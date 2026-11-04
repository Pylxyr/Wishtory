// Browser checks for Wishtory's interface. Serves the built frontend (../dist) with a mocked Tauri bridge
// (mock-tauri.js, synthetic data) and drives it in Chromium. See README.md. It never touches the Rust side.
const { chromium } = require('playwright');
const fs = require('fs');
const http = require('http');
const path = require('path');

const DIST = path.resolve(__dirname, '..', 'dist');
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.woff2': 'font/woff2', '.svg': 'image/svg+xml', '.json': 'application/json' };
function serve() {
  if (!fs.existsSync(path.join(DIST, 'index.html'))) { console.error('dist/ is missing: run "npm run build" in the repository root first.'); process.exit(2); }
  const mock = fs.readFileSync(path.join(__dirname, 'mock-tauri.js'));
  const server = http.createServer((req, res) => {
    const pathname = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (pathname === '/mock-tauri.js') { res.writeHead(200, { 'content-type': 'text/javascript' }); return res.end(mock); }
    const file = path.normalize(path.join(DIST, pathname === '/' ? 'index.html' : pathname));
    if (!file.startsWith(DIST) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) { res.writeHead(404); return res.end(); }
    let body = fs.readFileSync(file);
    if (path.basename(file) === 'index.html') body = Buffer.from(body.toString().replace('<script type="module"', '<script src="/mock-tauri.js"></script><script type="module"'));
    res.writeHead(200, { 'content-type': TYPES[path.extname(file)] ?? 'application/octet-stream' });
    res.end(body);
  });
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve({ url: `http://127.0.0.1:${server.address().port}/`, close: () => new Promise((r) => server.close(r)) })));
}

let BASE = '';
const results = [];
let browser;
async function open(query = '', { w = 1280, h = 800, scheme, theme = 'dark', reduced, tutorial = false, extra } = {}) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h }, colorScheme: scheme, reducedMotion: reduced });
  await ctx.addInitScript(([t, done]) => { if (t) localStorage.setItem('wishtory.theme', t); if (done) localStorage.setItem('wishtory.tutorialDone', '1'); }, [theme, !tutorial]);
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  await page.goto(BASE + query); await page.waitForSelector('.shell'); await page.waitForTimeout(600);
  page._errors = errors; page._ctx = ctx;
  return page;
}
async function check(name, fn) {
  if (process.env.ONLY && !name.includes(process.env.ONLY)) return;
  try { const detail = await fn(); results.push({ name, ok: true, detail: detail ?? '' }); }
  catch (e) { results.push({ name, ok: false, detail: String(e.message).split('\n')[0].slice(0, 300) }); }
}
const ok = (c, msg) => { if (!c) throw new Error(msg); };
const rail = (l) => `.rail-item[aria-label="${l}"]`;
const refreshBtn = '.pagebar-actions button.btn.primary';

(async () => {
  const { url, close } = await serve();
  BASE = url;
  browser = await chromium.launch();

  await check('UI-1 chip keeps its pill style in the dark theme', async () => {
    const p = await open();
    const c = await p.evaluate(() => { const s = getComputedStyle(document.querySelector('.chip.ok')); return [s.borderTopWidth, s.borderTopStyle, s.paddingLeft, s.borderTopLeftRadius]; });
    ok(c[0] === '1px' && c[1] === 'solid' && parseFloat(c[2]) >= 8 && parseFloat(c[3]) > 8, 'chip style ' + c);
    await p.click(rail('Accounts')); await p.waitForTimeout(250);
    const m = await p.evaluate(() => { const s = getComputedStyle(document.querySelector('.account-meta')); return s.color; });
    ok(m === 'rgb(156, 166, 181)', 'account-meta color ' + m);
    await p._ctx.close(); return `chip ${c.join(' ')}; account-meta ${m}`;
  });

  await check('UI-3 tooltip opens on keyboard focus, is described, hoverable, Esc closes', async () => {
    const p = await open();
    await p.focus(refreshBtn);
    const shown = await p.evaluate(() => { const t = document.querySelector('.tip'); return t && !t.hidden ? t.textContent : null; });
    ok(shown && shown.includes('read-only'), 'no tooltip on focus: ' + shown);
    const described = await p.evaluate((sel) => { const b = document.querySelector(sel); const id = b.getAttribute('aria-describedby'); return id && document.getElementById(id.split(' ').pop())?.textContent; }, refreshBtn);
    ok(described && described.includes('Reads the game'), 'aria-describedby text: ' + described);
    await p.keyboard.press('Escape'); await p.waitForTimeout(100);
    ok(await p.evaluate(() => document.querySelector('.tip').hidden), 'Esc did not close the tooltip');
    // hoverable: pointer moves from the trigger onto the bubble
    await p.hover(refreshBtn); await p.waitForTimeout(150);
    const box = await p.locator('.tip').boundingBox();
    await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 6 }); await p.waitForTimeout(400);
    ok(await p.evaluate(() => !document.querySelector('.tip').hidden), 'bubble closed while hovered');
    await p.mouse.move(5, 400); await p.waitForTimeout(400);
    ok(await p.evaluate(() => document.querySelector('.tip').hidden), 'bubble stayed after the pointer left');
    // the Read-only chip is reachable by keyboard and explains itself
    await p.focus('.rail-foot .chip'); const r = await p.evaluate(() => { const t = document.querySelector('.tip'); const c = document.querySelector('.rail-foot .chip').getBoundingClientRect(); return { shown: !t.hidden, left: parseFloat(t.style.left), chipRight: c.right }; });
    ok(r.shown && r.left >= r.chipRight, 'rail chip tooltip ' + JSON.stringify(r));
    await p._ctx.close(); return 'focus, describedby, Esc, hover bubble, rail placement';
  });

  await check('UI-8 error toasts stay until dismissed and have a close button', async () => {
    const p = await open('?s=toasterr');
    await p.click(rail('Settings')); await p.waitForTimeout(200);
    await p.click('button[role="switch"]'); await p.waitForTimeout(400);
    ok(await p.locator('.toast.err').count() === 1, 'no error toast');
    await p.waitForTimeout(9500);
    ok(await p.locator('.toast.err').count() === 1, 'error toast timed out');
    ok(await p.evaluate(() => document.querySelector('.toast.err').getAttribute('role')) === 'alert', 'role');
    ok(await p.evaluate(() => document.querySelectorAll('.toasts [aria-live]').length) === 0, 'container has a second live region');
    await p.click('.toast-close'); await p.waitForTimeout(500);
    ok(await p.locator('.toast.err').count() === 0, 'close did not dismiss');
    await p._ctx.close(); return 'persisted 9.5 s, dismissed by the close button';
  });

  await check('UI-8 timed toasts pause on hover (also with reduced motion)', async () => {
    const p = await open('', { reduced: 'reduce' });
    await p.click(rail('Settings')); await p.waitForTimeout(200);
    await p.locator('.setting-row', { hasText: 'App updates' }).getByRole('button', { name: 'Check for updates' }).click(); await p.waitForTimeout(500);
    ok(await p.locator('.toast.ok').count() === 1, 'no toast');
    await p.hover('.toast.ok'); await p.waitForTimeout(7500);
    ok(await p.locator('.toast.ok').count() === 1, 'toast timed out while hovered');
    await p.mouse.move(5, 5); await p.waitForTimeout(7000);
    ok(await p.locator('.toast.ok').count() === 0, 'toast never timed out after the pointer left');
    await p._ctx.close(); return 'held 7.5 s while hovered, gone after leaving';
  });

  await check('UI-9 theme follows the system until chosen', async () => {
    let p = await open('', { scheme: 'light', theme: null });
    ok(await p.evaluate(() => document.documentElement.dataset.theme) === 'light', 'light OS → light');
    await p.emulateMedia({ colorScheme: 'dark' }); await p.waitForTimeout(200);
    ok(await p.evaluate(() => document.documentElement.dataset.theme) === 'dark', 'runtime switch to dark');
    await p.click(rail('Settings')); await p.waitForTimeout(200);
    ok(await p.locator('.seg button[aria-pressed="true"]', { hasText: 'System' }).count() === 1, 'System not selected');
    await p.click('.seg button:has-text("Light")'); await p.waitForTimeout(150);
    await p.emulateMedia({ colorScheme: 'dark' });
    ok(await p.evaluate(() => document.documentElement.dataset.theme) === 'light', 'explicit Light ignored');
    await p._ctx.close();
    p = await open('', { scheme: 'light', theme: 'dark' });
    ok(await p.evaluate(() => document.documentElement.dataset.theme) === 'dark', 'saved dark must win over a light OS');
    await p._ctx.close(); return 'system, runtime change, explicit choice, saved choice';
  });

  await check('UI-11 remembers page, game and account across a reload', async () => {
    const p = await open();
    await p.click(rail('History')); await p.click('.rail-item[aria-label^="Honkai"]'); await p.waitForTimeout(400);
    await p.reload(); await p.waitForSelector('.shell'); await p.waitForTimeout(700);
    const s = await p.evaluate(() => ({ page: document.querySelector('.rail-item[aria-current="page"]')?.getAttribute('aria-label'), game: document.querySelector('.pagebar-game')?.textContent, uid: document.querySelector('#account-select')?.value }));
    ok(s.page === 'History' && s.game === 'Honkai: Star Rail' && s.uid === '700111222', JSON.stringify(s));
    await p._ctx.close(); return JSON.stringify(s);
  });

  await check('UI-2 expired link → recovery card → Help deep link → Try again', async () => {
    const p = await open('?s=expired-once');
    await p.click(refreshBtn); await p.waitForTimeout(900);
    const c = await p.evaluate(() => ({ title: document.querySelector('.recovery h2')?.textContent, role: document.querySelector('.recovery')?.getAttribute('role'), steps: [...document.querySelectorAll('.recovery-steps li')].map((l) => l.textContent), bar: document.querySelector('.statusbar .status-text')?.textContent, toasts: document.querySelectorAll('.toast').length }));
    ok(c.title === 'The history link has expired' && c.role === 'alert', JSON.stringify(c));
    ok(c.steps[0].includes('Genshin Impact') && c.steps[0].includes('Wish → History'), c.steps[0]);
    ok(c.bar.startsWith('Refresh stopped'), c.bar);
    ok(c.toasts === 0, 'error is duplicated as a toast');
    await p.click('text=Why does this happen?'); await p.waitForTimeout(500);
    const h = await p.evaluate(() => ({ open: document.getElementById('help-expired')?.open, focus: document.activeElement?.tagName, inSummary: document.getElementById('help-expired')?.contains(document.activeElement) }));
    ok(h.open && h.inSummary, JSON.stringify(h));
    await p.click(rail('Overview')); await p.waitForTimeout(400);
    ok(await p.locator('.recovery').count() === 1, 'card gone after navigating');
    await p.click('.rail-item[aria-label^="Honkai"]'); await p.waitForTimeout(300);
    ok(await p.locator('.recovery').count() === 0, 'card shown for a different game');
    await p.click('.rail-item[aria-label^="Genshin"]'); await p.waitForTimeout(300);
    await p.click('.recovery .btn.primary'); await p.waitForTimeout(900);
    ok(await p.locator('.recovery').count() === 0, 'card still there after a successful retry');
    const bar = await p.textContent('.statusbar .status-text');
    ok(/^Downloaded 40 pulls/.test(bar), bar);
    ok(await p.locator('.pagebar .updated').count() === 1, 'no "Updated" label');
    await p._ctx.close(); return `title ok; steps game-specific; help fold opened; retry succeeded (“${bar.slice(0, 40)}…”)`;
  });

  await check('UI-2 rate limit disables retry with a countdown', async () => {
    const p = await open('?s=rate');
    await p.click(refreshBtn); await p.waitForTimeout(900);
    const t1 = await p.textContent('.recovery .btn.primary');
    ok(/^Try again in \d+ s$/.test(t1.trim()) && await p.locator('.recovery .btn.primary').isDisabled(), t1);
    await p.waitForTimeout(2200);
    const t2 = await p.textContent('.recovery .btn.primary');
    ok(parseInt(t2.match(/\d+/)[0]) < parseInt(t1.match(/\d+/)[0]), `${t1} → ${t2}`);
    await p._ctx.close(); return `${t1.trim()} → ${t2.trim()}`;
  });

  await check('UI-2 network error classified', async () => {
    const p = await open('?s=network'); await p.click(refreshBtn); await p.waitForTimeout(900);
    const t = await p.textContent('.recovery h2'); ok(t === 'Could not reach the server', t);
    await p._ctx.close(); return t;
  });

  await check('Overview load error shows an inline notice with retry (no misleading zeros)', async () => {
    const p = await open('?s=pityerr');
    const t = await p.textContent('.notice.err'); ok(t.includes('Could not load the pity numbers') && await p.locator('.notice .btn').count() === 1, t);
    ok(await p.locator('.facts').count() === 0, 'zero facts shown despite the error');
    await p._ctx.close(); return 'notice with Try again';
  });

  await check('UI-6 live banners, worst-case plan and per-game wording', async () => {
    const p = await open();
    const live = await p.$$eval('.live-item', (els) => els.map((e) => e.textContent.replace(/\s+/g, ' ').trim()));
    ok(live.length === 2 && live.every((t) => /\d+ d \d+ h left/.test(t)), JSON.stringify(live));
    const plan = await p.textContent('.plan');
    ok(/Worst case \d+ more pulls \(.* primogems\) to a guaranteed featured 5★/.test(plan.replace(/\s+/g, ' ')), plan);
    ok(await p.textContent('.pity-now .sr') === '17 pulls toward a five-star, out of 90', 'pity sr text');
    await p.click('.rail-item[aria-label^="Zenless"]'); await p.waitForTimeout(500);
    const text = await p.innerText('main');
    ok(!/five-star|5★/i.test(text) && text.includes('S-rank'), 'Zenless still says five-star / 5★: ' + (text.match(/.{20}(five-star|5★).{20}/i) || [''])[0]);
    await p._ctx.close(); return `${live.length} live banners; “${plan.replace(/\s+/g, ' ').slice(0, 70)}…”; Zenless uses S-rank`;
  });

  await check('UI-5 History: 50,000 pulls, windowed rows, sort, filter, view menu, CSV', async () => {
    const p = await open('?s=big');
    const t0 = Date.now();
    await p.click(rail('History')); await p.waitForSelector('table.history tbody tr:not(.pad)'); const firstPaint = Date.now() - t0;
    await p.waitForTimeout(300);
    const count = await p.textContent('.toolbar .count'); ok(count.trim() === '50,000 pulls', count);
    const rowsDom = () => p.locator('table.history tbody tr:not(.pad)').count();
    const r0 = await rowsDom(); ok(r0 > 15 && r0 < 90, 'rows in DOM: ' + r0);
    // scroll to the bottom in steps and time it
    const scrollMs = await p.evaluate(async () => { const s = document.querySelector('.history-scroll'); const t = performance.now(); const steps = Math.ceil(s.scrollHeight / 20000) + 2; for (let i = 0; i < steps; i++) { s.scrollTop += 20000; await new Promise((r) => requestAnimationFrame(r)); } return (performance.now() - t) / steps; });
    await p.waitForTimeout(150);
    const r1 = await rowsDom(); ok(r1 < 90, 'rows in DOM after scrolling: ' + r1);
    const atEnd = await p.evaluate(() => { const s = document.querySelector('.history-scroll'); const rows = [...document.querySelectorAll('table.history tbody tr:not(.pad)')]; return { bottom: s.scrollTop + s.clientHeight >= s.scrollHeight - 2, lastIdx: rows.at(-1).getAttribute('aria-rowindex'), th: document.querySelector('table.history th').getBoundingClientRect().top - s.getBoundingClientRect().top }; });
    ok(atEnd.bottom && atEnd.lastIdx === '50001', JSON.stringify(atEnd)); ok(Math.abs(atEnd.th) < 2, 'header is not sticky: ' + atEnd.th);
    // sort by rarity: first row is a 5-star after sorting descending
    await p.click('.th-sort:has-text("Rarity")'); await p.waitForTimeout(300);
    ok(await p.evaluate(() => document.querySelector('th[aria-sort]')?.textContent.includes('Rarity')), 'aria-sort missing');
    ok((await p.textContent('table.history tbody tr:not(.pad) .rank .sr')).includes('5★'), 'rarity sort');
    await p.click('.th-sort:has-text("Item")'); await p.waitForTimeout(300);
    const names = await p.$$eval('table.history tbody tr:not(.pad) td.item', (e) => e.slice(0, 6).map((x) => x.textContent));
    ok(names.every((n, i) => i === 0 || names[i - 1].localeCompare(n) <= 0), 'item sort ' + names);
    // filters
    await p.click('.seg button:has-text("5★")'); await p.waitForTimeout(300);
    const c5 = parseInt((await p.textContent('.toolbar .count')).match(/[\d,]+/)[0].replace(/,/g, '')); ok(c5 > 0 && c5 < 50000, 'filter count ' + c5);
    await p.click('text=Clear filters'); await p.waitForTimeout(250);
    ok((await p.textContent('.toolbar .count')).trim() === '50,000 pulls', 'clear filters');
    // "/" focuses search
    await p.mouse.click(5, 5); await p.keyboard.press('/'); ok(await p.evaluate(() => document.activeElement?.type === 'search'), '/ did not focus search');
    // view menu
    const ths = () => p.locator('table.history thead th').count(); const th0 = await ths();
    await p.click('.menu > button'); ok(await p.getAttribute('.menu > button', 'aria-expanded') === 'true', 'menu closed');
    await p.uncheck('.menu-pop label:has-text("Pool") input'); await p.waitForTimeout(150);
    ok(await ths() === th0 - 1, 'column not removed');
    await p.check('.menu-pop label:has-text("Comfortable") input'); await p.waitForTimeout(150);
    const rh = await p.evaluate(() => document.querySelector('table.history tbody tr:not(.pad)').getBoundingClientRect().height); ok(Math.abs(rh - 42) < 1.5, 'row height ' + rh);
    await p.keyboard.press('Escape'); await p.waitForTimeout(100);
    ok(await p.evaluate(() => !document.querySelector('#history-view') && document.activeElement?.textContent.includes('View')), 'Esc did not close the menu / return focus');
    const e0 = p._errors.length;
    await p._ctx.close(); return `first rows in ${firstPaint} ms; ${r0}/${r1} DOM rows of 50,000; mean scroll step ${scrollMs.toFixed(1)} ms; console errors ${e0}`;
  });

  await check('UI-5 History: time, rarity and pity cells are not clipped', async () => {
    const out = [];
    for (const [w, h] of [[1280, 800], [960, 640]]) {
      const p = await open('', { w, h });
      await p.click(rail('History')); await p.waitForSelector('table.history tbody tr:not(.pad)'); await p.waitForTimeout(300);
      const clipped = await p.evaluate(() => [...document.querySelectorAll('table.history tbody tr:not(.pad) td:nth-child(1), table.history tbody tr:not(.pad) td:nth-child(2)')].slice(0, 80).filter((td) => td.scrollWidth > td.clientWidth).map((td) => td.textContent));
      ok(clipped.length === 0, `${w}x${h}: clipped ${clipped.slice(0, 3)}`);
      const rank = await p.evaluate(() => [...document.querySelectorAll('table.history tbody tr:not(.pad) td.pity, table.history tbody tr:not(.pad) .rank')].filter((el) => el.scrollWidth > el.clientWidth).length); ok(rank === 0, 'rarity/pity clipped');
      await p._ctx.close(); out.push(`${w}x${h}`);
    }
    return 'timestamps and short cells fully visible at ' + out.join(', ');
  });

  await check('UI-5 Copy CSV is escaped and matches the view', async () => {
    const p = await open();
    await p.click(rail('History'));
    await p.selectOption('#account-select', '600987654'); await p.waitForTimeout(500);
    await p.evaluate(() => { window.__csv = ''; Object.defineProperty(navigator, 'clipboard', { value: { writeText: async (t) => { window.__csv = t; } }, configurable: true }); });
    await p.click('button:has-text("Copy CSV")'); await p.waitForTimeout(300);
    const csv = await p.evaluate(() => window.__csv); const lines = csv.split('\n');
    const n = parseInt((await p.textContent('.toolbar .count')).match(/[\d,]+/)[0].replace(/,/g, ''));
    ok(lines[0] === 'Time,Item,Type,Rarity,Pity,Pool,Banner', lines[0]); ok(lines.length === n + 1, `${lines.length} lines for ${n} rows`);
    ok(csv.includes(`"'=1+1, ""test"""`), 'formula-looking name not neutralised');
    ok(await p.locator('.toast.ok').count() === 1, 'no confirmation toast');
    await p._ctx.close(); return `${n} rows, header ok, “=1+1” neutralised`;
  });

  await check('UI-12 Accounts: label, one confirm timer, announcements', async () => {
    const p = await open();
    await p.click(rail('Accounts')); await p.waitForTimeout(300);
    await p.locator('.account').nth(0).getByRole('button', { name: /Rename/ }).click();
    await p.fill('.label-edit input', 'Main'); await p.keyboard.press('Enter'); await p.waitForTimeout(200);
    ok((await p.locator('.account').nth(0).textContent()).includes('UID 800123456, Main'), 'label not shown');
    await p.click(rail('Overview')); const opt = await p.textContent('#account-select option:checked'); ok(opt.includes('· Main'), opt);
    await p.click(rail('Accounts')); await p.waitForTimeout(250);
    const remove = (i) => p.locator('.account').nth(i).getByRole('button', { name: /Remove|confirm/ });
    await remove(0).click(); await p.waitForTimeout(2000); await remove(1).click(); await p.waitForTimeout(3300);
    ok((await remove(1).textContent()).includes('confirm'), 'second confirm was cancelled by the first timer');
    ok(!(await remove(0).textContent()).includes('confirm'), 'first still confirming');
    ok((await p.textContent('main [role=status]')).includes('Remove again'), 'confirm state not announced');
    await p.waitForTimeout(2200); ok(!(await remove(1).textContent()).includes('confirm'), 'confirm never expired');
    await p._ctx.close(); return 'label saved, selector shows it, second confirm survived 3.3 s, announced';
  });

  await check('Keyboard: skip link moves focus into the page', async () => {
    const p = await open(); await p.keyboard.press('Tab');
    ok(await p.evaluate(() => document.activeElement?.classList.contains('skip')), 'first Tab is not the skip link');
    await p.keyboard.press('Enter'); ok(await p.evaluate(() => document.activeElement?.id === 'main'), 'focus did not reach main');
    await p._ctx.close(); return 'Tab → skip link → Enter → main';
  });

  await check('First-run tour focuses Next, not Skip, and Esc closes it', async () => {
    const p = await open('?s=empty', { tutorial: true });
    const f = await p.evaluate(() => document.activeElement?.textContent.trim()); ok(f === 'Next', 'focus on ' + f);
    ok(!(await p.innerText('.modal')).includes('? icon'), 'tour still points at the old ? icon');
    await p.keyboard.press('Escape'); await p.waitForTimeout(400);
    ok(await p.locator('.overlay').count() === 0, 'Esc did not close'); await p._ctx.close(); return 'focus on Next; Esc closes';
  });

  await check('Rail: expanded at 1280, icons only at 1024, every item has a name', async () => {
    let p = await open();
    const w1 = await p.evaluate(() => document.querySelector('.rail').getBoundingClientRect().width); ok(w1 === 224, 'rail width ' + w1);
    const unnamed = await p.$$eval('.rail-item, .rail-foot .chip', (els) => els.filter((e) => !(e.getAttribute('aria-label') || '').trim()).length); ok(unnamed === 0, unnamed + ' unnamed');
    await p._ctx.close(); p = await open('', { w: 1024, h: 700 });
    const w2 = await p.evaluate(() => document.querySelector('.rail').getBoundingClientRect().width); ok(w2 === 64, 'rail width ' + w2);
    ok(await p.evaluate(() => getComputedStyle(document.querySelector('.rail-text')).display) === 'none', 'labels still shown');
    await p._ctx.close(); return '224 px → 64 px';
  });

  await check('Layout: status bar stays pinned, only the page scrolls, content has padding', async () => {
    const out = [];
    for (const [w, h] of [[1280, 800], [960, 640], [800, 600]]) {
      const p = await open('', { w, h });
      for (const label of ['Overview', 'History', 'Accounts', 'Settings', 'Help']) {
        await p.click(rail(label)); await p.waitForTimeout(250);
        const m = await p.evaluate(() => {
          const sb = document.querySelector('.statusbar').getBoundingClientRect(); const main = document.querySelector('main'); const cs = getComputedStyle(main);
          const page = document.querySelector('main .page').getBoundingClientRect(); const rail = document.querySelector('.rail').getBoundingClientRect();
          return { sbBottom: Math.round(sb.bottom), vh: innerHeight, docScroll: document.documentElement.scrollHeight - innerHeight, overflowY: cs.overflowY, padL: parseFloat(cs.paddingLeft), gap: Math.round(page.left - rail.right) };
        });
        ok(m.sbBottom === m.vh && m.docScroll <= 0 && m.overflowY === 'auto' && m.padL >= 16 && m.gap >= 16, `${label} ${w}x${h}: ${JSON.stringify(m)}`);
      }
      await p._ctx.close(); out.push(`${w}x${h}`);
    }
    return 'status bar bottom = viewport bottom, page scrolls inside main, padding ≥ 16 px at ' + out.join(', ');
  });

  await check('Forced colors and empty state render without errors', async () => {
    const p = await open(); await p.emulateMedia({ forcedColors: 'active' }); await p.waitForTimeout(300);
    if (process.env.SCREENSHOTS) await p.screenshot({ path: `${process.env.SCREENSHOTS}/forced-colors-overview.png` });
    await p._ctx.close();
    const e = await open('?s=empty'); const text = await e.innerText('main'); ok(text.includes('Click Refresh at the top of the page'), 'empty state text');
    if (process.env.SCREENSHOTS) await e.screenshot({ path: `${process.env.SCREENSHOTS}/empty-overview.png` });
    ok(e._errors.length === 0, 'console errors: ' + e._errors.join(' | ')); await e._ctx.close(); return 'screenshots saved';
  });

  await check('Linux: Game locations is hidden on Windows and works on Linux (detected prefixes, add, remove, warning)', async () => {
    let p = await open('');
    await p.click(rail('Settings')); await p.waitForTimeout(250);
    ok(await p.locator('text=Game locations').count() === 0, 'Game locations shown where it is not supported');
    await p._ctx.close();

    p = await open('?s=linux');
    await p.click(rail('Settings')); await p.waitForTimeout(300);
    const detected = await p.$$eval('.locations .loc-list li', (els) => els.map((e) => e.textContent.replace(/\s+/g, ' ').trim()));
    ok(detected.length === 3 && detected[0].includes('anime-game-launcher/prefix') && detected[0].includes('Genshin') && detected[2].includes('no game log yet'), JSON.stringify(detected));
    ok(await p.locator('text=Folders you added').count() === 0, 'empty list is shown');
    await p.click('button:has-text("Add folder")'); await p.waitForTimeout(500);
    ok(await p.locator('.toast.ok', { hasText: 'Added /home/me/Games.' }).count() === 1, 'no success toast');
    const after = await p.$$eval('.locations .loc-list li', (els) => els.map((e) => e.textContent.replace(/\s+/g, ' ').trim()));
    ok(after.some((t) => t.startsWith('/home/me/Games') && t.includes('Remove')) && after.some((t) => t.includes('/home/me/Games/zzz') && t.includes('Zenless')), JSON.stringify(after));
    ok(await p.locator('button[aria-label="Remove /home/me/Games"]').count() === 1, 'remove button is not labelled with its folder');
    await p.click('button[aria-label="Remove /home/me/Games"]'); await p.waitForTimeout(400);
    ok(await p.locator('text=Folders you added').count() === 0, 'folder was not removed');
    await p._ctx.close();

    p = await open('?s=linux-empty');
    await p.click(rail('Settings')); await p.waitForTimeout(250);
    await p.click('button:has-text("Add folder")'); await p.waitForTimeout(500);
    ok(await p.locator('.toast.warn', { hasText: 'no Wine prefix was found' }).count() === 1, 'no warning for a folder without a prefix');
    await p._ctx.close(); return '3 detected prefixes with game chips; add → toast + new prefix; remove; warning for an empty folder';
  });

  await check('Linux: "game not found" points at Game locations; Windows does not', async () => {
    let p = await open('?s=linux-nogame');
    await p.click(refreshBtn); await p.waitForTimeout(900);
    const card = await p.evaluate(() => ({ title: document.querySelector('.recovery h2')?.textContent, steps: [...document.querySelectorAll('.recovery-steps li')].map((l) => l.textContent) }));
    ok(card.title === 'Genshin Impact was not detected' && card.steps.some((t) => t.includes('Settings → Game locations')), JSON.stringify(card));
    await p.click('.recovery .btn:has-text("Game locations")'); await p.waitForTimeout(400);
    ok(await p.evaluate(() => document.querySelector('.rail-item[aria-current="page"]')?.getAttribute('aria-label')) === 'Settings', 'did not open Settings');
    ok(await p.locator('#locations-title').count() === 1, 'Settings has no Game locations section');
    await p._ctx.close();
    p = await open('?s=nogame');
    await p.click(refreshBtn); await p.waitForTimeout(900);
    ok(await p.locator('.recovery h2', { hasText: 'was not detected' }).count() === 1, 'no recovery card');
    ok(await p.locator('.recovery .btn:has-text("Game locations")').count() === 0, 'Windows card offers Game locations');
    ok(!(await p.innerText('.recovery')).includes('Wine'), 'Windows card mentions Wine');
    await p._ctx.close(); return 'Linux card has the Settings step and button; Windows card does not';
  });

  await check('Linux: Settings and Help pass axe with the new sections', async () => {
    const axeSrc = fs.readFileSync(require.resolve('axe-core/axe.min.js'), 'utf8');
    for (const theme of ['dark', 'light']) {
      const p = await open('?s=linux', { theme });
      for (const label of ['Settings', 'Help']) {
        await p.click(rail(label)); await p.waitForTimeout(300);
        await p.evaluate(axeSrc);
        const v = await p.evaluate(async () => (await axe.run(document, { runOnly: ['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa', 'best-practice'] })).violations.map((x) => `${x.id} (${x.nodes.length}): ${x.nodes[0].target.join(' ')}`));
        ok(v.length === 0, `${theme} ${label}: ${v.join('; ')}`);
      }
      ok(await p.locator('#help-linux').count() === 1, 'Help has no Linux card');
      await p._ctx.close();
    }
    return 'no violations on Settings and Help, both themes';
  });

  await check('Accessibility: axe finds nothing on any page in either theme (WCAG 2.2 AA + best practice)', async () => {
    const axeSrc = fs.readFileSync(require.resolve('axe-core/axe.min.js'), 'utf8');
    const labels = ['Overview', 'History', 'Import / Export', 'Accounts', 'Settings', 'Help', 'About', 'Logging'];
    let runs = 0;
    for (const theme of ['dark', 'light']) {
      const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
      await ctx.addInitScript((t) => { localStorage.setItem('wishtory.theme', t); localStorage.setItem('wishtory.tutorialDone', '1'); localStorage.setItem('wishtory.logging', '1'); }, theme);
      const page = await ctx.newPage(); await page.goto(BASE); await page.waitForSelector('.shell'); await page.waitForTimeout(600);
      for (const label of labels) {
        await page.click(rail(label)); await page.waitForTimeout(250);
        if (process.env.SCREENSHOTS) await page.screenshot({ path: `${process.env.SCREENSHOTS}/${theme}-${label.replace(/\W+/g, '')}.png` });
        await page.evaluate(axeSrc);
        const v = await page.evaluate(async () => (await axe.run(document, { runOnly: ['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa', 'best-practice'] })).violations.map((x) => `${x.id} (${x.nodes.length}): ${x.nodes[0].target.join(' ')}`));
        ok(v.length === 0, `${theme} ${label}: ${v.join('; ')}`);
        runs++;
      }
      await ctx.close();
    }
    return `${runs} page/theme runs, 0 violations`;
  });

  await browser.close();
  await close();
  let failed = 0;
  for (const r of results) { if (!r.ok) failed++; console.log(`${r.ok ? 'PASS' : 'FAIL'}  ${r.name}\n      ${r.detail}`); }
  console.log(`\n${results.length - failed}/${results.length} checks passed`);
  process.exit(failed ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(1); });
