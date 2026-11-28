# Building the app

Wishtory is a Tauri 2 app: the Rust core (`crates/wishtory-core`) is wrapped by a typed command layer
(`crates/wishtory-api`), exposed to the window by the Tauri shell (`src-tauri`), and the window itself is a
Svelte 5 + TypeScript app built with Vite (`src`). Node is also used by `scripts/` for the signed catalog.

## One-time setup

1. **Rust**, the version in `rust-toolchain.toml` (`rustup` picks it up automatically).
2. **Node.js** 22 or newer (CI builds with 26), then `npm ci` from the repository root.
3. **The platform's web view and compiler:**
   * Windows: Visual Studio 2022 (or its Build Tools) with the "Desktop development with C++" workload. The
     WebView2 runtime ships with Windows 10 and 11.
   * Debian / Ubuntu: `sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev libssl-dev xdg-utils`
   * Arch / CachyOS: `sudo pacman -S --needed base-devel rust nodejs npm webkit2gtk-4.1 gtk3 xdg-utils`
   * On Linux you can skip all of this: `bash packaging/linux/install.sh` sets up what it needs and cleans up.

## Every build

From the repository root:

```
npm run tauri -- dev                    # the real app, with hot reload for the window
VITE_MOCK=1 npm run dev                 # only the window, in a browser, on made-up data (no Rust needed)

npm run check                           # svelte-check: types and accessibility hints
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked
cargo test --locked --workspace
node --test scripts/*.test.mjs          # the catalog and release tooling
python3 -m unittest discover -s scripts -p "test_*.py"
```

`VITE_MOCK=1` swaps the Rust commands for `src/lib/mock.ts`, which answers with the real catalog and a made-up
history. It is what `e2e/` drives with Playwright. The mock is not the backend: keep its answers the same shape
as `crates/wishtory-api/src/api/types.rs`.

## Release builds

```
npm run tauri -- build                  # Windows: the NSIS installer in target/release/bundle/nsis
npm run tauri -- build --no-bundle      # Linux: the program alone, target/release/wishtory
```

## Installing and packaging

* **Linux:** `bash packaging/linux/install.sh` asks once, installs the build packages with the system package
  manager (pacman, apt or dnf), downloads Node.js (checked against nodejs.org's published checksum) and Rust when the
  system has none or an old one into a temporary folder, runs the whole build above, installs the app under
  `~/.local` and deletes everything it downloaded and built. `--yes` skips the question, `--keep-build` keeps the
  build output and the downloaded tools (in `.wishtory-tools/`), `--uninstall` removes the app. Run it with
  `bash`, so it does not depend on the file's executable bit. `packaging/arch/` holds two PKGBUILDs for
  `makepkg`. Releases carry no Linux download.
* **Windows:** the installer is built by `tauri build` from `src-tauri/tauri.conf.json`; the release workflow
  also zips the program. See [Releasing](../releasing.md).
