#!/usr/bin/env python3
"""Keeps the app version in step across every file that carries it.

python scripts/bump_version.py 0.2.0          set the version everywhere
python scripts/bump_version.py --check v0.2.0 fail unless every file equals the tag
python scripts/bump_version.py --next patch   print the next version (patch, minor or major)
python scripts/bump_version.py --current      print the current version

Files: package.json, package-lock.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml and the
`wishtory` entry of src-tauri/Cargo.lock. Text is edited in place, so formatting and line endings
stay as they are. Standard library only.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKAGE = ROOT / "package.json"
PACKAGE_LOCK = ROOT / "package-lock.json"
TAURI_CONF = ROOT / "src-tauri" / "tauri.conf.json"
CARGO_TOML = ROOT / "src-tauri" / "Cargo.toml"
CARGO_LOCK = ROOT / "src-tauri" / "Cargo.lock"

SEMVER = re.compile(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$")
NL = r"\r?\n"
# (path, label, pattern with three groups: before, version, after, expected number of matches)
TARGETS = [
    (PACKAGE, "package.json", re.compile(r'(^\s*"version": ")([^"]*)(")', re.M), 1),
    (PACKAGE_LOCK, "package-lock.json", re.compile(r'(^\s*"name": "wishtory",' + NL + r'\s*"version": ")([^"]*)(")', re.M), 2),
    (TAURI_CONF, "src-tauri/tauri.conf.json", re.compile(r'(^\s*"version": ")([^"]*)(")', re.M), 1),
    (CARGO_TOML, "src-tauri/Cargo.toml", re.compile(r'(\[package\][^\[]*?^version = ")([^"]*)(")', re.M | re.S), 1),
    (CARGO_LOCK, "src-tauri/Cargo.lock", re.compile(r'(^name = "wishtory"' + NL + r'version = ")([^"]*)(")', re.M), 1),
]


def _read(path: Path) -> str:
    # newline="" keeps the file's own line endings.
    with path.open("r", encoding="utf-8", newline="") as handle:
        return handle.read()


def _write(path: Path, text: str) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        handle.write(text)


def read_versions() -> dict[str, str | None]:
    """{label: version} for every place that carries the version."""
    found: dict[str, str | None] = {}
    for path, label, pattern, expected in TARGETS:
        matches = pattern.findall(_read(path))
        if len(matches) != expected:
            found[label] = None
            continue
        versions = {m[1] for m in matches}
        found[label] = versions.pop() if len(versions) == 1 else None
    return found


def current_version() -> str:
    version = read_versions()["package.json"]
    if version is None:
        raise ValueError("version not found in package.json")
    return version


def next_version(current: str, kind: str) -> str:
    """The version after `current`. A pre-release suffix is dropped first (1.2.0-rc1 + patch = 1.2.1);
    to promote a release candidate to 1.2.0, give the exact version instead."""
    if not SEMVER.match(current):
        raise ValueError(f"'{current}' is not MAJOR.MINOR.PATCH[-suffix].")
    major, minor, patch = (int(part) for part in current.split("-", 1)[0].split("."))
    if kind == "major":
        return f"{major + 1}.0.0"
    if kind == "minor":
        return f"{major}.{minor + 1}.0"
    if kind == "patch":
        return f"{major}.{minor}.{patch + 1}"
    raise ValueError(f"Unknown bump kind '{kind}' (use patch, minor or major).")


def check(tag: str) -> int:
    expected = tag.removeprefix("v")
    if not SEMVER.match(expected):
        print(f"'{tag}' is not a vMAJOR.MINOR.PATCH[-suffix] tag.", file=sys.stderr)
        return 1
    versions = read_versions()
    for label, found in versions.items():
        print(f"{'ok  ' if found == expected else 'FAIL'} {label}: {found}")
    if any(found != expected for found in versions.values()):
        print(
            f"\nThe tag says {expected} but the files above disagree. Run: python scripts/bump_version.py {expected}",
            file=sys.stderr,
        )
        return 1
    return 0


def bump(version: str) -> int:
    if not SEMVER.match(version):
        print(f"'{version}' is not MAJOR.MINOR.PATCH (optionally -rc1 etc.).", file=sys.stderr)
        return 1
    edits = []
    for path, label, pattern, expected in TARGETS:
        text, count = pattern.subn(lambda m: f"{m.group(1)}{version}{m.group(3)}", _read(path))
        if count != expected:
            print(f"{label}: expected {expected} version entries, found {count}. Nothing was changed.", file=sys.stderr)
            return 1
        edits.append((path, text))
    for path, text in edits:
        _write(path, text)
    print(f"Version set to {version} in package.json, package-lock.json, tauri.conf.json, Cargo.toml and Cargo.lock.")
    return 0


def main(argv: list[str]) -> int:
    try:
        if len(argv) == 3 and argv[1] == "--check":
            return check(argv[2])
        if len(argv) == 3 and argv[1] == "--next":
            print(next_version(current_version(), argv[2]))
            return 0
        if len(argv) == 2 and argv[1] == "--current":
            print(current_version())
            return 0
        if len(argv) == 2 and not argv[1].startswith("-"):
            return bump(argv[1].removeprefix("v"))
    except ValueError as exc:
        print(exc, file=sys.stderr)
        return 1
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
