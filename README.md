<div align="center">

<img src="assets/icons/icon.png" alt="Wishtory icon" width="110" height="110">

# Wishtory

Track your wish history, pity and 50/50 results for Genshin Impact, Honkai: Star Rail and Zenless Zone Zero.

[![Release](https://img.shields.io/github/v/release/Pylxyr/Wishtory?style=for-the-badge&logo=github&label=Release)](https://github.com/Pylxyr/Wishtory/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Pylxyr/Wishtory/total?style=for-the-badge&logo=github&label=Downloads)](https://github.com/Pylxyr/Wishtory/releases)
[![Platforms](https://img.shields.io/badge/Windows%20%7C%20Linux-0078D6?style=for-the-badge&logo=windows11&logoColor=white)](#-install)
[![License](https://img.shields.io/github/license/Pylxyr/Wishtory?style=for-the-badge&label=License)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/Pylxyr/Wishtory/ci.yml?style=for-the-badge&logo=githubactions&logoColor=white&label=CI)](https://github.com/Pylxyr/Wishtory/actions/workflows/ci.yml)

[Install](#-install) · [Use it](#-use-it) · [Privacy](#-privacy) · [More](#-more)

</div>

## ✨ Features

- 🎯 **Pity at a glance.** Your current pity on every banner, the distance to soft and hard pity, and whether your next 5-star is guaranteed.
- 📊 **Statistics per banner.** Totals, share of pulls and average pity for 5-star and 4-star, how your 50/50s went, and a chip for every 5-star showing how lucky it was.
- 🔎 **Every pull, searchable.** A timeline of your rare pulls and a full table with search, filters, grouping and CSV copy.
- 🎮 **Three games, many accounts.** Switch between games and accounts in one app, and give each account a nickname.
- 🔁 **Import and export.** UIGF, SRGF and Paimon.moe files.
- 🔄 **Up to date.** Banners and item names update themselves, and a catalog update that would change your saved pulls waits for your review.

| Game | Open this page in the game |
| :--- | :--- |
| **Genshin Impact** | Wish, then History |
| **Honkai: Star Rail** | Warp, then Records |
| **Zenless Zone Zero** | Signal Search, then History |

## ⬇️ Install

**Windows 10 and 11.** Download from the [latest release](https://github.com/Pylxyr/Wishtory/releases/latest). No administrator rights needed.

| Download | What it is |
| :--- | :--- |
| `Wishtory_x.y.z_x64-setup.exe` | Installer, for your user only |
| `Wishtory_x.y.z_windows_x64.zip` | No installer: unpack anywhere and run `wishtory.exe` |

Builds are not code-signed yet, so Windows SmartScreen may warn you the first time: choose **More info**, then **Run anyway**.

**Linux.** One command builds and installs it for your user. See the [Linux guide](docs/linux.md).

## 🚀 Use it

1. Open the history page in the game and let the list load.
2. Switch to Wishtory and press **Refresh**.

The game keeps only about six months of history, so refresh now and then. More in [Using Wishtory](docs/using.md): shortcuts, importing and exporting, and where your data is stored.

## 🔒 Privacy

- Wishtory only **reads** files the game already wrote. It never touches the game.
- Your history stays on your PC. Your history link goes only to the official history service of that game.
- No accounts, no analytics, no tracking.

Read the [Privacy Policy](PRIVACY.md).

## 📚 More

| | |
| :--- | :--- |
| ☁️ [Cloud backup](docs/cloud-backup.md) | Optional encrypted backup to your own Google Drive |
| 🐧 [Linux guide](docs/linux.md) | Install, update and uninstall, and game locations under Wine |
| 📖 [Using Wishtory](docs/using.md) | Shortcuts, import and export, data folder |
| 🤝 [Credits and disclaimers](docs/credits.md) | Projects Wishtory relies on |
| 🛠️ [Developer guide](docs/development.md) | [Building the app](docs/redesign/BUILDING.md) · [Releasing](docs/releasing.md) · [Game catalog updates](docs/game-catalog-updates.md) |

## License

[MIT](LICENSE)
