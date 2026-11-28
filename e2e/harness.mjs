// Shared by shots.mjs and smoke.mjs: starts the Svelte window on the mock backend and opens it in Chromium.
// PW_CHROMIUM=/path/to/chrome uses that browser instead of Playwright's own; PW_NO_SANDBOX=1 (or running as
// root, as in many containers) adds --no-sandbox.
import { chromium } from 'playwright';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const PORT = Number(process.env.E2E_PORT || 1421);

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Starts Vite with VITE_MOCK=1 and waits until it answers. `stop()` ends it for good. */
export async function startApp() {
  // `node vite.js` rather than `npx vite`: killing npx would leave the real server running on the port.
  const vite = spawn(process.execPath, [`${root}node_modules/vite/bin/vite.js`, '--port', String(PORT), '--strictPort', '--host', '127.0.0.1'], {
    cwd: root, env: { ...process.env, VITE_MOCK: '1' }, stdio: 'ignore'
  });
  let exited = false;
  vite.on('exit', () => { exited = true; });
  const url = `http://127.0.0.1:${PORT}`;
  for (let i = 0; i < 120; i++) {
    if (exited) throw new Error(`Vite exited early (is port ${PORT} taken? set E2E_PORT)`);
    try { if ((await fetch(url)).ok) return { url, stop: () => vite.kill() }; } catch { /* not up yet */ }
    await sleep(500);
  }
  vite.kill();
  throw new Error('Vite did not start in time');
}

export async function openBrowser({ width = 1360, height = 900, scheme = 'dark', init } = {}) {
  const sandbox = process.env.PW_NO_SANDBOX || (process.getuid && process.getuid() === 0);
  const browser = await chromium.launch({
    executablePath: process.env.PW_CHROMIUM || undefined,
    args: sandbox ? ['--no-sandbox', '--disable-gpu'] : []
  });
  const context = await browser.newContext({ viewport: { width, height }, colorScheme: scheme });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => m.type() === 'error' && !/enka|githubusercontent|ERR_|Failed to load resource/.test(m.text()) && errors.push(m.text()));
  await page.route(/enka\.network|githubusercontent/, (r) => r.abort()); // pictures fall back to initials
  if (init) await page.addInitScript(init);
  return { browser, context, page, errors };
}
