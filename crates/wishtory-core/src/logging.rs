//! Opt-in, in-memory diagnostics log.
//!
//! * Off by default. While off, `wlog!` is a single relaxed atomic load: its format arguments are
//!   never evaluated and nothing is stored.
//! * Entries live only in a bounded ring buffer (nothing is written to disk).
//! * Never log history links or authkeys. Messages are also scrubbed defensively in `push`.

use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        Mutex, OnceLock,
    },
};

const CAPACITY: usize = 1000;
const MAX_MESSAGE_CHARS: usize = 600;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub seq: u64,
    pub time: String,
    pub level: &'static str,
    pub source: String,
    pub message: String,
}

static ENABLED: AtomicBool = AtomicBool::new(false);
/// Minimum level that is recorded. DEBUG by default, so everything is kept once enabled.
static MIN_LEVEL: AtomicU8 = AtomicU8::new(Level::Debug as u8);
static SEQ: AtomicU64 = AtomicU64::new(1);
static BUFFER: OnceLock<Mutex<VecDeque<LogEntry>>> = OnceLock::new();

fn buffer() -> &'static Mutex<VecDeque<LogEntry>> {
    BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

#[inline]
pub fn enabled(level: Level) -> bool {
    ENABLED.load(Ordering::Relaxed) && level as u8 >= MIN_LEVEL.load(Ordering::Relaxed)
}

/// Parameters whose values are never kept: the game's authkey, and the OAuth values of cloud backup
/// (`token=` also covers `access_token=` and `refresh_token=`).
const SECRET_PARAMS: [&str; 6] = [
    "authkey=",
    "token=",
    "code=",
    "code_verifier=",
    "client_secret=",
    "passphrase=",
];

/// Replaces the value of any secret parameter, in case a message ever carries a link or a form body.
fn scrub(message: &str) -> String {
    SECRET_PARAMS
        .iter()
        .fold(message.to_string(), |text, key| scrub_param(&text, key))
}

fn scrub_param(message: &str, key: &str) -> String {
    let lower = message.to_ascii_lowercase(); // same byte offsets as `message`
    let mut out = String::with_capacity(message.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find(key) {
        let value_start = at + found + key.len();
        out.push_str(&message[at..value_start]);
        out.push_str("<redacted>");
        let rest = &message[value_start..];
        at = value_start
            + rest
                .find(|c: char| c == '&' || c.is_whitespace() || c == '"' || c == '\'')
                .unwrap_or(rest.len());
    }
    out.push_str(&message[at..]);
    out
}

pub fn push(level: Level, source: &str, message: String) {
    let mut message = scrub(&message);
    if message.chars().count() > MAX_MESSAGE_CHARS {
        message = message.chars().take(MAX_MESSAGE_CHARS).collect::<String>() + "…";
    }
    let entry = LogEntry {
        seq: SEQ.fetch_add(1, Ordering::Relaxed),
        time: chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S%.3f")
            .to_string(),
        level: level.as_str(),
        source: source.trim_start_matches("wishtory::").to_string(),
        message,
    };
    if let Ok(mut buf) = buffer().lock() {
        if buf.len() >= CAPACITY {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}

/// `wlog!(Level::Debug, "fetched {n} pulls")`. Arguments are only evaluated when logging is on.
macro_rules! wlog {
    ($level:expr, $($arg:tt)*) => {
        if $crate::logging::enabled($level) {
            $crate::logging::push($level, module_path!(), format!($($arg)*));
        }
    };
}

pub fn set_logging_enabled(enabled: bool) {
    if enabled {
        ENABLED.store(true, Ordering::Relaxed);
        wlog!(
            Level::Info,
            "Logging enabled (level DEBUG, in-memory, last {CAPACITY} lines)"
        );
    } else {
        ENABLED.store(false, Ordering::Relaxed);
        if let Ok(mut buf) = buffer().lock() {
            buf.clear();
        } // nothing lingers once switched off
    }
}

pub fn get_logs() -> Vec<LogEntry> {
    if !ENABLED.load(Ordering::Relaxed) {
        return Vec::new();
    }
    buffer()
        .lock()
        .map(|b| b.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn clear_logs() {
    if let Ok(mut buf) = buffer().lock() {
        buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::scrub;

    #[test]
    fn game_and_cloud_secrets_are_redacted() {
        let line = "GET https://x.example/h?authkey=AAA%2B&game_biz=hk4e&code=4/0Abc&state=S \
                    refresh_token=1//RT access_token=ya29.AT client_secret=GOCSPX-x passphrase=hunter2 done";
        let out = scrub(line);
        for secret in ["AAA", "4/0Abc", "1//RT", "ya29.AT", "GOCSPX-x", "hunter2"] {
            assert!(!out.contains(secret), "{secret} leaked: {out}");
        }
        // Everything around the secrets survives.
        assert!(out.contains("game_biz=hk4e") && out.contains("state=S") && out.ends_with("done"));
    }

    #[test]
    fn text_without_secrets_is_unchanged() {
        let line = "cloud: backup uploaded (51234 bytes), status 200 OK";
        assert_eq!(scrub(line), line);
    }
}
