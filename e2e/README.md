# Interface checks (optional)

Browser checks for the Wishtory frontend. They serve the built `dist/` with a mocked Tauri bridge
(`mock-tauri.js`, synthetic pulls for all three games) and drive it in Chromium. Nothing here touches the
Rust backend, and nothing in the main package depends on this folder.

```sh
npm run build                 # in the repository root
cd e2e
npm install
npx playwright install chromium
npm test                      # SCREENSHOTS=some/dir npm test also saves a screenshot per page and theme
```

What it checks: the pill style of chips in both themes, tooltips (focus, Esc, hover), toasts, theme following the
system, remembered page and game, the expired-link recovery card and Try again, rate-limit countdown, live banners
and per-game wording, a 50,000-pull History (windowed rows, sorting, filters, CSV), Accounts labels and the confirm
timer, keyboard skip link, the first-run tour, the collapsed sidebar, layout at 1280x800, 960x640 and 800x600, and an
axe-core sweep (WCAG 2.2 AA plus best practice) of every page in both themes.

Scenarios are chosen with `?s=` on the page URL: `empty`, `expired`, `expired-once`, `rate`, `network`, `busy`,
`pityerr`, `toasterr` and `big` (50,000 pulls).

Limits: this is Chromium, not Tauri's WebView2, so Windows font rendering, DPI scaling, High Contrast mode and real
screen readers still need a manual pass on Windows.
