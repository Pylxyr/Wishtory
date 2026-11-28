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
# and dnf are supported), download Rust and Node.js (when your system has none, or an old one) into a
# temporary folder, build, and install under ~/.local (or $PREFIX). Your saved history is not touched:
# it lives in ~/.local/share/app.wishtory.desktop.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
prefix="${PREFIX:-$HOME/.local}"
# The prefix ends up in the launcher entry and in rm/mv commands: keep to plain path characters.
case "$prefix" in
  /*) ;;
  *) echo "PREFIX must be an absolute path." >&2; exit 1 ;;
esac
if printf '%s' "$prefix" | LC_ALL=C grep -q '[^A-Za-z0-9_./+ -]'; then
  echo "PREFIX may only contain letters, digits, spaces and . _ / + - (got: $prefix)." >&2
  exit 1
fi
app_dir="$prefix/share/wishtory"
bin_link="$prefix/bin/wishtory"
desktop_file="$prefix/share/applications/wishtory.desktop"
# icon folder : file in src-tauri/icons
icons=("32x32:32x32.png" "128x128:128x128.png" "256x256:128x128@2x.png" "512x512:icon.png")
minimum_rust="1.100"
# The front end is built with Vite, which needs a current Node.js. 26 is what CI builds with.
minimum_node="22"
node_major="26"

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
  pacman) packages=(base-devel curl xz webkit2gtk-4.1 gtk3 xdg-utils) ;;
  apt) packages=(build-essential curl xz-utils pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev libssl-dev xdg-utils) ;;
  dnf) packages=(gcc gcc-c++ make curl xz pkgconf-pkg-config webkit2gtk4.1-devel gtk3-devel libsoup3-devel librsvg2-devel openssl-devel xdg-utils) ;;
  *) packages=() ;;
esac

missing_tools=()
for tool in cc curl xz tar pkg-config; do
  have "$tool" || missing_tools+=("$tool")
done
if have pkg-config && ! pkg-config --exists webkit2gtk-4.1; then
  missing_tools+=("the WebKitGTK 4.1 development files")
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

# Node.js: a system one is used when it is new enough, otherwise it is installed through pacman on Arch, or
# downloaded (and checked against the checksum nodejs.org publishes) to a temporary folder.
node_source=system
if have node && have npm && version_at_least "$(node --version | sed 's/^v//')" "$minimum_node"; then
  node_source=system
elif [ "$manager" = pacman ]; then
  node_source=pacman
  packages+=(nodejs npm)
else
  node_source=download
fi

# ---- say what will happen, and ask once -------------------------------------------------------------------
plan=()
if [ "${#missing_tools[@]}" -gt 0 ] || [ "$rust_source" = pacman ] || [ "$node_source" = pacman ]; then
  if [ -z "$manager" ]; then
    echo "Wishtory needs these tools, and this script does not know your package manager: ${missing_tools[*]}" >&2
    echo "Install them (and Rust ${minimum_rust}+ and Node.js ${minimum_node}+), then run the script again. docs/linux.md lists package names." >&2
    exit 1
  fi
  plan+=("install build packages with sudo and $manager: ${packages[*]}")
fi
[ "$rust_source" = download ] && plan+=("download Rust $minimum_rust into a temporary folder (about 300 MB)")
[ "$node_source" = download ] && plan+=("download Node.js $node_major into a temporary folder (about 30 MB)")
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
    rm -rf "$work" "$root/target" "$root/node_modules" "$root/dist"
  else
    echo "Build files kept: $root/target, $root/node_modules and $work"
  fi
}
trap cleanup EXIT

# ---- set up the tools --------------------------------------------------------------------------------------
if [ "${#missing_tools[@]}" -gt 0 ] || [ "$rust_source" = pacman ] || [ "$node_source" = pacman ]; then
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

if [ "$node_source" = download ]; then
  if [ ! -x "$work/node/bin/node" ]; then
    echo "Downloading Node.js..."
    base="https://nodejs.org/dist/latest-v${node_major}.x"
    sums="$(curl --proto '=https' --tlsv1.2 -fsSL "$base/SHASUMS256.txt")"
    line="$(printf '%s\n' "$sums" | grep -E ' node-v[0-9.]+-linux-x64\.tar\.xz$' | head -n 1 || true)"
    if [ -z "$line" ]; then
      echo "Could not find a Node.js $node_major download for this computer." >&2
      exit 1
    fi
    file="${line##* }"
    want="${line%% *}"
    curl --proto '=https' --tlsv1.2 -fsSL -o "$work/$file" "$base/$file"
    got="$(sha256sum "$work/$file" | awk '{print $1}')"
    if [ "$got" != "$want" ]; then
      echo "The Node.js download does not match its published checksum; not using it." >&2
      exit 1
    fi
    mkdir -p "$work/node"
    tar -xJf "$work/$file" -C "$work/node" --strip-components=1
  fi
  export PATH="$work/node/bin:$PATH"
fi

# ---- build ---------------------------------------------------------------------------------------------
cd "$root"
echo "Installing the front-end packages..."
npm ci --no-audit --no-fund
echo "Building the app (several minutes)..."
# --no-bundle: this script installs the program itself, so no .deb or AppImage is made.
npm run tauri -- build --no-bundle

binary="$root/target/release/wishtory"
if [ ! -x "$binary" ]; then
  echo "The build did not produce $binary." >&2
  exit 1
fi

# ---- install -------------------------------------------------------------------------------------------
echo "Installing to $prefix ..."
# Copy next to the old install and swap, so a failed copy never leaves the app half removed.
mkdir -p "$prefix/bin" "$prefix/share/applications"
rm -rf "$app_dir.new" "$app_dir.old"
mkdir -p "$app_dir.new"
install -m755 "$binary" "$app_dir.new/wishtory"
if [ -e "$app_dir" ]; then mv "$app_dir" "$app_dir.old"; fi
mv "$app_dir.new" "$app_dir"
rm -rf "$app_dir.old"
ln -sf "$app_dir/wishtory" "$bin_link"
# The launcher entry points at the installed program by full path, so it works even when ~/.local/bin
# is not on the desktop's PATH.
# The path is quoted (it may contain spaces) and passed to awk as data, never parsed as a pattern.
awk -v exec_line="Exec=\"$app_dir/wishtory\"" '/^Exec=/ { print exec_line; next } { print }' \
  "$root/packaging/linux/wishtory.desktop" >"$desktop_file"
chmod 644 "$desktop_file"
for entry in "${icons[@]}"; do
  install -Dm644 "$root/src-tauri/icons/${entry#*:}" "$prefix/share/icons/hicolor/${entry%%:*}/apps/wishtory.png"
done
refresh_desktop_caches

echo
echo "Wishtory is installed. Start it from your application menu, or run: wishtory"
case ":$PATH:" in
  *":$prefix/bin:"*) ;;
  *) echo "Note: $prefix/bin is not on your PATH, so the 'wishtory' command needs the full path: $bin_link" ;;
esac
