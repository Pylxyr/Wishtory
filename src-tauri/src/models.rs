use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    Hk4e,  // Genshin Impact
    Hkrpg, // Honkai: Star Rail
    Nap,   // Zenless Zone Zero
}

impl Game {
    pub const ALL: [Game; 3] = [Game::Hk4e, Game::Hkrpg, Game::Nap];

    pub fn as_str(&self) -> &'static str {
        match self {
            Game::Hk4e => "hk4e",
            Game::Hkrpg => "hkrpg",
            Game::Nap => "nap",
        }
    }
    pub fn display(&self) -> &'static str {
        match self {
            Game::Hk4e => "Genshin Impact",
            Game::Hkrpg => "Honkai: Star Rail",
            Game::Nap => "Zenless Zone Zero",
        }
    }
    /// Gacha types queried from the API. Add new types here when the game adds banners
    /// (e.g. collaboration banners in Star Rail).
    pub fn gacha_types(&self) -> &'static [&'static str] {
        match self {
            // 301 also returns 400 (second character banner) from the API.
            Game::Hk4e => &["100", "200", "301", "302", "500"],
            // UIGF v4.1 added the additional event pool types introduced with Star Rail 3.4.
            Game::Hkrpg => &["1", "2", "11", "12", "21", "22"],
            // 102 / 103 are the selectable rerun channels (Exclusive Rescreening and W-Engine
            // Reverberation, since Zenless 2.5). The UIGF standard still lists only 1, 2, 3 and 5.
            Game::Nap => &["1", "2", "3", "5", "102", "103"],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pull {
    pub id: String,
    pub game: String,
    pub uid: String,
    pub gacha_type: String,
    pub gacha_id: String,
    pub item_id: String,
    pub name: String,
    pub item_type: String,
    pub rank_type: u8,
    pub time: String,
    pub lang: String,
    pub count: u32,
}

/// Zenless' selectable rerun channels (Exclusive Rescreening 102, W-Engine Reverberation 103).
/// Their records are stored under these numbers even though the history API reports them as the
/// ordinary Exclusive / W-Engine channel (2 / 3).
pub fn is_rerun_gacha_type(game: &str, gacha_type: &str) -> bool {
    game == "nap" && matches!(gacha_type, "102" | "103")
}

/// The ordinary channel a stored Zenless `gacha_type` is reported as in the UIGF standard, which
/// lists only 1, 2, 3 and 5: Rescreening (102) is "2", Reverberation (103) is "3".
pub fn uigf_gacha_type<'a>(game: &str, gacha_type: &'a str) -> &'a str {
    match (game, gacha_type) {
        ("nap", "102") => "2",
        ("nap", "103") => "3",
        _ => gacha_type,
    }
}

impl Pull {
    /// Column list matching `from_row`, for `SELECT {COLUMNS} FROM pulls`.
    pub const COLUMNS: &'static str =
        "id, game, uid, gacha_type, gacha_id, item_id, name, item_type, rank_type, time, lang, count";

    pub fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Pull> {
        Ok(Pull {
            id: r.get(0)?,
            game: r.get(1)?,
            uid: r.get(2)?,
            gacha_type: r.get(3)?,
            gacha_id: r.get(4)?,
            item_id: r.get(5)?,
            name: r.get(6)?,
            item_type: r.get(7)?,
            rank_type: r.get(8)?,
            time: r.get(9)?,
            lang: r.get(10)?,
            count: r.get(11)?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Account {
    pub game: String,
    pub uid: String,
    pub nickname: String,
    pub timezone: i32,
    pub pull_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameInstall {
    pub game: Game,
    pub found: bool,
    pub log_path: Option<String>,
    pub data_dir: Option<String>,
}
