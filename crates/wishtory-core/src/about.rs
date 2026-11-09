//! About screen data and the one place that may open a link in the browser.
//!
//! The credits live here so the About page, the list of links the app may open and the README
//! stay in step: only URLs listed below, and release pages of this project, can be opened.

use crate::error::AppError;
use serde::Serialize;

pub const REPOSITORY: &str = "https://github.com/Pylxyr/Wishtory";
pub const RELEASES: &str = "https://github.com/Pylxyr/Wishtory/releases";
const ISSUES: &str = "https://github.com/Pylxyr/Wishtory/issues";

#[derive(Serialize)]
pub struct Credit {
    pub name: &'static str,
    pub role: &'static str,
    pub url: &'static str,
    /// Licence of software and fonts; empty for data sources and references.
    pub license: &'static str,
}

#[derive(Serialize)]
pub struct CreditGroup {
    pub title: &'static str,
    pub note: &'static str,
    pub credits: &'static [Credit],
}

#[derive(Serialize)]
pub struct About {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub copyright: &'static str,
    pub license: &'static str,
    pub disclaimers: &'static [&'static str],
    pub repository: &'static str,
    pub releases: &'static str,
    pub issues: &'static str,
    pub groups: &'static [CreditGroup],
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
        ],
    },
    CreditGroup {
        title: "Banner schedule cross-checks",
        note: "Banner windows, titles and featured items were checked against these community trackers and the game wikis below. They are references; Wishtory does not use them at run time.",
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
        ],
    },
    CreditGroup {
        title: "Game wikis",
        note: "The community wikis for each game, used to check banner dates, featured items and game terms. They are references; Wishtory does not use them at run time.",
        credits: &[
            credit(
                "Genshin Impact Wiki (Fandom)",
                "Event wish durations and server times.",
                "https://genshin-impact.fandom.com/",
                "",
            ),
            credit(
                "Honkai: Star Rail Wiki (Fandom)",
                "Warp banner windows, featured characters and Light Cones, and game terms.",
                "https://honkai-star-rail.fandom.com/",
                "",
            ),
            credit(
                "Zenless Zone Zero Wiki (Fandom)",
                "Channel history, Exclusive Rescreening and Custom Search details.",
                "https://zenless-zone-zero.fandom.com/",
                "",
            ),
        ],
    },
    CreditGroup {
        title: "Built with",
        note: "Open-source software and fonts shipped inside Wishtory. The full list of libraries and their licences is in the source repository.",
        credits: &[
            credit("Flutter", "Interface toolkit.", "https://flutter.dev/", "BSD-3-Clause"),
            credit("Rust", "Core: game-file discovery, history download, pity maths and storage.", "https://www.rust-lang.org/", "Apache-2.0 OR MIT"),
            credit("flutter_rust_bridge", "Connects the Flutter interface to the Rust core.", "https://github.com/fzyzcjy/flutter_rust_bridge", "MIT"),
            credit("Riverpod", "State management.", "https://riverpod.dev/", "MIT"),
            credit("SQLite", "Local wish database.", "https://sqlite.org/", "Public domain"),
            credit("Google Sans (Fontsource)", "Interface font.", "https://fontsource.org/fonts/google-sans", "OFL-1.1"),
            credit("JetBrains Mono (Fontsource)", "Monospace font.", "https://www.jetbrains.com/lp/mono/", "OFL-1.1"),
        ],
    },
];

pub fn get_about() -> About {
    About {
        name: "Wishtory",
        version: env!("CARGO_PKG_VERSION"),
        description: env!("CARGO_PKG_DESCRIPTION"),
        copyright: "© 2026 Wishtory contributors",
        license: "MIT License",
        disclaimers: &[
            "Wishtory is an unofficial fan project. It is not affiliated with or endorsed by HoYoverse / COGNOSPHERE, or by any of the projects credited here.",
            "No tool can offer a 100% guarantee. HoYoverse can change its terms of service or its detection at any time, and nothing here can promise how the publisher will treat any third-party tool. Use Wishtory at your own risk.",
            "Genshin Impact, Honkai: Star Rail and Zenless Zone Zero, and their names, characters and images, belong to their owners.",
            "Soft-pity ranges are community estimates, because the games do not publish them.",
        ],
        repository: REPOSITORY,
        releases: RELEASES,
        issues: ISSUES,
        groups: GROUPS,
    }
}

/// The part of a release-page link after `releases/`: plain path segments only. No `..` / `.`,
/// `\`, `%`, `?` or `#`, so a browser cannot normalise the link onto another repository.
fn is_release_subpath(rest: &str) -> bool {
    rest.split('/').all(|segment| {
        segment != "."
            && segment != ".."
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+' | b'~'))
    })
}

/// Only links the app ships, and release pages of this project, may be opened.
fn is_allowed(url: &str) -> bool {
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    [REPOSITORY, RELEASES, ISSUES].contains(&url)
        || url
            .strip_prefix(RELEASES)
            .and_then(|rest| rest.strip_prefix('/'))
            .is_some_and(is_release_subpath)
        || GROUPS
            .iter()
            .flat_map(|group| group.credits)
            .any(|credit| credit.url == url)
}

pub fn open_external(url: String) -> Result<(), AppError> {
    if !is_allowed(&url) {
        return Err(AppError::Message(
            "Wishtory only opens the links it shows.".into(),
        ));
    }
    open_in_browser(&url).map_err(AppError::Message)
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

#[cfg(unix)]
fn open_in_browser(url: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    // xdg-open opens the default browser on every Linux desktop (KDE, GNOME, ...); gio is the
    // fallback. The URL is one argument, never parsed by a shell, and it was checked against the
    // fixed list above.
    for (program, prefix) in [("xdg-open", None), ("gio", Some("open"))] {
        let mut command = Command::new(program);
        command
            .args(prefix)
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match command.spawn() {
            Ok(mut child) => {
                // Reap the helper when it exits, so it does not linger as a zombie process.
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("Could not open the browser: {e}")),
        }
    }
    Err(
        "Could not open the browser. Install xdg-utils (xdg-open) or open the link yourself."
            .into(),
    )
}

#[cfg(not(any(windows, unix)))]
fn open_in_browser(_url: &str) -> Result<(), String> {
    Err("Opening links is not supported on this system.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_only_listed_links_and_release_pages_of_this_project() {
        for link in [
            REPOSITORY,
            RELEASES,
            ISSUES,
            "https://github.com/Pylxyr/Wishtory/releases/",
            "https://github.com/Pylxyr/Wishtory/releases/tag/v0.1.0",
            "https://github.com/Pylxyr/Wishtory/releases/tag/v1.2.3-rc.1+b5",
            "https://uigf.org/",
            "https://paimon.moe/",
            "https://genshin-impact.fandom.com/",
            "https://honkai-star-rail.fandom.com/",
            "https://zenless-zone-zero.fandom.com/",
        ] {
            assert!(is_allowed(link), "{link}");
        }
        for link in [
            "",
            "https://uigf.org",
            "http://github.com/Pylxyr/Wishtory",
            "https://github.com/Pylxyr/Wishtory-evil",
            "https://github.com/Pylxyr/Wishtory/releasesx",
            "https://github.com/Pylxyr/Wishtory/releases/../../../evil/repo",
            "https://github.com/Pylxyr/Wishtory/releases/tag/%2e%2e/x",
            "https://github.com/Pylxyr/Wishtory/releases/\\..\\..\\evil",
            "https://github.com/Pylxyr/Wishtory/releases/tag/v1?next=https://evil.example",
            "https://github.com/Pylxyr/Wishtory/releases/tag/v1#frag",
            "https://github.com/Pylxyr/Wishtory/releases/tag/v1\n",
            "file:///C:/Windows/System32/calc.exe",
        ] {
            assert!(!is_allowed(link), "{link:?}");
        }
    }
}
