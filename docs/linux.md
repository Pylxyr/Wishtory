# Wishtory on Linux

Wishtory runs on Linux and is built from source by one script.

## Install

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

On a system without `pacman`, `apt` or `dnf`, the script lists what is missing: a C toolchain, `git`, `curl`, `unzip`, `xz`, `zip`, `clang`, `cmake`, `ninja`, `pkg-config`, the GTK 3 development files and Rust 1.99 or newer. Arch users who already have Flutter can also build a package from `packaging/arch/wishtory-git` with `makepkg -sic`. To include [cloud backup](cloud-backup.md) in your own build, see [cloud-backup.md](cloud-backup.md).

[Back to the README](../README.md)
