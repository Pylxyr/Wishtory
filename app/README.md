# Wishtory app

The Flutter interface of Wishtory. The logic (reading the game's files, downloading history, pity maths, storage) is
in the Rust crates under `../crates`, and this app calls it through `flutter_rust_bridge`.

- `lib/features/` one folder per screen
- `lib/bridge/` the Rust API as the app sees it (`generated/` is machine-written)
- `lib/domain/`, `lib/state/`, `lib/ui/` game terms, app state, shared widgets and design tokens
- `test/` unit tests for the history and planning logic
- `windows/` and `linux/` the platform runners

To build and run it, see [docs/redesign/BUILDING.md](../docs/redesign/BUILDING.md). For using the app, see the
[README](../README.md).
