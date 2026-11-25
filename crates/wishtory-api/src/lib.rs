//! What the Wishtory UI can call. Everything the Svelte front end reaches lives in `api`; the types
//! there are plain data that serialise to JSON for the Tauri shell (no core internals).

pub mod api;
mod convert;
