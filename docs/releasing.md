# Releasing

Releases are built by GitHub Actions and contain the Windows setup installer (`Wishtory_<version>_x64-setup.exe`, built with Inno Setup from `packaging/windows/wishtory.iss`) and a portable zip, the Linux packages (`.tar.gz`, `.deb` and `.rpm`, made by `packaging/linux/package.sh`) and `SHA256SUMS.txt`. When the repository allows it, the installer, the zip and the Linux packages also carry signed build provenance.

## One click

1. Open **Actions -> Cut release -> Run workflow** on `main`, pick patch, minor or major (or type an exact version such as `0.2.0` or `0.2.0-rc1`; a `-` makes it a pre-release) and run it.

The workflow runs CI on `main` first; if anything fails, nothing is bumped or tagged. It then sets the new version in `package.json`, `app/pubspec.yaml` (the `+build` number stays), the `[workspace.package]` section of `Cargo.toml` (both Rust crates inherit it) and `Cargo.lock`, commits `Release X.Y.Z`, pushes the commit and the tag `vX.Y.Z` in one atomic push, and calls the **Release** workflow. That builds the Rust library and the Flutter app, then the installer and zip on `windows-latest` and, at the same time, the Linux packages on `ubuntu-24.04` (the same runner image as the Linux CI job); nothing is published unless both succeed. Then a final job writes the checksums, attests the build, writes the release notes and publishes the GitHub Release. A final job fails the run if no published release exists for the tag. The Rust version is the one in `rust-toolchain.toml` and the Flutter version the one in `flutter-toolchain.txt` (the workflows read both from there), the Rust library and the Dart packages are built from their lock files (`--locked`, `--enforce-lockfile`), and the bundled catalog, banner list and rules are stamped with the versions currently published (see [Game catalog updates](game-catalog-updates.md)). The Windows installer is not code-signed, so SmartScreen may warn on first run. The Linux packages are built on Ubuntu 24.04, so they need a recent distribution (glibc 2.39 or newer).

## Release notes

`scripts/release_notes.py` writes the changelog from the commits since the previous release, after the install text in `.github/release-notes-header.md`. It tries, in order:

1. An AI summary of the real diff (reduced to plain text: links, images, markup and @mentions are removed from every line, because the notes are published without a review step), using the optional secret `ANTHROPIC_API_KEY`, or `AI_API_KEY` with the variables `AI_BASE_URL` and `AI_MODEL` (any OpenAI-compatible service), or GitHub Models with the built-in token (free).
2. Commit messages grouped by their first word (`Add` -> Added, `Fix` -> Fixed; `feat:` and `fix:` prefixes work too).
3. For commits whose message says nothing, a read of the code: which parts changed, new or removed functions of the Rust API the interface calls, new or removed public Rust items and new screens.

With no AI configured the notes still come out, from layers 2 and 3. The first stable release after only pre-releases is compared with the newest pre-release, so its notes are never empty. Edit the header file to change the install text. The script has unit tests (`python3 -m unittest discover -s scripts -p "test_*.py"`), and CI also lints every workflow with `actionlint`.

## Repository settings it relies on

- **Settings -> Actions -> General -> Workflow permissions** set to *Read and write*.
- If `main` is protected, a bypass for `github-actions[bot]`.
- Build provenance needs a public repository, or GitHub Enterprise Cloud for a private one. If it is unavailable the release is still published, without it.

## Other ways in

- Pushing a tag such as `v0.2.0` builds and publishes the version already in the files; the tag must match it.
- **Actions -> Release -> Run workflow** is a dry run: it builds the installer, the zip and the Linux packages, uploads them as workflow artifacts, and publishes nothing.
- `python scripts/bump_version.py 0.3.0` sets the version by hand, `--check v0.3.0` verifies it.

## Dependencies

Dependabot opens weekly PRs for GitHub Actions, the Rust crates and the Dart packages. `flutter_rust_bridge` is left out on purpose: the Rust crate, the Dart package and the code generator must all be the same version, so it is bumped by hand ([BUILDING.md](redesign/BUILDING.md#upgrading-flutter_rust_bridge)). CI runs on every PR like on any other change.

## Catalog

The item catalog, banner list and pity rules are not part of an app release, and their versions are set automatically when they are published. See [Game catalog updates](game-catalog-updates.md).
