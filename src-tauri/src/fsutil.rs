//! Read-only file helpers shared by discover.rs and authkey.rs.
//! SAFETY: every file is opened for reading only. If a file cannot be opened directly
//! (for example the game holds a lock), it is copied to OUR temp folder and the copy is read.
//! Nothing is ever written next to, or into, a game folder.

use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static COPY_SEQ: AtomicU64 = AtomicU64::new(0);

/// Reads a whole file with maximum sharing flags (so a running game is never blocked),
/// falling back to a temp-folder copy. Returns the ORIGINAL error if both attempts fail.
pub fn read_shared(path: &Path) -> std::io::Result<Vec<u8>> {
    read_shared_head(path, u64::MAX)
}

/// Like `read_shared`, but reads at most the first `max_bytes` (a huge log whose interesting
/// lines are all near the start does not have to be loaded whole).
pub fn read_shared_head(path: &Path, max_bytes: u64) -> std::io::Result<Vec<u8>> {
    let direct = (|| -> std::io::Result<Vec<u8>> {
        let mut opts = fs::OpenOptions::new();
        opts.read(true); // never write
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.share_mode(0x1 | 0x2 | 0x4); // FILE_SHARE_READ | WRITE | DELETE
        }
        let mut buf = Vec::new();
        opts.open(path)?.take(max_bytes).read_to_end(&mut buf)?;
        Ok(buf)
    })();
    match direct {
        Ok(b) => Ok(b),
        Err(first) => read_from_private_copy(path)
            .map(|mut b| {
                b.truncate(usize::try_from(max_bytes).unwrap_or(usize::MAX));
                b
            })
            .map_err(|_| first),
    }
}

fn read_from_private_copy(path: &Path) -> std::io::Result<Vec<u8>> {
    // `create_new` refuses pre-existing files and symlinks. `fs::copy` would follow and
    // truncate a predictable temp-path symlink, which could overwrite an arbitrary file.
    for _ in 0..16 {
        let n = COPY_SEQ.fetch_add(1, Ordering::Relaxed);
        let tmp =
            std::env::temp_dir().join(format!("wishtory-copy-{}-{n}.tmp", std::process::id()));
        let mut output = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let copied = (|| {
            let mut input = fs::File::open(path)?;
            std::io::copy(&mut input, &mut output)?;
            output.flush()?;
            drop(output);
            fs::read(&tmp)
        })();
        let _ = fs::remove_file(&tmp);
        return copied;
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a private temporary file",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_copy_reads_data_and_leaves_no_temp_file() {
        let source = std::env::temp_dir().join(format!(
            "wishtory-source-{}-{}.tmp",
            std::process::id(),
            COPY_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&source, b"read-only fallback").unwrap();
        let contents = read_from_private_copy(&source).unwrap();
        assert_eq!(contents, b"read-only fallback");
        fs::remove_file(source).unwrap();
    }

    #[test]
    fn head_reads_stop_at_the_limit() {
        let path = std::env::temp_dir().join(format!("wishtory-head-{}.bin", std::process::id()));
        std::fs::write(&path, b"0123456789").unwrap();
        assert_eq!(read_shared_head(&path, 4).unwrap(), b"0123");
        assert_eq!(read_shared_head(&path, 100).unwrap(), b"0123456789");
        assert_eq!(read_shared(&path).unwrap(), b"0123456789");
        std::fs::remove_file(&path).unwrap();
    }
}
