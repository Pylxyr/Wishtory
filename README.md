<div align="center">

<img src="assets/icons/icon.png" alt="Wishtory" width="112" height="112">

# Wishtory

**Your wish history, pity and 50/50s for Genshin Impact, Honkai: Star Rail and Zenless Zone Zero.**<br>
Read straight from the game's own saved data. Kept on your own PC.

<br>

[![Release](https://img.shields.io/github/v/release/Pylxyr/Wishtory?style=for-the-badge&color=5AA2FF&label=release)](https://github.com/Pylxyr/Wishtory/releases)
[![License](https://img.shields.io/github/license/Pylxyr/Wishtory?style=for-the-badge&color=5AA2FF)](LICENSE)
![Platforms](https://img.shields.io/badge/Windows%20%7C%20Linux-0D1015?style=for-the-badge&logo=linux&logoColor=white)
![Flutter](https://img.shields.io/badge/Flutter-02569B?style=for-the-badge&logo=flutter&logoColor=white)
![Rust](https://img.shields.io/badge/Rust%20core-1A1A1A?style=for-the-badge&logo=rust&logoColor=white)

![Genshin Impact](https://img.shields.io/badge/Genshin%20Impact-supported-5AA2FF?style=flat-square)
![Honkai: Star Rail](https://img.shields.io/badge/Honkai%3A%20Star%20Rail-supported-5AA2FF?style=flat-square)
![Zenless Zone Zero](https://img.shields.io/badge/Zenless%20Zone%20Zero-supported-5AA2FF?style=flat-square)
![Local first](https://img.shields.io/badge/data-stays%20on%20your%20PC%20by%20default-2EA44F?style=flat-square)
![Encrypted backup](https://img.shields.io/badge/cloud%20backup-optional%20%C2%B7%20end--to--end%20encrypted-2EA44F?style=flat-square)
![Read only](https://img.shields.io/badge/game%20files-read%20only-2EA44F?style=flat-square)

[Features](#features) &nbsp;|&nbsp; [Install](#install) &nbsp;|&nbsp; [Linux](#build-on-linux-cachyos-and-arch-included) &nbsp;|&nbsp; [Use it](#use-it) &nbsp;|&nbsp; [Is it safe?](#is-it-safe) &nbsp;|&nbsp; [Import and export](#import-and-export) &nbsp;|&nbsp; [Cloud backup](#cloud-backup-optional) &nbsp;|&nbsp; [Credits](#credits)

</div>

<br>

Wishtory opens the wish history the game already saved, then lays it out so you can answer the questions that matter right after a pull: where is my pity, is my next 5-star guaranteed, and how lucky was I?

---

## Features

<table>
<tr>
<td width="50%" valign="top">

### Pity at a glance
One card per banner with your current pity, the distance to soft and hard pity, and whether your next 5-star is a guaranteed one. Colour is never the only signal: every state also has a label.

</td>
<td width="50%" valign="top">

### Your luck, charted
A bar for each 5-star you have pulled, sized by the pity it took and tinted by the result: won the 50/50, lost it, or guaranteed. Averages sit right next to it.

</td>
</tr>
<tr>
<td width="50%" valign="top">

### Every pull, searchable
A full timeline of your rare pulls, plus a dense table of every pull with search, banner and rarity filters, grouping by banner or month, and CSV copy.

</td>
<td width="50%" valign="top">

### Three games, many accounts
Switch between Genshin Impact, Honkai: Star Rail and Zenless Zone Zero, and between accounts, in one app. Give each account a nickname.

</td>
</tr>
<tr>
<td width="50%" valign="top">

### Move in, move out
Import and export with UIGF, SRGF and Paimon.moe backups, so your history is never locked in. Optionally back up to your own Google Drive, encrypted on your PC first.

</td>
<td width="50%" valign="top">

### Stays current
Banners and item names update by themselves, and Wishtory tells you when a new version is out. Dark and light themes follow your system, or pick one.

</td>
</tr>
</table>

### Supported games

| Game | Open this page in the game | Banners tracked |
| :--- | :--- | :--- |
| **Genshin Impact** | Wish, then History | Character Event, Weapon Event, Standard, Chronicled, Beginners' |
| **Honkai: Star Rail** | Warp, then Records | Character Event, Light Cone Event, Stellar, Collaboration, Departure |
| **Zenless Zone Zero** | Signal Search, then History | Exclusive, W-Engine, Standard, Bangboo, Rescreening, Reverberation |

---

## Install

**Windows 10 or 11:** download the build from the [latest release](https://github.com/Pylxyr/Wishtory/releases). Wishtory never needs administrator rights.

| Download | Notes |
| :--- | :--- |
| `Wishtory_x.y.z_x64-setup.exe` | Installer, for your user only |
| `Wishtory_x.y.z_windows_x64.zip` | No installer: unpack anywhere and run `wishtory.exe` |

To check a download, compare its SHA-256 with `SHA256SUMS.txt` from the same release page: run `Get-FileHash <file>` in PowerShell. Builds are not code-signed yet, so Windows SmartScreen may warn you the first time: choose **More info**, then **Run anyway**.

**Linux:** there is no download. You build Wishtory from source with one command, which also cleans up after itself. See [Build on Linux](#build-on-linux-cachyos-and-arch-included) below.

<details>
<summary><b>Linux: Wine and Proton</b></summary>

<br>

The games run in Wine or Proton on Linux, and Wishtory reads the same files inside the Wine prefix. It looks in the usual places by itself: Wine's default prefix, Lutris, the anime-game and honkers launchers, Bottles, Steam, and their Flatpak versions.

If a game is not found, launch it once so it writes its log, then add the folder that holds its prefix under **Settings, then Game locations**.

Opening links from the app needs `xdg-utils`, which most desktops already have.

</details>

---

## Build on Linux (CachyOS and Arch included)

There is no Linux download, but building it takes one command. The script asks before it does anything, sets up what it needs, installs Wishtory for your user (no root for the app itself, everything goes under `~/.local`), and then **deletes everything it downloaded and built**, so only the installed app is left.

**1. Get the source and run the script.** `git` comes with CachyOS; on other systems install it first (`sudo pacman -S git`, `sudo apt install git` or `sudo dnf install git`).

```sh
git clone https://github.com/Pylxyr/Wishtory.git
cd Wishtory
bash packaging/linux/install.sh
```

**2. Read what it plans, and answer `Y`.** On a fresh CachyOS it will show something like this:

```text
To build Wishtory this script will:
  - install build packages with sudo and pacman: base-devel git curl unzip xz zip clang cmake ninja gtk3 xdg-utils rust
  - download Flutter into a temporary folder (about 2 GB)
  - build Wishtory (several minutes)
  - install it under /home/you/.local
  - delete everything it downloaded and built
Continue? [Y/n]
```

Only what your system lacks is listed. Packages come from your own package manager (`pacman`, `apt` or `dnf`, with a `sudo` password prompt), so there is no AUR helper and no manual Flutter or Rust setup. Flutter, and Rust when your system has none or an old one, are downloaded into a temporary folder and removed at the end. You need an internet connection and about 3 GB of free disk space while it runs.

Wishtory then appears in your application menu, and `wishtory` starts it from a terminal.

| To | Run |
| :--- | :--- |
| Update | `git pull`, then `bash packaging/linux/install.sh` again |
| Skip the questions | `bash packaging/linux/install.sh --yes` |
| Keep the build files for faster rebuilds | `bash packaging/linux/install.sh --keep-build` |
| Uninstall | `bash packaging/linux/install.sh --uninstall` |

Your saved history lives in `~/.local/share/app.wishtory.desktop` and survives updates and uninstalling. Delete that folder to remove it too. The cleanup only removes what the script itself downloaded and built in this folder; the packages it installed through `pacman`, `apt` or `dnf` stay, because other programs use them too.

To include [cloud backup](#cloud-backup-optional) in your own build, give the script your Google client ID first (see [docs/cloud-backup.md](docs/cloud-backup.md)): `WISHTORY_GOOGLE_CLIENT_ID=... WISHTORY_GOOGLE_CLIENT_SECRET=... bash packaging/linux/install.sh`. Without them the app is built without cloud backup.

On a distribution without `pacman`, `apt` or `dnf`, the script lists what is missing: a C toolchain, `git`, `curl`, `unzip`, `xz`, `zip`, `clang`, `cmake`, `ninja`, `pkg-config`, the GTK 3 development files and Rust 1.91 or newer. Install them, then run it again, and it downloads Flutter itself.

Arch users who already have Flutter installed can also build a pacman package from `packaging/arch/wishtory-git` with `makepkg -sic`.

---

## Use it

**1. Open the history page in the game** and let the list load.

**2. Switch to Wishtory and press Refresh.**

That is the whole routine. The game only keeps about six months of history, so refresh now and then, and import an older export to keep the rest.

If Wishtory says it cannot find the history page, you have not opened it yet, or the link has expired: open it in the game again and retry. You can also paste a history link by hand from the arrow next to **Refresh**.

| Shortcut | Action |
| :--- | :--- |
| `Ctrl` + `R` | Refresh |
| `Alt` + `1` / `2` / `3` | Open Genshin Impact / Star Rail / Zenless Zone Zero |
| `Ctrl` + `,` | Settings |
| `F1` | Help |

Your data lives in one folder on your PC:

| System | Location |
| :--- | :--- |
| Windows | `%APPDATA%\app.wishtory.desktop\wishtory.db` |
| Linux | `~/.local/share/app.wishtory.desktop/wishtory.db` |

---

## Is it safe?

Wishtory works the way the well-known community trackers do. It only **reads** files the game already wrote, then asks the official history service for your records.

- It never attaches to, injects into or talks to the game.
- It never changes, locks or writes to any game file or folder.
- It never sets a proxy and never needs administrator rights.
- Your history link is only ever sent to the official HoYoverse history service for that game.
- Its only other internet use is the banner and item catalog, item pictures (downloaded once, then kept on your PC), and the check for new versions. No account or wish data is sent to any of them.
- **Cloud backup is off until you turn it on.** Only then does anything of yours leave your PC, and only as an encrypted file in your own Google Drive; see [Cloud backup](#cloud-backup-optional) below.

The technical details are in [the developer guide](docs/development.md).

---

## Updates

- **The app** checks this project's GitHub releases at most once a day, and when you press **Check for updates** in Settings or About. If a newer version exists you see a dot on Settings and a **View release** button that opens the page in your browser. Nothing is installed automatically.
- **Banners and item names** refresh by themselves once a day. Press **Check for updates** next to Catalog updates in Settings to check right away.

---

## Import and export

Wishtory speaks the [UIGF](https://uigf.org) standard and a few older formats:

| Format | Import | Export |
| :--- | :---: | :---: |
| UIGF v4.x, all three games | yes | yes (v4.2, recommended) |
| UIGF v2.x and v3.x, Genshin | yes | v3.0 |
| SRGF v1.0, Star Rail | yes | yes |
| Paimon.moe backup, Genshin | yes | yes |

UIGF is the best choice for moving between trackers. Times are kept as the account's server-local time. Rows without a rarity are skipped, and the import summary tells you how many.

---

## Cloud backup (optional)

Wishtory can keep a backup of your wish history in **your own Google Drive**, the way Android backup apps do: sign in once, then press **Back up now** or let it back up by itself once a day. Restore it on any PC by signing in with the same Google account and passphrase. It is off until you set it up in **Settings → Cloud backup**.

**What protects your data**

- **Encrypted before it leaves your PC.** You choose a passphrase (at least 10 characters). Every backup is encrypted with it using Argon2id and ChaCha20-Poly1305, so Google, and anyone who gets into your Google account, only ever sees a file they cannot read. **If you forget the passphrase, the backups cannot be opened**, and nobody, including the developer, can recover them.
- **Wishtory can see nothing else in your Drive.** It asks Google for one permission only, access to its own hidden app-data folder. It cannot list, read or change any of your other files, and you will not see the backups in the normal Drive view.
- **Your Google password never reaches Wishtory.** Sign-in happens on Google's own page in your browser.
- **Secrets stay in your system's password storage.** The Google sign-in and the passphrase are kept in Windows Credential Manager, or in the Secret Service on Linux (KWallet and GNOME Keyring provide it). They are never written to a file, and if your system has no such storage, cloud backup tells you and stays off.
- **Restoring only adds.** A restore merges into your saved history and skips pulls you already have. It never removes anything.
- **You stay in control.** The last 5 backups are kept and older ones are deleted. **Disconnect** removes the sign-in from your PC and revokes it with Google, and can delete the backups from your Drive too. You can also remove Wishtory's access any time at [myaccount.google.com/permissions](https://myaccount.google.com/permissions).

What is backed up is the same wish history the **Export** page writes as UIGF, for every account. Item pictures, nicknames and settings are not included.

This needs a build that includes a Google client ID. The Windows releases do when the maintainer has set one up; if your copy was built without it, the Settings page says so and nothing is ever sent. Building it yourself is described in [docs/cloud-backup.md](docs/cloud-backup.md).

---

## Disclaimers

- Wishtory is an unofficial fan project. It is not affiliated with or endorsed by HoYoverse / COGNOSPHERE, or by any of the projects credited below.
- No tool can offer a 100% guarantee. HoYoverse can change its terms of service or its detection at any time, and nothing here can promise how the publisher will treat any third-party tool. Use Wishtory at your own risk.
- Genshin Impact, Honkai: Star Rail and Zenless Zone Zero, and their names, characters and images, belong to their owners.
- Soft-pity ranges are community estimates, because the games do not publish them.

---

## Credits

Wishtory relies on these independent projects and references. The same list is under **About** in the app.

### Data and references

- [EnkaNetwork API documentation and game stores](https://github.com/EnkaNetwork/API-docs/tree/master/store): item IDs, names, rarities and character traits behind the game catalog. Item icons load from enka.network.
- [Paimon.moe](https://paimon.moe/) and its [open-source item data](https://github.com/MadeBaruna/paimon-moe/tree/main/src/data): exact item keys match backup records, its Genshin backup format is supported, and its documented read-only cache lookup approach informed history-link discovery.
- The [UIGF project](https://uigf.org/): the format used to move wish histories between compatible tools.

### Game wikis

The community wiki for each game, used to check banner dates, featured items and game terms. References only; Wishtory does not use them at run time.

| Game | Wiki |
| :--- | :--- |
| Genshin Impact | [Genshin Impact Wiki (Fandom)](https://genshin-impact.fandom.com/) |
| Honkai: Star Rail | [Honkai: Star Rail Wiki (Fandom)](https://honkai-star-rail.fandom.com/) |
| Zenless Zone Zero | [Zenless Zone Zero Wiki (Fandom)](https://zenless-zone-zero.fandom.com/) |

### Banner schedule cross-checks

Banner windows, titles and featured items were also checked against these community trackers. References only.

- [Banner History](https://bannerhistory.app/)
- [GachaTracker](https://gachatracker.app/)

### Built with

[Flutter](https://flutter.dev/) (BSD-3-Clause), [Rust](https://www.rust-lang.org/) (Apache-2.0 OR MIT), [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) (MIT), [Riverpod](https://riverpod.dev/) (MIT), [SQLite](https://sqlite.org/) (public domain), and the fonts Google Sans and JetBrains Mono through [Fontsource](https://fontsource.org/) (OFL-1.1). The full list of libraries and their licences is in the source repository.

---

## For developers

- [Building the app](docs/redesign/BUILDING.md): the Rust core, the Flutter UI and the bridge between them.
- [Design notes](docs/redesign/DESIGN.md): why Flutter, and how the screens fit together.
- [Developer guide](docs/development.md): project layout, how the game files are read, import and export details.
- [Releasing](docs/releasing.md) and [game catalog updates](docs/game-catalog-updates.md).

## License

[MIT](LICENSE)
