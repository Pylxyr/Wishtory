# Wishtory

Keep your **Genshin Impact**, **Honkai: Star Rail** and **Zenless Zone Zero** wish history in one place on your own PC. Wishtory reads the history the game already saved, then shows your pity, your 50/50 results and every pull, with the banner each one was made on. Your history stays on your own PC.

## What you get

- **Pity at a glance.** How far you are into pity, how close to soft and hard pity, and your 50/50 state (win, loss or guaranteed) for each banner.
- **A full history.** Every pull with its banner name, and the pity number on each 4★ and 5★.
- **All three games, many accounts.** Switch between games and accounts in one app.
- **Import and export.** Move your history to or from other trackers with UIGF (also SRGF and Paimon.moe backups).
- **Stays up to date.** Banners and item names update by themselves, and Wishtory tells you when a new version is out.

## Install

Windows 10 or 11.

1. Download **`Wishtory_x.y.z_x64-setup.exe`** from the [latest release](https://github.com/Pylxyr/Wishtory/releases).
2. Run it. It installs for your user only and never needs administrator rights.

The installer is not code-signed yet, so Windows SmartScreen may warn you the first time. Choose **More info**, then **Run anyway**. To check the download first, compare its SHA-256 with `SHA256SUMS.txt` in the release (`Get-FileHash <file>` in PowerShell).

## Use it

1. **Open the history page in the game** (Genshin: Wish → History, Star Rail: Warp → Records, Zenless: Signal Search → History) and let it load.
2. **Switch to Wishtory and press Refresh.**

If Wishtory says it cannot find the history page, you have not opened it yet, or the link expired: open it in the game again and retry. You can also paste a history link on the Overview page.

The games only keep about 6 months of history. Import an older UIGF export to keep the rest. Your data is stored on your PC at `%APPDATA%\app.wishtory.desktop\wishtory.db`.

## Is it safe?

Wishtory works the way the well-known community trackers do. It only **reads** files the game already wrote, then asks the official history service for your records.

- It never attaches to, injects into or talks to the game.
- It never changes, locks or writes to any game file or folder.
- It never sets a proxy and never needs administrator rights.
- Your history link is only ever sent to the official HoYoverse history service for that game.
- Its only other internet use is the banner and item catalog and the check for new versions, both from this project's GitHub page. No account or wish data is sent there.

The technical details are in [the developer guide](docs/development.md).

## Updates

- **The app** checks this project's GitHub releases at most once a day, and when you press **Check for updates** in Settings or About. If a newer version exists you see a dot on Settings and a **View release** button that opens the page in your browser. Nothing is installed automatically.
- **Banners and item names** (the catalog) refresh by themselves once a day. Press **Check for updates** next to Catalog updates in Settings to check right away.

## Import and export

Wishtory imports and exports the [UIGF](https://uigf.org) standard:

- UIGF v4.x for Genshin, Star Rail and Zenless (exported as v4.2, including Star Rail event pool types 21 and 22).
- Import of older UIGF (v2.x and v3.x for Genshin) and SRGF v1.0 (Star Rail).
- Export to UIGF v3.0 (Genshin) and SRGF v1.0 (Star Rail) for older trackers.
- Paimon.moe backups for Genshin, both ways. UIGF is still the better choice for moving between trackers.

Times are kept as the account's server-local time. Rows without a rarity are skipped and the import summary tells you how many.

## Disclaimers

- Wishtory is an unofficial fan project. It is not affiliated with or endorsed by HoYoverse / COGNOSPHERE, or by any of the projects credited below.
- No tool can offer a 100% guarantee. HoYoverse can change its terms of service or its detection at any time, and nothing here can promise how the publisher will treat any third-party tool. Use Wishtory at your own risk.
- Genshin Impact, Honkai: Star Rail and Zenless Zone Zero, and their names, characters and images, belong to their owners.
- Soft-pity ranges are community estimates, because the games do not publish them.
- The installer is not code-signed, so Windows SmartScreen may warn about it.

## Credits

Wishtory relies on these independent projects and references.

**Data and references**

- [EnkaNetwork's API documentation and processed game stores](https://github.com/EnkaNetwork/API-docs/tree/master/store): item IDs, names, rarities and character traits behind the game catalog. Item icons load from enka.network.
- [Paimon.moe](https://paimon.moe/) and its [open-source item data](https://github.com/MadeBaruna/paimon-moe/tree/main/src/data): exact item keys match backup records, its Genshin backup format is supported, and its documented read-only cache lookup approach informed history-link discovery.
- The [UIGF project](https://uigf.org/): the format used to move wish histories between compatible tools.
- HoYoverse's official announcements, used to check game terminology, mechanics and banner schedules, including the [Honkai: Star Rail Path of Elation](https://www.hoyolab.com/article/43681649) and [Zenless Zone Zero's new specialties](https://zenless.hoyoverse.com/en-us/news/166000).

**Banner schedule cross-checks.** Banner windows, titles and featured items were checked against HoYoverse's notices and these community trackers and wikis. They are references only; Wishtory does not use them at run time: [Banner History](https://bannerhistory.app/), [GachaTracker](https://gachatracker.app/), the [Genshin Impact Wiki](https://genshin-impact.fandom.com/) and the [Zenless Zone Zero Wiki](https://zenless-zone-zero.fandom.com/).

**Built with:** [Tauri](https://tauri.app/) (Apache-2.0 OR MIT), [Svelte](https://svelte.dev/) (MIT), [Vite](https://vite.dev/) (MIT), [SQLite](https://sqlite.org/) (public domain), and the fonts Google Sans and JetBrains Mono through [Fontsource](https://fontsource.org/) (OFL-1.1). The full list of libraries and their licences is in the source repository. The same credits are under **About** in the app.

## For developers

- [Developer guide](docs/development.md): building, project layout, checks, how the game files are read, import and export details.
- [Releasing](docs/releasing.md): cutting a release and the repository settings it needs.
- [Game catalog updates](docs/game-catalog-updates.md): how the signed item catalog and banner list are sourced, reviewed and published.

## License

[MIT](LICENSE)
