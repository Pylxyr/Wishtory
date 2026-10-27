//! About screen data and the one place that may open a link in the browser.
//!
//! The credits live here so the About page, the list of links the app may open and the README
//! stay in step: only URLs listed below, and release pages of this project, can be opened.

use serde::Serialize;

pub const REPOSITORY: &str = "https://github.com/Pylxyr/Wishtory";
pub const RELEASES: &str = "https://github.com/Pylxyr/Wishtory/releases";
const ISSUES: &str = "https://github.com/Pylxyr/Wishtory/issues";

#[derive(Serialize)]
pub struct Credit {
    name: &'static str,
    role: &'static str,
    url: &'static str,
    /// Licence of software and fonts; empty for data sources and references.
    license: &'static str,
}

#[derive(Serialize)]
pub struct CreditGroup {
    title: &'static str,
    note: &'static str,
    credits: &'static [Credit],
}

#[derive(Serialize)]
pub struct About {
    name: &'static str,
    version: &'static str,
    description: &'static str,
    copyright: &'static str,
    license: &'static str,
    disclaimer: &'static str,
    repository: &'static str,
    releases: &'static str,
    issues: &'static str,
    groups: &'static [CreditGroup],
}

const fn credit(
    name: &'static str,
    role: &'static str,
    url: &'static str,
    license: &'static str,
) -> Credit {
    Credit {
        name,
        role,
        url,
        license,
    }
}

const GROUPS: &[CreditGroup] = &[
    CreditGroup {
        title: "Data and references",
        note: "Wishtory relies on these independent projects and references for compatibility and verification.",
        credits: &[
            credit(
                "EnkaNetwork API documentation and game stores",
                "Item IDs, names, rarities and character traits behind the updateable game catalog. Item icons load from enka.network.",
                "https://github.com/EnkaNetwork/API-docs/tree/master/store",
                "",
            ),
            credit(
                "Paimon.moe",
                "Open-source item data. Its exact item keys match backup records, its Genshin backup format is supported, and its documented read-only cache lookup informed history-link discovery.",
                "https://paimon.moe/",
                "",
            ),
            credit(
                "UIGF",
                "The interchange format used to move wish histories between compatible tools.",
                "https://uigf.org/",
                "",
            ),
            credit(
                "HoYoverse official announcements",
                "Used to check game terminology, mechanics and banner schedules.",
                "https://www.hoyolab.com/",
                "",
            ),
        ],
    },
    CreditGroup {
        title: "Banner schedule cross-checks",
        note: "Banner windows, titles and featured items were checked against HoYoverse's notices and these community trackers. They are references; Wishtory does not use them at run time.",
        credits: &[
            credit(
                "Banner History",
                "Genshin Impact phase and banner schedule.",
                "https://bannerhistory.app/",
                "",
            ),
            credit(
                "GachaTracker",
                "Genshin Impact banner archive with every banner's own title.",
                "https://gachatracker.app/",
                "",
            ),
            credit(
                "TimeSaver",
                "Zenless Zone Zero channel history, rerun and Rescreening windows.",
                "https://timesaver.gg/",
                "",
            ),
            credit(
                "Genshin Impact Wiki",
                "Event wish durations and server times.",
                "https://genshin-impact.fandom.com/",
                "",
            ),
            credit(
                "Zenless Zone Zero Wiki",
                "Exclusive Rescreening and Custom Search details.",
                "https://zenless-zone-zero.fandom.com/",
                "",
            ),
        ],
    },
    CreditGroup {
        title: "Built with",
        note: "Open-source software and fonts shipped inside Wishtory. Each project's repository has its full licence text.",
        credits: &[
            credit("Tauri", "Desktop app framework and file dialogs.", "https://tauri.app/", "Apache-2.0 OR MIT"),
            credit("Svelte", "Interface framework.", "https://svelte.dev/", "MIT"),
            credit("Vite", "Frontend build tooling.", "https://vite.dev/", "MIT"),
            credit(
                "rusqlite",
                "Local wish database. SQLite itself is in the public domain.",
                "https://github.com/rusqlite/rusqlite",
                "MIT",
            ),
            credit("reqwest", "Network requests.", "https://github.com/seanmonstar/reqwest", "MIT OR Apache-2.0"),
            credit("Tokio", "Asynchronous runtime.", "https://tokio.rs/", "MIT"),
            credit(
                "ring",
                "Signature check for the signed catalog.",
                "https://github.com/briansmith/ring",
                "Apache-2.0 AND ISC",
            ),
            credit("Serde", "Reading and writing data files.", "https://serde.rs/", "MIT OR Apache-2.0"),
            credit("chrono", "Dates and times.", "https://github.com/chronotope/chrono", "MIT OR Apache-2.0"),
            credit("regex", "Text matching.", "https://github.com/rust-lang/regex", "MIT OR Apache-2.0"),
            credit("url", "Reading history links.", "https://github.com/servo/rust-url", "MIT OR Apache-2.0"),
            credit("thiserror", "Error types.", "https://github.com/dtolnay/thiserror", "MIT OR Apache-2.0"),
            credit("base64", "Catalog signature encoding.", "https://github.com/marshallpierce/rust-base64", "MIT OR Apache-2.0"),
            credit("windows-sys", "Windows file access.", "https://github.com/microsoft/windows-rs", "MIT OR Apache-2.0"),
            credit("Google Sans (Fontsource)", "Interface font.", "https://fontsource.org/fonts/google-sans", "OFL-1.1"),
            credit("JetBrains Mono (Fontsource)", "Monospace font.", "https://www.jetbrains.com/lp/mono/", "OFL-1.1"),
        ],
    },
];

#[tauri::command]
pub fn get_about() -> About {
    About {
        name: "Wishtory",
        version: env!("CARGO_PKG_VERSION"),
        description: env!("CARGO_PKG_DESCRIPTION"),
        copyright: "© 2026 Wishtory contributors",
        license: "MIT License",
        disclaimer: "Wishtory is an unofficial fan project and is not affiliated with or endorsed by HoYoverse / COGNOSPHERE, or by the projects credited here. Genshin Impact, Honkai: Star Rail and Zenless Zone Zero, and their names, characters and images, belong to their owners.",
        repository: REPOSITORY,
        releases: RELEASES,
        issues: ISSUES,
        groups: GROUPS,
    }
}

/// Only links the app ships, and release pages of this project, may be opened.
fn is_allowed(url: &str) -> bool {
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    [REPOSITORY, RELEASES, ISSUES].contains(&url)
        || url
            .strip_prefix(RELEASES)
            .is_some_and(|rest| rest.starts_with('/'))
        || GROUPS
            .iter()
            .flat_map(|group| group.credits)
            .any(|credit| credit.url == url)
}

#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    if !is_allowed(&url) {
        return Err("Wishtory only opens the links it shows.".into());
    }
    open_in_browser(&url)
}

#[cfg(windows)]
fn open_in_browser(url: &str) -> Result<(), String> {
    // The URL is one argument, never parsed by a shell.
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open the browser: {e}"))
}

#[cfg(not(windows))]
fn open_in_browser(_url: &str) -> Result<(), String> {
    Err("Opening links is only supported on Windows.".into())
}
