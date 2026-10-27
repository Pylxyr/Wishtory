# Releasing

Releases are built by GitHub Actions and contain only the NSIS setup installer (`Wishtory_<version>_x64-setup.exe`) and a SHA256 checksum file.

## One click

1. Open **Actions -> Cut release -> Run workflow** on `main`, pick patch, minor or major (or type an exact version such as `0.2.0` or `0.2.0-rc1`; a `-` makes it a pre-release) and run it.

The workflow runs CI on `main` first; if anything fails, nothing is bumped or tagged. It then sets the new version in `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`, commits `Release X.Y.Z`, pushes the commit and the tag `vX.Y.Z` in one atomic push, and calls the **Release** workflow, which builds on `windows-latest`, creates the GitHub Release and attaches the setup exe and its `.sha256`. A final job fails the run if no published release exists for the tag. The installer is not code-signed, so SmartScreen may warn on first run.

## Repository settings it relies on

- **Settings -> Actions -> General -> Workflow permissions** set to *Read and write*.
- If `main` is protected, a bypass for `github-actions[bot]`.

**Actions -> Release -> Run workflow** (or pushing a tag such as `v0.2.0`) still builds the version already in the files; the tag must match it. `python scripts/bump_version.py 0.2.0` sets the version by hand, `--check v0.2.0` verifies it.

## Catalog

The item catalog and banner list are not part of an app release. See [Game catalog updates](game-catalog-updates.md).
