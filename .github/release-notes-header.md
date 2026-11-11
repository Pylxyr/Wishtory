## Installation

Wishtory is for **Windows 10 and Windows 11**, and for **Linux** (the games run in Wine or Proton). The Windows build is below; Linux is built from source with one command, see [the Linux guide](https://github.com/Pylxyr/Wishtory/blob/main/docs/linux.md).

**Windows:** download **`Wishtory_x.y.z_x64-setup.exe`** below and run it. It installs for your user only and never needs administrator rights. Prefer no installer? **`Wishtory_x.y.z_windows_x64.zip`** holds the same app: unpack it anywhere and run `wishtory.exe`.

**Linux:** there is no download. Build it from source with `bash packaging/linux/install.sh`; the README has the steps for CachyOS, Arch, Debian, Ubuntu and Fedora. If a game is not found, add its Wine prefix under Settings, Game locations.

**Windows SmartScreen:** the installer is not code-signed yet, so Windows may show "Windows protected your PC". Choose **More info**, then **Run anyway**. To check the file first, compare its SHA-256 with `SHA256SUMS.txt` (`Get-FileHash <file>` in PowerShell).

Wishtory is an unofficial fan project and is not affiliated with HoYoverse or COGNOSPHERE. It only reads files the game already wrote and uses the official history service. No tool can offer a 100% guarantee; use it at your own risk.
