#!/usr/bin/env bash
# Packs the Flutter Linux bundle into the files a release carries:
#   Wishtory_<version>_linux_x64.tar.gz   portable, runs from any folder
#   Wishtory_<version>_amd64.deb          Debian, Ubuntu and derivatives
#   Wishtory-<version>-1.x86_64.rpm       Fedora, openSUSE and derivatives
#
#   packaging/linux/package.sh <version> <bundle folder> <output folder>
#
# <bundle folder> is app/build/linux/x64/release/bundle after `flutter build linux --release`.
# The .deb and .rpm need `fpm` (and `rpm` for the .rpm); the .tar.gz needs nothing extra.
set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "usage: $0 <version> <bundle folder> <output folder>" >&2
  exit 2
fi
version="$1"
bundle="$(cd "$2" && pwd)"
mkdir -p "$3"
out="$(cd "$3" && pwd)"
root="$(cd "$(dirname "$0")/../.." && pwd)"

for needed in "$bundle/wishtory" "$bundle/lib/libwishtory_bridge.so" "$bundle/data"; do
  if [ ! -e "$needed" ]; then
    echo "error: $needed is missing from the bundle. Build the Rust library, then run flutter build linux --release." >&2
    exit 1
  fi
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# ---- portable tarball ---------------------------------------------------------------------------
folder="Wishtory_${version}_linux_x64"
mkdir -p "$work/tar/$folder"
cp -a "$bundle/." "$work/tar/$folder/"
install -Dm644 "$root/packaging/linux/wishtory.desktop" "$work/tar/$folder/wishtory.desktop"
install -Dm644 "$root/assets/icons/icon.png" "$work/tar/$folder/wishtory.png"
install -Dm644 "$root/LICENSE" "$work/tar/$folder/LICENSE"
tar -C "$work/tar" --owner=0 --group=0 -czf "$out/$folder.tar.gz" "$folder"

# ---- the tree the .deb and .rpm install ---------------------------------------------------------
tree="$work/tree"
install -d "$tree/opt/wishtory" "$tree/usr/bin"
cp -a "$bundle/." "$tree/opt/wishtory/"
ln -s /opt/wishtory/wishtory "$tree/usr/bin/wishtory"
install -Dm644 "$root/packaging/linux/wishtory.desktop" "$tree/usr/share/applications/wishtory.desktop"
install -Dm644 "$root/assets/icons/32x32.png" "$tree/usr/share/icons/hicolor/32x32/apps/wishtory.png"
install -Dm644 "$root/assets/icons/128x128.png" "$tree/usr/share/icons/hicolor/128x128/apps/wishtory.png"
install -Dm644 "$root/assets/icons/128x128@2x.png" "$tree/usr/share/icons/hicolor/256x256/apps/wishtory.png"
install -Dm644 "$root/assets/icons/icon.png" "$tree/usr/share/icons/hicolor/512x512/apps/wishtory.png"
install -Dm644 "$root/LICENSE" "$tree/usr/share/licenses/wishtory/LICENSE"

# A pre-release such as 0.3.0-rc1 sorts before 0.3.0 only when written with a tilde.
package_version="${version/-/\~}"
common=(
  -s dir -C "$tree"
  --name wishtory
  --version "$package_version"
  --license MIT
  --url "https://github.com/Pylxyr/Wishtory"
  --maintainer "Wishtory contributors"
  --description "Local wish history manager for Genshin Impact, Honkai: Star Rail and Zenless Zone Zero"
  --category utils
)

fpm "${common[@]}" -t deb --architecture amd64 \
  --depends xdg-utils --depends "libgtk-3-0 | libgtk-3-0t64" \
  --package "$out/Wishtory_${version}_amd64.deb" \
  opt usr

fpm "${common[@]}" -t rpm --architecture x86_64 \
  --depends xdg-utils --depends gtk3 \
  --package "$out/Wishtory-${version}-1.x86_64.rpm" \
  opt usr

ls -l "$out"
