#!/usr/bin/env bash
# Builds Wishtory from this source folder and installs it for the current user. It sets up what it
# needs by itself, asking before it does, and deletes everything it downloaded or built afterwards,
# so only the installed app is left.
#
#   bash packaging/linux/install.sh                 build, install, clean up
#   bash packaging/linux/install.sh --yes           the same, without asking questions
#   bash packaging/linux/install.sh --keep-build    keep the build output and downloaded tools (faster rebuilds)
#   bash packaging/linux/install.sh --uninstall     remove the installed app
#
# What it may do, after you agree: install build packages with your package manager (sudo; pacman, apt
# and dnf are supported), download Flutter (and Rust when your system has none, or an old one) into a
# temporary folder, build, and install under ~/.local (or $PREFIX). Your saved history is not touched:
# it lives in ~/.local/share/app.wishtory.desktop.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
prefix="${PREFIX:-$HOME/.local}"
app_dir="$prefix/share/wishtory"
bin_link="$prefix/bin/wishtory"
desktop_file="$prefix/share/applications/wishtory.desktop"
# icon folder : file in assets/icons
icons=("32x32:32x32.png" "128x128:128x128.png" "256x256:128x128@2x.png" "512x512:icon.png")
minimum_rust="1.91"

keep=0
uninstall=0
assume_yes=0
for arg in "$@"; do
  case "$arg" in
    --keep-build) keep=1 ;;
    --uninstall) uninstall=1 ;;
    -y | --yes) assume_yes=1 ;;
    -h | --help)
      sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Unknown option: $arg (try --help)" >&2
      exit 2
      ;;
  esac
done

have() { command -v "$1" >/dev/null 2>&1; }

refresh_desktop_caches() {
  if have update-desktop-database; then
    update-desktop-database "$prefix/share/applications" >/dev/null 2>&1 || true
  fi
  if have gtk-update-icon-cache && [ -d "$prefix/share/icons/hicolor" ]; then
    gtk-update-icon-cache -q -f -t "$prefix/share/icons/hicolor" >/dev/null 2>&1 || true
  fi
}

if [ "$uninstall" -eq 1 ]; then
  rm -rf "$app_dir"
  rm -f "$bin_link" "$desktop_file"
  for entry in "${icons[@]}"; do
    rm -f "$prefix/share/icons/hicolor/${entry%%:*}/apps/wishtory.png"
  done
  refresh_desktop_caches
  echo "Wishtory is uninstalled."
  echo "Your saved history is still in ~/.local/share/app.wishtory.desktop; delete that folder to remove it too."
  exit 0
fi

# ---- what is missing? -------------------------------------------------------------------------------
if have pacman; then
  manager=pacman
elif have apt-get; then
  manager=apt
elif have dnf; then
  manager=dnf
else
  manager=""
fi

case "$manager" in
  pacman) packages=(base-devel git curl unzip xz zip clang cmake ninja gtk3 xdg-utils) ;;
  apt) packages=(build-essential git curl unzip xz-utils zip clang cmake ninja-build pkg-config libgtk-3-dev xdg-utils) ;;
  dnf) packages=(gcc gcc-c++ make git curl unzip xz zip clang cmake ninja-build pkgconf-pkg-config gtk3-devel xdg-utils) ;;
  *) packages=() ;;
esac

missing_tools=()
for tool in cc git curl unzip xz zip clang cmake ninja pkg-config; do
  have "$tool" || missing_tools+=("$tool")
done
if have pkg-config && ! pkg-config --exists gtk+-3.0; then
  missing_tools+=("the GTK 3 development files")
fi

# Rust: a system one is used when it is new enough (or when rustup manages versions), otherwise it is
# installed through pacman on Arch, or downloaded to a temporary folder.
version_at_least() { [ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -n 1)" = "$2" ]; }
rust_source=system
if have rustup; then
  rust_source=rustup
elif have cargo && have rustc && version_at_least "$(rustc --version | awk '{print $2}')" "$minimum_rust"; then
  rust_source=system
elif [ "$manager" = pacman ]; then
  rust_source=pacman
  packages+=(rust)
else
  rust_source=download
fi

flutter_source=system
have flutter || flutter_source=download

# ---- say what will happen, and ask once -------------------------------------------------------------------
plan=()
if [ "${#missing_tools[@]}" -gt 0 ] || [ "$rust_source" = pacman ]; then
  if [ -z "$manager" ]; then
    echo "Wishtory needs these tools, and this script does not know your package manager: ${missing_tools[*]}" >&2
    echo "Install them (and Rust ${minimum_rust}+), then run the script again. docs/linux.md lists package names." >&2
    exit 1
  fi
  plan+=("install build packages with sudo and $manager: ${packages[*]}")
fi
[ "$rust_source" = download ] && plan+=("download Rust $minimum_rust into a temporary folder (about 300 MB)")
[ "$flutter_source" = download ] && plan+=("download Flutter into a temporary folder (about 2 GB)")
plan+=("build Wishtory (several minutes)" "install it under $prefix" "delete everything it downloaded and built")

echo "To build Wishtory this script will:"
printf '  - %s\n' "${plan[@]}"
if [ "$assume_yes" -eq 0 ]; then
  if [ ! -t 0 ]; then
    echo "Run it from a terminal, or add --yes to continue without asking." >&2
    exit 1
  fi
  read -r -p "Continue? [Y/n] " reply
  case "$reply" in
    "" | [Yy]*) ;;
    *)
      echo "Nothing was changed."
      exit 0
      ;;
  esac
fi

# ---- from here on, clean up when done, whether the build worked or not -------------------------------------
if [ "$keep" -eq 1 ]; then
  work="$root/.wishtory-tools"
  mkdir -p "$work"
else
  work="$(mktemp -d)"
fi
cleanup() {
  if [ "$keep" -eq 0 ]; then
    echo "Cleaning up..."
    rm -rf "$work" "$root/target" "$root/app/build" "$root/app/.dart_tool" "$root/app/linux/flutter/ephemeral"
  else
    echo "Build files kept: $root/target, $root/app/build and $work"
  fi
}
trap cleanup EXIT

# ---- set up the tools --------------------------------------------------------------------------------------
if [ "${#missing_tools[@]}" -gt 0 ] || [ "$rust_source" = pacman ]; then
  yes_flag=()
  [ "$assume_yes" -eq 1 ] && yes_flag=(--noconfirm)
  as_root=()
  [ "$(id -u)" -ne 0 ] && as_root=(sudo)
  case "$manager" in
    pacman) "${as_root[@]}" pacman -S --needed "${yes_flag[@]}" "${packages[@]}" ;;
    apt) "${as_root[@]}" apt-get update && "${as_root[@]}" apt-get install -y "${packages[@]}" ;;
    dnf) "${as_root[@]}" dnf install -y "${packages[@]}" ;;
  esac
fi

channel="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$root/rust-toolchain.toml")"
if [ "$rust_source" = download ]; then
  export RUSTUP_HOME="$work/rustup" CARGO_HOME="$work/cargo"
  export PATH="$CARGO_HOME/bin:$PATH"
  if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
    echo "Downloading Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain none
  fi
fi
if [ "$rust_source" = download ] || [ "$rust_source" = rustup ]; then
  # The repository's rust-toolchain.toml picks this version while building here; the default toolchain
  # of an existing rustup is left alone.
  rustup toolchain install "$channel" --profile minimal
fi
if [ "$rust_source" = download ]; then
  rustup default "$channel" >/dev/null
fi

if [ "$flutter_source" = download ]; then
  export PUB_CACHE="$work/pub-cache"
  if [ ! -x "$work/flutter/bin/flutter" ]; then
    echo "Downloading Flutter..."
    pin="$(grep -v '^#' "$root/flutter-toolchain.txt" | head -n 1 | tr -d '[:space:]')"
    tag="${pin%.x}"
    if [ "$tag" != "$pin" ]; then
      # "3.47.x" means the newest 3.47 release.
      tag="$(git ls-remote --tags --refs https://github.com/flutter/flutter.git "$tag.*" |
        sed 's#.*refs/tags/##' | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -n 1 || true)"
    fi
    git clone --quiet --depth 1 --branch "${tag:-stable}" https://github.com/flutter/flutter.git "$work/flutter"
  fi
  export PATH="$work/flutter/bin:$PATH"
  flutter --suppress-analytics config --enable-linux-desktop >/dev/null
fi

# ---- build ---------------------------------------------------------------------------------------------
cd "$root"
echo "Building the Rust core..."
cargo build --locked -p wishtory-bridge --release
echo "Building the app..."
(cd app && flutter pub get --enforce-lockfile && flutter build linux --release)

bundle="$root/app/build/linux/x64/release/bundle"
for needed in "$bundle/wishtory" "$bundle/lib/libwishtory_bridge.so"; do
  if [ ! -e "$needed" ]; then
    echo "The build did not produce $needed." >&2
    exit 1
  fi
done

# ---- install -------------------------------------------------------------------------------------------
echo "Installing to $prefix ..."
rm -rf "$app_dir"
mkdir -p "$app_dir" "$prefix/bin" "$prefix/share/applications"
cp -a "$bundle/." "$app_dir/"
ln -sf "$app_dir/wishtory" "$bin_link"
# The launcher entry points at the installed program by full path, so it works even when ~/.local/bin
# is not on the desktop's PATH.
sed "s|^Exec=.*|Exec=$app_dir/wishtory|" "$root/packaging/linux/wishtory.desktop" >"$desktop_file"
chmod 644 "$desktop_file"
for entry in "${icons[@]}"; do
  install -Dm644 "$root/assets/icons/${entry#*:}" "$prefix/share/icons/hicolor/${entry%%:*}/apps/wishtory.png"
done
refresh_desktop_caches

echo
echo "Wishtory is installed. Start it from your application menu, or run: wishtory"
case ":$PATH:" in
  *":$prefix/bin:"*) ;;
  *) echo "Note: $prefix/bin is not on your PATH, so the 'wishtory' command needs the full path: $bin_link" ;;
esac
