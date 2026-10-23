//! Read-only file helpers shared by discover.rs and authkey.rs.
//! SAFETY: every file is opened for reading only. If a file cannot be opened directly
//! (for example the game holds a lock), it is copied to OUR temp folder and the copy is read.
//! Nothing is ever written next to, or into, a game folder.

use std::{
    fs,
    io::Read,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static COPY_SEQ: AtomicU64 = AtomicU64::new(0);

/// Reads a whole file with maximum sharing flags (so a running game is never blocked),
/// falling back to a temp-folder copy. Returns the ORIGINAL error if both attempts fail.
pub fn read_shared(path: &Path) -> std::io::Result<Vec<u8>> {
    let direct = (|| -> std::io::Result<Vec<u8>> {
        let mut opts = fs::OpenOptions::new();
        opts.read(true); // never write
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.share_mode(0x1 | 0x2 | 0x4); // FILE_SHARE_READ | WRITE | DELETE
        }
        let mut buf = Vec::new();
        opts.open(path)?.read_to_end(&mut buf)?;
        Ok(buf)
    })();
    match direct {
        Ok(b) => Ok(b),
        Err(first) => {
            let n = COPY_SEQ.fetch_add(1, Ordering::Relaxed);
            let tmp = std::env::temp_dir().join(format!("wishtory-copy-{}-{n}.tmp", std::process::id()));
            let copied = fs::copy(path, &tmp).and_then(|_| fs::read(&tmp));
            let _ = fs::remove_file(&tmp);
            copied.map_err(|_| first)
        }
    }
}
