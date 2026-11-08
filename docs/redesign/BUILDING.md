# Building the Flutter app

The Rust core and the Dart UI are built separately; the UI loads the Rust library at start-up.
Nothing here needs Tauri or Node (Node is only used by `scripts/` for the signed catalog).

## One-time setup

1. **Rust** 1.91 or newer (`rust-toolchain.toml` pins it; `rustup` picks it up automatically).
2. **Flutter** stable with desktop support enabled (`flutter config --enable-linux-desktop`,
   `--enable-windows-desktop`).
   * Linux: `sudo apt install clang cmake ninja-build pkg-config libgtk-3-dev` (or the equivalent).
   * Windows: Visual Studio 2022 with the "Desktop development with C++" workload.
3. The code generator, **exactly the version the Rust crate uses**:
   ```
   cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
   ```

## Create the platform runners and dependencies (once)

From the repository root:

```
cd app
flutter create --platforms=linux,windows --project-name wishtory --org app.wishtory .
git checkout -- lib test            # keep our lib/ and test/ if flutter create touched them
rm -f test/widget_test.dart         # the template's sample test does not apply
flutter pub add flutter_riverpod shared_preferences file_selector window_manager flutter_rust_bridge:2.13.0
```

`flutter pub add` resolves the latest stable versions of each package and writes them to `pubspec.yaml`.

### Bundle the Rust library with the app

Append to `app/linux/CMakeLists.txt` (before the final `install(CODE ...)` / at the end is fine):

```cmake
# Wishtory native core (built by `cargo build -p wishtory-bridge --release`)
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/libwishtory_bridge.so"
        DESTINATION "${INSTALL_BUNDLE_LIB_DIR}" COMPONENT Runtime)
```

Append to `app/windows/CMakeLists.txt`:

```cmake
# Wishtory native core (built by `cargo build -p wishtory-bridge --release`)
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release/wishtory_bridge.dll"
        DESTINATION "${CMAKE_INSTALL_PREFIX}" COMPONENT Runtime)
```

(`lib/main.dart` looks for the library next to the executable, then in `lib/`, then in
`target/release` or `target/debug` when running from source. `WISHTORY_NATIVE_LIB=/path/to/lib` overrides.)

## Every build

```
# 1. generate the Dart <-> Rust glue (repository root). Re-run whenever crates/wishtory-bridge/src/api changes.
flutter_rust_bridge_codegen generate

# 2. build the Rust library
cargo build -p wishtory-bridge --release

# 3. check, test and run the app
cd app
flutter analyze
flutter test
flutter run -d linux        # or: -d windows
# release bundle:
flutter build linux --release   # or: flutter build windows --release
```

Commit the generated files in `app/lib/bridge/generated/` and `crates/wishtory-bridge/src/frb_generated.rs`
(the `frb_generated.rs` in the repository is only a placeholder so the crate type-checks before the first run).

## Rust-only checks (no Flutter needed)

```
cargo test -p wishtory-core --lib     # 133 tests
cargo check -p wishtory-bridge
```

## What to paste back if something fails

* `flutter_rust_bridge_codegen generate` output (first error is enough).
* `flutter analyze` output (full; it is the main check that the Dart matches the generated API).
* For a crash at start-up: the "Wishtory couldn't start" screen text.

## Places most likely to need a one-line fix

I could not run Flutter in my environment, so these are the spots where the generated API or a
package's newest signature may differ from what I wrote. `flutter analyze` will point at them:

* `lib/main.dart`: `ExternalLibrary.open(path)`. If it is not found, use
  `ExternalLibrary(ffiDynamicLibrary: DynamicLibrary.open(path))` (import `dart:ffi`).
* `lib/bridge/wishtory_api.dart`: parameter names of generated calls (they follow the Rust names in
  camelCase, e.g. `refreshFromGame(game:, full:)`).
* `lib/features/import_export/import_export_page.dart`: `getSaveLocation` / `RadioGroup` need a recent
  `file_selector` and Flutter 3.35+.
* Riverpod 3 is assumed (`ref.mounted`). With 2.x, remove those two guards.
