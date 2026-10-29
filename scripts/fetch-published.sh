#!/usr/bin/env bash
# Downloads the signed feeds that the rolling `catalog-v1` release publishes now into <dir>
# (<name>.envelope.json for catalog, banners and rules) and lists every asset name in
# <dir>/assets.txt. Needs the GitHub CLI with GH_TOKEN and GITHUB_REPOSITORY set.
#
# "The release does not exist yet" is the only case treated as nothing published. Any other
# failure (network, auth, rate limit) stops the caller: reading it as "first publish" would make
# the workflow publish a version number that may be lower than the live one, which every app
# ignores.
set -euo pipefail
dir="${1:?usage: fetch-published.sh <dir>}"
mkdir -p "$dir"
: > "$dir/assets.txt"
if ! out="$(gh api "repos/${GITHUB_REPOSITORY:?}/releases/tags/catalog-v1" --jq '.assets[].name' 2>&1)"; then
  if grep -q 'HTTP 404' <<<"$out"; then
    echo "The catalog release does not exist yet: nothing is published."
    exit 0
  fi
  echo "Could not read the catalog release: $out" >&2
  exit 1
fi
printf '%s\n' "$out" > "$dir/assets.txt"
for name in catalog banners rules; do
  if grep -qx "$name.envelope.json" "$dir/assets.txt"; then
    gh release download catalog-v1 --pattern "$name.envelope.json" --dir "$dir" --clobber
  fi
done
