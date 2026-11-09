#!/usr/bin/env bash
# Builds Wishtory from this source folder, installs it for the current user, and then deletes the
# build output (target/ and app/build/, several GB) so nothing is left behind but the installed app.
#
#   bash packaging/linux/install.sh                 build, install, clean up
#   bash packaging/linux/install.sh --keep-build    the same, but keep the build output (faster rebuilds)
#   bash packaging/linux/install.sh --uninstall     remove the installed app
#
# It installs under ~/.local (or $PREFIX) and never needs root. Your saved history is not touched:
# it lives in ~/.local/share/app.wishtory.desktop.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
prefix="${PREFIX:-$HOME/.local}"
app_dir="$prefix/share/wishtory"
bin_link="$prefix/bin/wishtory"
desktop_file="$prefix/share/applications/wishtory.desktop"
# icon folder : file in assets/icons
icons=("32x32:32x32.png" "128x128:128x128.png" "256x256:128x128@2x.png" "512x512:icon.png")

keep=0
uninstall=0
for arg in "$@"; do
  case "$arg" in
    --keep-build) keep=1 ;;
    --uninstall) uninstall=1 ;;
    -h | --help)
      sed -n '2,10p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Unknown option: $arg (try --help)" >&2
      exit 2
      ;;
  esac
done

refresh_desktop_caches() {
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$prefix/share/applications" >/dev/null 2>&1 || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1 && [ -d "$prefix/share/icons/hicolor" ]; then
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

# ---- tools -----------------------------------------------------------------------------------
missing=()
for tool in cargo flutter cmake ninja clang pkg-config; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if command -v pkg-config >/dev/null 2>&1 && ! pkg-config --exists gtk+-3.0; then
  missing+=("the GTK 3 development files")
fi
if [ "${#missing[@]}" -gt 0 ]; then
  echo "Missing: ${missing[*]}" >&2
  echo "On Arch and CachyOS: sudo pacman -S --needed base-devel git rust clang cmake ninja gtk3 xdg-utils" >&2
  echo "plus Flutter (AUR package 'flutter'). Details are in the README." >&2
  exit 1
fi

# ---- clean up when done, whether the build worked or not ------------------------------------------
cleanup() {
  if [ "$keep" -eq 0 ]; then
    echo "Cleaning up build files..."
    rm -rf "$root/target" "$root/app/build" "$root/app/.dart_tool" "$root/app/linux/flutter/ephemeral"
  fi
}
trap cleanup EXIT

# ---- build ---------------------------------------------------------------------------------------
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

# ---- install ---------------------------------------------------------------------------------------
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
