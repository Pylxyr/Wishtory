// Looks at the UI the way a person would: starts Vite with the mock backend, opens each game's
// Overview and History, a page change and the Settings page, and saves screenshots to e2e/out/.
// Usage: npm run shots  (needs `npx playwright install chromium` once)
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { openBrowser, startApp } from './harness.mjs';

const out = fileURLToPath(new URL('./out/', import.meta.url));
await mkdir(out, { recursive: true });
const app = await startApp();
const { browser, page, errors } = await openBrowser();
await page.goto(app.url);
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
app.stop();
