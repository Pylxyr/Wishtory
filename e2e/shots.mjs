// Looks at the UI the way a person would: starts Vite with the mock backend, opens each game's
// Overview and History, a page change and the Settings page, and saves screenshots to e2e/out/.
// Usage: npm run shots  (needs `npx playwright install chromium` once)
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';

const PORT = 1420;
const out = new URL('./out/', import.meta.url).pathname;
await mkdir(out, { recursive: true });
const vite = spawn('npx', ['vite', '--port', String(PORT), '--strictPort'], { env: { ...process.env, VITE_MOCK: '1' }, stdio: 'ignore' });
for (let i = 0; i < 60; i++) { try { if ((await fetch(`http://localhost:${PORT}`)).ok) break; } catch {} await new Promise((r) => setTimeout(r, 500)); }

const browser = await chromium.launch({ executablePath: process.env.PW_CHROMIUM || undefined });
const page = await browser.newPage({ viewport: { width: 1360, height: 900 } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
page.on('console', (m) => m.type() === 'error' && !/enka|githubusercontent|ERR_/.test(m.text()) && errors.push(m.text()));
await page.route(/enka\.network|githubusercontent/, (r) => r.abort());
await page.goto(`http://localhost:${PORT}`);
await page.emulateMedia({ colorScheme: 'dark' });
await page.waitForSelector('.shell');

const games = [['genshin', 'Genshin'], ['star_rail', 'Star Rail'], ['zenless', 'Zenless Zone Zero']];
for (const [id, label] of games) {
  await page.getByRole('button', { name: new RegExp(label) }).first().click();
  await page.waitForTimeout(2600);
  await page.screenshot({ path: `${out}${id}-overview.png` });
  await page.getByRole('tab', { name: 'History' }).click();
  await page.waitForTimeout(900);
  await page.screenshot({ path: `${out}${id}-history.png` });
  await page.getByRole('tab', { name: 'Overview' }).click();
  await page.waitForTimeout(500);
}
await page.getByRole('button', { name: /Settings/ }).click();
await page.waitForTimeout(800);
await page.screenshot({ path: `${out}settings.png` });
// a refresh, mid-way, on Genshin
await page.getByRole('button', { name: /Genshin/ }).first().click();
await page.getByRole('button', { name: /^Refresh/ }).click();
await page.waitForTimeout(1200);
await page.screenshot({ path: `${out}genshin-refreshing.png` });
await page.waitForTimeout(3000);
await page.screenshot({ path: `${out}genshin-refreshed.png` });
// light mode
await page.emulateMedia({ colorScheme: 'light' });
await page.waitForTimeout(600);
await page.screenshot({ path: `${out}genshin-light.png` });
console.log(errors.length ? 'Errors:\n' + errors.join('\n') : 'No page errors.');
await browser.close();
vite.kill();
