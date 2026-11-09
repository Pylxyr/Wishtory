<div align="center">

<img src="assets/icons/icon.png" alt="Wishtory" width="96" height="96">

# Wishtory

**Wish history, pity and 50/50 tracking for Genshin Impact, Honkai: Star Rail and Zenless Zone Zero.**

Reads the history your game already saved. Your data stays on your PC.

[![Latest release](https://img.shields.io/github/v/release/Pylxyr/Wishtory?style=flat-square&label=release)](https://github.com/Pylxyr/Wishtory/releases) [![License: MIT](https://img.shields.io/github/license/Pylxyr/Wishtory?style=flat-square)](LICENSE)

[Install](#install) · [Use it](#use-it) · [Privacy and safety](#privacy-and-safety) · [Cloud backup](#cloud-backup-optional) · [Credits](#credits)

</div>

<br>

Wishtory answers the questions that matter right after a pull: where is my pity, is my next 5-star guaranteed, and how lucky have I been?

## What you get

- **Pity at a glance.** One card per banner with your current pity, the distance to soft and hard pity, and whether your next 5-star is guaranteed.
- **Your luck, charted.** A bar for every 5-star, sized by the pity it took and marked won, lost or guaranteed, with averages beside it.
- **Every pull, searchable.** A timeline of your rare pulls and a full table with search, filters, grouping and CSV copy.
- **Three games, many accounts.** Switch between games and accounts in one app, and give each account a nickname.
- **Import and export.** UIGF, SRGF and Paimon.moe files, so your history is never locked in.
- **Stays current.** Banners and item names update by themselves, and Wishtory tells you when a new version is out. Dark and light themes follow your system.

| Game | Open this page in the game | Banners tracked |
| :--- | :--- | :--- |
| **Genshin Impact** | Wish, then History | Character Event, Weapon Event, Standard, Chronicled, Beginners' |
| **Honkai: Star Rail** | Warp, then Records | Character Event, Light Cone Event, Stellar, Collaboration, Departure |
| **Zenless Zone Zero** | Signal Search, then History | Exclusive, W-Engine, Standard, Bangboo, Rescreening, Reverberation |

## Install

### Windows 10 and 11

Download from the [latest release](https://github.com/Pylxyr/Wishtory/releases). Wishtory never needs administrator rights.

| Download | Notes |
| :--- | :--- |
| `Wishtory_x.y.z_x64-setup.exe` | Installer, for your user only |
| `Wishtory_x.y.z_windows_x64.zip` | No installer: unpack anywhere and run `wishtory.exe` |

To check a download, compare its SHA-256 with `SHA256SUMS.txt` on the release page (`Get-FileHash <file>` in PowerShell). Builds are not code-signed yet, so Windows SmartScreen may warn you the first time: choose **More info**, then **Run anyway**.

### Build on Linux (CachyOS and Arch included)

There is no Linux download. One command builds and installs Wishtory for your user, then deletes everything it downloaded:

```sh
git clone https://github.com/Pylxyr/Wishtory.git
cd Wishtory
bash packaging/linux/install.sh
```

The script shows its plan and asks before doing anything. It installs only the build packages your system lacks (through `pacman`, `apt` or `dnf`, with a `sudo` prompt), downloads Flutter, and Rust if needed, into a temporary folder, builds the app, installs it under `~/.local`, and cleans up. Allow about 3 GB of free disk space and several minutes. Wishtory then appears in your application menu, and `wishtory` starts it from a terminal.

| To | Run |
| :--- | :--- |
| Update | `git pull`, then `bash packaging/linux/install.sh` again |
| Skip the questions | `bash packaging/linux/install.sh --yes` |
| Keep build files for faster rebuilds | `bash packaging/linux/install.sh --keep-build` |
| Uninstall | `bash packaging/linux/install.sh --uninstall` |

Your saved history lives in `~/.local/share/app.wishtory.desktop` and survives updates and uninstalling.

The games run in Wine or Proton, and Wishtory reads the same files inside the Wine prefix. It checks the usual places by itself (Wine, Lutris, Bottles, Steam, the anime-game launchers and their Flatpak versions). If a game is not found, launch it once, then add its prefix folder under **Settings, then Game locations**. Opening links needs `xdg-utils`.

On a system without `pacman`, `apt` or `dnf`, the script lists what is missing: a C toolchain, `git`, `curl`, `unzip`, `xz`, `zip`, `clang`, `cmake`, `ninja`, `pkg-config`, the GTK 3 development files and Rust 1.91 or newer. Arch users who already have Flutter can also build a package from `packaging/arch/wishtory-git` with `makepkg -sic`. To include [cloud backup](#cloud-backup-optional) in your own build, see [docs/cloud-backup.md](docs/cloud-backup.md).

## Use it

1. **Open the history page in the game** and let the list load.
2. **Switch to Wishtory and press Refresh.**

That is the whole routine. The game only keeps about six months of history, so refresh now and then, and import an older export to keep the rest. If Wishtory cannot find the history page, open it in the game again and retry, or paste a history link by hand from the arrow next to **Refresh**.

| Shortcut | Action |
| :--- | :--- |
| `Ctrl` + `R` | Refresh |
| `Alt` + `1` / `2` / `3` | Open Genshin Impact / Star Rail / Zenless Zone Zero |
| `Ctrl` + `,` | Settings |
| `F1` | Help |

Your data lives in one folder: `%APPDATA%\app.wishtory.desktop` on Windows, `~/.local/share/app.wishtory.desktop` on Linux.

**Import and export.** Wishtory reads and writes the [UIGF](https://uigf.org) standard (v4.x, recommended for moving between trackers, plus older v2 and v3 for Genshin), SRGF v1.0 for Star Rail, and Paimon.moe backups for Genshin. Rows without a rarity are skipped, and the import summary tells you how many.

## Privacy and safety

Wishtory works the way the well-known community trackers do. It only **reads** files the game already wrote, then asks the official history service for your records.

- It never attaches to, injects into or talks to the game, and never changes, locks or writes to any game file.
- It never sets a proxy and never needs administrator rights.
- Your history link is sent only to the official HoYoverse history service for that game.
- Its only other internet use is the banner and item catalog, item pictures (downloaded once), and the check for new versions. None of them receive account or wish data.
- No accounts, no analytics, no tracking. The developer receives nothing.

Read the full [Privacy Policy](PRIVACY.md). Technical details are in the [developer guide](docs/development.md).

## Cloud backup (optional)

Back up your history to **your own Google Drive**: sign in once in **Settings, then Cloud backup**, then press **Back up now** or let it run once a day. Restore on any PC with the same Google account and passphrase. It is off until you set it up.

- **Encrypted on your PC first.** You choose a passphrase, and Google only ever stores a file it cannot read. **If you forget the passphrase, the backups cannot be opened**, and nobody can recover them.
- **Wishtory sees nothing else in your Drive.** It asks Google for a single permission: access to its own hidden app folder.
- **Your Google password never reaches Wishtory**, and the sign-in and passphrase are kept in Windows Credential Manager or the Linux Secret Service, never in a file.
- **Restoring only adds.** It never removes a pull. **Disconnect** revokes access and can delete your backups from Drive.

Cloud backup needs a build that includes a Google client ID. If your copy has none, Settings says so and nothing is sent. Details are in [docs/cloud-backup.md](docs/cloud-backup.md).

## Disclaimers

- Wishtory is an unofficial fan project, not affiliated with or endorsed by HoYoverse / COGNOSPHERE, Google, or any project credited below.
- No tool can offer a 100% guarantee. HoYoverse can change its terms or its detection at any time, and nothing here can promise how the publisher treats any third-party tool. Use Wishtory at your own risk.
- The games and their names, characters and images belong to their owners. Soft-pity ranges are community estimates, because the games do not publish them.

## Credits

Wishtory relies on these independent projects and references (the same list is under **About** in the app):

- [EnkaNetwork API docs and game stores](https://github.com/EnkaNetwork/API-docs/tree/master/store) for item IDs, names, rarities and traits. Item icons load from enka.network.
- [Paimon.moe](https://paimon.moe/) and its [open-source item data](https://github.com/MadeBaruna/paimon-moe/tree/main/src/data), whose item keys and backup format are supported, and whose read-only cache lookup approach informed history-link discovery.
- The [UIGF project](https://uigf.org/), the format used to move wish histories between tools.
- Banner dates and featured items were checked against the community wikis ([Genshin Impact](https://genshin-impact.fandom.com/), [Honkai: Star Rail](https://honkai-star-rail.fandom.com/), [Zenless Zone Zero](https://zenless-zone-zero.fandom.com/)) and the trackers [Banner History](https://bannerhistory.app/) and [GachaTracker](https://gachatracker.app/). References only; Wishtory does not use them at run time.
- Built with [Flutter](https://flutter.dev/) (BSD-3-Clause), [Rust](https://www.rust-lang.org/) (Apache-2.0 OR MIT), [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) (MIT), [Riverpod](https://riverpod.dev/) (MIT), [SQLite](https://sqlite.org/) (public domain), and the fonts Google Sans and JetBrains Mono via [Fontsource](https://fontsource.org/) (OFL-1.1). The full list of libraries and licences is in the source repository.

## For developers

[Building the app](docs/redesign/BUILDING.md) · [Developer guide](docs/development.md) · [Releasing](docs/releasing.md) · [Game catalog updates](docs/game-catalog-updates.md) · [Cloud backup internals](docs/cloud-backup.md)

## License

[MIT](LICENSE)
