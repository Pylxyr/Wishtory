use serde::{Serialize, Serializer};

/// Every error is shown to the user as-is, so messages are written to be friendly.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0} wasn't found. Launch the game at least once so it creates its log file, then try again.")]
    GameNotFound(&'static str),
    #[error("No history page found in the game's cache yet. In the game, open the Wish / Warp / Signal Search history page, wait until it loads, then come back and click Refresh.")]
    HistoryPageNotOpened,
    #[error("This history link has expired. In the game, open the Wish / Warp / Signal Search history page again, wait until the list loads, then click Refresh.")]
    AuthExpired,
    #[error("That doesn't look like a valid HoYoverse history link for this game. Use Refresh instead, or paste the full link: it starts with https:// and contains authkey=.")]
    InvalidUrl,
    #[error("The server asked us to slow down. Please wait a minute and try again.")]
    RateLimited,
    #[error("The server returned an error: {0}")]
    Api(String),
    #[error("Network problem: {0}. Check your internet connection (and turn off any VPN or proxy that blocks HoYoverse), then click Refresh again.")]
    Network(String),
    #[error("Database error: {0}")]
    Db(String),
    #[error("File error: {0}")]
    Io(String),
    #[error("Unsupported or invalid file: {0}")]
    Format(String),
    /// A failure whose message is already written for the user (update checks, the signed
    /// catalog / banner / rule feeds, links that may not be opened).
    #[error("{0}")]
    Message(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e.to_string())
    }
}
impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        // reqwest includes the request URL in its display text; for history requests
        // that URL contains the account's authkey, so never forward it to the UI.
        Self::Network(e.without_url().to_string())
    }
}
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Format(e.to_string())
    }
}
