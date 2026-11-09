# Building the Flutter app

The Rust core and the Dart interface are built separately; the interface loads the Rust library at
start-up. Node is only used by `scripts/` for the signed catalog, never to build the app.

## One-time setup

1. **Rust**, the version in `rust-toolchain.toml` (`rustup` picks it up automatically).
2. **Flutter** (stable) with desktop support, the version in `flutter-toolchain.txt`
   (`flutter config --enable-linux-desktop --enable-windows-desktop`).
   * Windows: Visual Studio 2022 (or its Build Tools) with the "Desktop development with C++" workload.
   * Debian / Ubuntu: `sudo apt install clang cmake ninja-build pkg-config libgtk-3-dev xdg-utils`
   * Arch / CachyOS: `sudo pacman -S --needed base-devel git rust clang cmake ninja gtk3 xdg-utils`, plus
     Flutter from the AUR (`flutter`) or your own install.
3. **The code generator**, only needed when you change the Rust API in `crates/wishtory-bridge/src/api`
   (it must be exactly the version the Rust crate uses):
   ```
   cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
   ```

## Every build

From the repository root:

```
# 1. Only after changing crates/wishtory-bridge/src/api: regenerate the Dart <-> Rust glue.
flutter_rust_bridge_codegen generate

# 2. Build the Rust library. The Flutter build copies it into the app bundle.
cargo build -p wishtory-bridge --release

# 3. Check, test, run.
cd app
flutter pub get
flutter analyze
flutter test
flutter run -d linux        # or: -d windows

# Release bundle (app/build/linux/x64/release/bundle, app/build/windows/x64/runner/Release):
flutter build linux --release   # or: flutter build windows --release
```

The generated files (`app/lib/bridge/generated/` and `crates/wishtory-bridge/src/frb_generated.rs`) are
committed. CI regenerates them and fails if they differ from what is committed.

`lib/main.dart` looks for the Rust library next to the executable, then in `lib/` beside it (a built
bundle), then in `target/release` and `target/debug` when running from source. Set
`WISHTORY_NATIVE_LIB=/path/to/the/library` to use another one.

## Rust-only checks (no Flutter needed)

```
cargo fmt --all --check
cargo test --locked --workspace
```

## Installing and packaging

* **Linux:** `bash packaging/linux/install.sh` does the whole build above, installs the app under `~/.local`
  and deletes the build output afterwards (`--keep-build` keeps it, `--uninstall` removes the app). Run it with
  `bash`, so it does not depend on the file's executable bit. `packaging/arch/` holds two PKGBUILDs for
  `makepkg`. Releases carry no Linux download.
* **Windows:** `packaging/windows/wishtory.iss` is the Inno Setup script for the installer; the release
  workflow runs it. See [Releasing](../releasing.md).

## Upgrading flutter_rust_bridge

The Rust crate (`crates/wishtory-bridge/Cargo.toml`), the Dart package (`app/pubspec.yaml`) and the code
generator must be the same version, so Dependabot is told to leave it alone. To upgrade, change all
three, run `flutter_rust_bridge_codegen generate`, and commit the regenerated files with them.
