// Opens the real Svelte window on the mock backend and checks the things that have broken before. Unlike
// `svelte-check` and the build, this renders the page, so it catches layout and runtime failures.
// Usage: npm run e2e  (needs `npx playwright install chromium` once; see e2e/harness.mjs for env options)
import { openBrowser, sleep, startApp } from './harness.mjs';

const GAMES = [['genshin', 'Genshin'], ['star_rail', 'Star Rail'], ['zenless', 'Zenless Zone Zero']];
const results = [];
async function check(name, fn) {
  try { const detail = await fn(); results.push([true, name, detail]); }
  catch (e) { results.push([false, name, e.message]); }
}
const must = (cond, msg) => { if (!cond) throw new Error(msg); };
const calls = (page) => page.evaluate(() => { const c = window.__wishtoryCalls ?? []; window.__wishtoryCalls = []; return c; });
const goGame = async (page, label) => { await page.getByRole('button', { name: new RegExp(label) }).first().click(); await sleep(1400); };

const app = await startApp();
try {
  // 1. Nothing is cut off, at the smallest and the default window size, on every game and on Settings.
  for (const [w, h] of [[980, 640], [1240, 770]]) {
    const { browser, page, errors } = await openBrowser({ width: w, height: h });
    await page.goto(app.url); await page.waitForSelector('.shell'); await sleep(1200);
    for (const [, label] of GAMES) {
      await check(`${w}x${h} ${label}: content scrolls inside the window`, async () => {
        await goGame(page, label);
        const m = await page.evaluate(() => { const c = document.querySelector('.content'); return { main: document.querySelector('.main').getBoundingClientRect().height, win: innerHeight, bar: document.querySelector('.statusbar').getBoundingClientRect().bottom, can: c.scrollHeight > c.clientHeight }; });
        must(m.main <= m.win + 1, `.main is ${Math.round(m.main)}px in a ${m.win}px window`);
        must(m.bar <= m.win + 1, `the status bar is off screen (bottom ${Math.round(m.bar)}px)`);
        return m.can ? 'scrolls' : 'fits';
      });
    }
    await check(`${w}x${h} Settings: reachable`, async () => {
      await page.getByRole('button', { name: /Settings/ }).click(); await sleep(600);
      await page.mouse.move(600, 300); await page.mouse.wheel(0, 4000); await sleep(300);
      const r = await page.getByText('Diagnostics').first().boundingBox();
      const win = await page.evaluate(() => innerHeight);
      must(r && r.y >= 0 && r.y + r.height <= win, `the last Settings section is at y=${r ? Math.round(r.y) : 'missing'} in a ${win}px window`);
    });
    await check(`${w}x${h}: no page errors`, async () => { must(errors.length === 0, errors[0]); });
    await browser.close();
  }

  // 2. A 10-pull can return the same 4★ twice in the same second; that must not blank the Overview.
  {
    const { browser, page, errors } = await openBrowser({ init: () => {
      window.__wishtoryMock = { get_pity: async (_a, real) => { const out = await real(); for (const b of out) { const f = b.recent_fours; if (f?.length) f.push({ ...f.at(-1), pity: 3 }); } return out; } };
    } });
    await page.goto(app.url); await page.waitForSelector('.shell'); await sleep(2000);
    await check('same-second duplicate 4★ still renders the Overview', async () => {
      must((await page.locator('.content').innerText()).length > 100, 'the Overview is blank');
      must(errors.length === 0, errors[0]);
    });
    await browser.close();
  }

  // 3. The main flows on the default data.
  {
    const { browser, page, errors } = await openBrowser({ width: 1240, height: 770 });
    await page.goto(app.url); await page.waitForSelector('.shell'); await sleep(2000);

    await check('every checkbox has an accessible name', async () => {
      const bad = [];
      for (const name of ['Settings', 'Import', 'Accounts']) {
        await page.getByRole('button', { name: new RegExp(name) }).click(); await sleep(500);
        bad.push(...await page.$$eval('input[type=checkbox]', (els) => els.filter((e) => !e.labels?.length && !e.getAttribute('aria-label') && !e.getAttribute('aria-labelledby')).map((e) => e.outerHTML.slice(0, 80))));
      }
      must(bad.length === 0, `unlabeled: ${bad[0]}`);
    });

    await check('History asks the backend only for the ranks it shows', async () => {
      await goGame(page, 'Genshin'); await calls(page);
      await page.getByRole('tab', { name: 'History' }).click(); await sleep(800);
      const first = (await calls(page)).find((c) => c.cmd === 'get_history');
      must(first?.args.minRank === 4, `minRank was ${first?.args.minRank}`);
      const few = await page.locator('tbody tr').count();
      await page.getByRole('button', { name: 'All pulls' }).click(); await sleep(700);
      const all = await page.locator('tbody tr').count();
      must(all > few, `All pulls listed ${all} rows, 4★+ listed ${few}`);
      return `${few} → ${all} rows`;
    });

    await check('shortcuts: Alt+2 opens Star Rail, Ctrl+, opens Settings', async () => {
      await page.keyboard.press('Alt+2'); await sleep(700);
      must((await page.locator('h1').first().innerText()).toUpperCase().includes('STAR RAIL'), 'Alt+2 did nothing');
      await page.keyboard.press('Control+,'); await sleep(500);
      must((await page.locator('h1').first().innerText()).toUpperCase().includes('SETTINGS'), 'Ctrl+, did nothing');
    });

    for (const [id, label] of GAMES.slice(1)) {
      await goGame(page, label);
      await check(`${label}: no infinite animation drives layout or paint`, async () => {
        const { bad, inspected } = await page.evaluate(() => {
          const deny = /^(left|right|top|bottom|width|height|margin.*|padding.*|backgroundPosition|boxShadow|borderWidth)$/;
          const found = new Set();
          let inspected = 0;
          for (const a of document.getAnimations()) {
            if (a.effect?.getTiming().iterations !== Infinity) continue;
            inspected++;
            for (const k of a.effect.getKeyframes()) for (const p of Object.keys(k)) if (deny.test(p)) found.add(`${a.animationName}:${p}`);
          }
          return { bad: [...found], inspected };
        });
        must(inspected >= 10, `only ${inspected} animations were running, so nothing was checked`);
        must(bad.length === 0, bad.join(', '));
      });
      await check(`${label}: keyboard focus is visible`, async () => {
        await page.evaluate(() => document.documentElement.setAttribute('data-motion', 'off'));
        const btn = page.locator('.btn.primary').first();
        const box = await btn.boundingBox();
        const clip = { x: box.x - 8, y: box.y - 8, width: box.width + 16, height: box.height + 16 };
        await page.mouse.move(2, 2); await page.keyboard.press('Shift'); await btn.evaluate((e) => e.blur()); await sleep(250);
        const before = await page.screenshot({ clip });
        await page.keyboard.press('Tab'); await btn.focus(); await sleep(250);
        const after = await page.screenshot({ clip });
        must(!before.equals(after), 'focusing the button changes nothing on screen');
        await page.evaluate(() => document.documentElement.setAttribute('data-motion', 'on'));
      });
    }
    for (const [, label] of GAMES) {
      await check(`${label}: History timeline stays inside its panel`, async () => {
        await goGame(page, label);
        await page.getByRole('tab', { name: 'History' }).click(); await sleep(900);
        const m = await page.evaluate(() => {
          const box = document.querySelector('.timeline')?.getBoundingClientRect();
          if (!box) return null;
          const dots = [...document.querySelectorAll('.timeline .dot')].map((d) => d.getBoundingClientRect());
          const track = document.querySelector('.timeline .track').getBoundingClientRect();
          const ends = document.querySelector('.timeline .ends').getBoundingClientRect();
          return { dots: dots.length, outside: dots.filter((d) => d.left < box.left || d.right > box.right || d.top < box.top || d.bottom > box.bottom).length, labelsBelowTrack: ends.top > track.bottom, labelsInside: ends.right <= box.right + 1 && ends.bottom <= box.bottom };
        });
        must(m, 'no timeline is shown');
        must(m.outside === 0, `${m.outside} of ${m.dots} dots lie outside the panel`);
        must(m.labelsBelowTrack && m.labelsInside, 'the date labels are not under the track inside the panel');
        await page.getByRole('tab', { name: 'Overview' }).click(); await sleep(300);
      });
    }

    await check('Ctrl+R pressed three times starts one refresh', async () => {
      await goGame(page, 'Genshin'); await calls(page);
      for (let i = 0; i < 3; i++) await page.keyboard.press('Control+r');
      await sleep(800);
      const n = (await calls(page)).filter((c) => c.cmd === 'refresh_from_game').length;
      must(n === 1, `${n} refreshes started`);
      await sleep(3500);
    });
    await check('main flows raised no page errors', async () => { must(errors.length === 0, errors[0]); });
    await browser.close();
  }

  // 4. Clicking Refresh again while one is running must not start another.
  {
    const { browser, page } = await openBrowser({ init: () => { window.__wishtoryMock = { list_accounts: async () => [] }; } });
    await page.goto(app.url); await page.waitForSelector('.shell'); await sleep(2000); await calls(page);
    await check('three quick clicks on Refresh start one refresh', async () => {
      const btn = page.locator('.content').getByRole('button', { name: /Refresh/ }).first();
      await btn.click(); await btn.click({ force: true }).catch(() => {}); await btn.click({ force: true }).catch(() => {});
      await sleep(3500);
      const n = (await calls(page)).filter((c) => c.cmd === 'refresh_from_game').length;
      must(n === 1, `${n} refreshes started`);
    });
    await browser.close();
  }
  // 5. Features whose backend existed long before they had a screen.
  {
    const { browser, page, errors } = await openBrowser({ width: 1240, height: 900 });
    await page.goto(app.url); await page.waitForSelector('.shell'); await sleep(1800);
    await page.getByRole('button', { name: /Settings/ }).click(); await sleep(600);

    await check('Cloud backup: passphrase is checked, then connect, back up, restore, disconnect', async () => {
      const pass = page.getByLabel('Backup passphrase', { exact: true });
      const again = page.getByLabel('Backup passphrase again');
      const signIn = page.getByRole('button', { name: 'Sign in with Google' });
      await pass.fill('short'); await sleep(150);
      must(await signIn.isDisabled(), 'sign-in is enabled with a 5-character passphrase');
      must(await page.getByText('Use at least 10 characters.').count() === 1, 'no hint about the length');
      await pass.fill('a long enough passphrase'); await again.fill('a different one'); await sleep(150);
      must(await signIn.isDisabled(), 'sign-in is enabled although the two passphrases differ');
      await again.fill('a long enough passphrase'); await sleep(150);
      must(await signIn.isEnabled(), 'sign-in stays disabled with a good passphrase');
      await signIn.click();
      await page.getByText(/Connected as/).waitFor({ timeout: 4000 });
      await page.getByRole('button', { name: 'Back up now' }).click();
      await page.getByText(/Last backup/).waitFor({ timeout: 4000 });
      await page.getByRole('button', { name: 'Show backups' }).click(); await sleep(300);
      must(await page.getByRole('button', { name: 'Restore…' }).count() === 1, 'the new backup is not listed');
      await page.getByRole('button', { name: 'Restore…' }).click();
      await page.getByRole('button', { name: 'Restore', exact: true }).click();
      await page.getByText(/Restored/).first().waitFor({ timeout: 4000 });
      await page.getByRole('button', { name: 'Disconnect…' }).click();
      await page.getByRole('button', { name: 'Disconnect', exact: true }).click();
      await page.getByRole('button', { name: 'Sign in with Google' }).waitFor({ timeout: 4000 });
    });

    await check('Game locations: add and remove a prefix folder', async () => {
      const field = page.getByLabel('Prefix folder to add');
      await field.fill('/tmp/e2e-prefix'); await page.getByRole('button', { name: 'Add', exact: true }).click(); await sleep(300);
      must(await page.getByText('/tmp/e2e-prefix').count() === 1, 'the folder was not added');
      await page.locator('.item', { hasText: '/tmp/e2e-prefix' }).getByRole('button', { name: 'Remove' }).click(); await sleep(300);
      must(await page.getByText('/tmp/e2e-prefix').count() === 0, 'the folder was not removed');
    });

    await check('Diagnostics: log can be reloaded, copied and cleared', async () => {
      await page.getByLabel('Keep a log').check(); await sleep(400);
      for (const n of ['Reload', 'Copy', 'Clear']) must(await page.getByRole('button', { name: n, exact: true }).count() >= 1, `no ${n} button`);
    });

    await check('Accounts: delete all saved history', async () => {
      await page.getByRole('button', { name: /Accounts/ }).click(); await sleep(500);
      await page.getByRole('button', { name: 'Delete all saved history…' }).click();
      await page.getByRole('button', { name: 'Delete everything' }).click(); await sleep(800);
      must(await page.getByText('No account yet.').count() === 3, 'the accounts are still listed');
      must(await page.getByRole('button', { name: 'Delete all saved history…' }).count() === 0, 'the data panel is still shown with nothing to delete');
    });
    await check('new screens raised no page errors', async () => { must(errors.length === 0, errors[0]); });
    await browser.close();
  }
} finally {
  app.stop();
}

for (const [pass, name, detail] of results) console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
const failed = results.filter((r) => !r[0]).length;
console.log(failed ? `\n${failed} of ${results.length} checks failed` : `\nAll ${results.length} checks passed`);
process.exit(failed ? 1 : 0);
