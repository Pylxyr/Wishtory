//! The encrypted container a backup is stored in. Everything is encrypted on this PC before it is
//! uploaded, so Google only ever holds an opaque file.
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! "WSHB" | version u8 | kdf u8 | m_cost_kib u32 | t_cost u32 | p_cost u32 | salt 16 | nonce 12 | ciphertext + tag
//! ```
//!
//! * The key is Argon2id(passphrase, salt), 256 bits, with a fresh random salt for every backup, so a
//!   key (and therefore a nonce) is never used twice.
//! * The cipher is ChaCha20-Poly1305. The whole header is authenticated as associated data, so
//!   changing the cost parameters, the salt or the version makes decryption fail.
//! * The cost parameters in a file are read before the passphrase is hashed, so they are capped:
//!   a tampered file cannot make this PC allocate gigabytes.

use crate::error::AppError;
use argon2::{Algorithm, Argon2, Params, Version};
use ring::{
    aead::{self, Aad, LessSafeKey, Nonce, UnboundKey},
    rand::{SecureRandom, SystemRandom},
};
use zeroize::Zeroizing;

const MAGIC: &[u8; 4] = b"WSHB";
const FORMAT_VERSION: u8 = 1;
const KDF_ARGON2ID: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const HEADER_LEN: usize = 4 + 1 + 1 + 4 + 4 + 4 + SALT_LEN + NONCE_LEN;

/// Shortest passphrase accepted. A backup file can be copied and attacked offline, so this is the
/// only thing standing between it and a guesser.
pub const MIN_PASSPHRASE_CHARS: usize = 10;
const MAX_PASSPHRASE_CHARS: usize = 256;

/// Argon2id cost. These are OWASP's "high memory" tier or better and take well under a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cost {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl Cost {
    pub const DEFAULT: Cost = Cost {
        m_kib: 64 * 1024,
        t: 3,
        p: 1,
    };
    /// Upper bounds accepted from a file.
    const MAX: Cost = Cost {
        m_kib: 256 * 1024,
        t: 10,
        p: 4,
    };
}

pub fn validate_passphrase(passphrase: &str) -> Result<(), AppError> {
    let chars = passphrase.chars().count();
    if chars < MIN_PASSPHRASE_CHARS {
        return Err(AppError::Message(format!(
            "Choose a backup passphrase of at least {MIN_PASSPHRASE_CHARS} characters."
        )));
    }
    if chars > MAX_PASSPHRASE_CHARS {
        return Err(AppError::Message(format!(
            "The backup passphrase can be at most {MAX_PASSPHRASE_CHARS} characters."
        )));
    }
    Ok(())
}

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    cost: Cost,
) -> Result<Zeroizing<[u8; KEY_LEN]>, AppError> {
    let bad = |_| AppError::Format("this backup uses settings Wishtory cannot read".into());
    let params = Params::new(cost.m_kib, cost.t, cost.p, Some(KEY_LEN)).map_err(bad)?;
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut_slice())
        .map_err(bad)?;
    Ok(key)
}

fn cipher(key: &[u8; KEY_LEN]) -> Result<LessSafeKey, AppError> {
    UnboundKey::new(&aead::CHACHA20_POLY1305, key)
        .map(LessSafeKey::new)
        .map_err(|_| AppError::Format("the encryption key was rejected".into()))
}

/// Encrypts `plaintext` under `passphrase` with the default cost.
pub fn seal(passphrase: &str, plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
    seal_with(passphrase, plaintext, Cost::DEFAULT)
}

fn seal_with(passphrase: &str, plaintext: &[u8], cost: Cost) -> Result<Vec<u8>, AppError> {
    validate_passphrase(passphrase)?;
    let rng = SystemRandom::new();
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    let random_failed = |_| AppError::Io("The system could not provide random numbers.".into());
    rng.fill(&mut salt).map_err(random_failed)?;
    rng.fill(&mut nonce).map_err(random_failed)?;

    let mut out =
        Vec::with_capacity(HEADER_LEN + plaintext.len() + aead::CHACHA20_POLY1305.tag_len());
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_VERSION);
    out.push(KDF_ARGON2ID);
    out.extend_from_slice(&cost.m_kib.to_le_bytes());
    out.extend_from_slice(&cost.t.to_le_bytes());
    out.extend_from_slice(&cost.p.to_le_bytes());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    debug_assert_eq!(out.len(), HEADER_LEN);

    let key = derive_key(passphrase, &salt, cost)?;
    let (header, _) = out.split_at(HEADER_LEN);
    let mut body = plaintext.to_vec();
    cipher(&key)?
        .seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(header),
            &mut body,
        )
        .map_err(|_| AppError::Format("the backup could not be encrypted".into()))?;
    out.extend_from_slice(&body);
    Ok(out)
}

/// Decrypts a backup. A wrong passphrase and a damaged file are deliberately not told apart.
pub fn open(passphrase: &str, blob: &[u8]) -> Result<Zeroizing<Vec<u8>>, AppError> {
    let not_a_backup = || AppError::Format("this is not a Wishtory backup".into());
    if blob.len() < HEADER_LEN + aead::CHACHA20_POLY1305.tag_len() || &blob[..4] != MAGIC {
        return Err(not_a_backup());
    }
    if blob[4] != FORMAT_VERSION || blob[5] != KDF_ARGON2ID {
        return Err(AppError::Message(
            "This backup was made by a newer Wishtory. Update Wishtory, then try again.".into(),
        ));
    }
    let word = |at: usize| u32::from_le_bytes(blob[at..at + 4].try_into().expect("4 bytes"));
    let cost = Cost {
        m_kib: word(6),
        t: word(10),
        p: word(14),
    };
    if cost.m_kib > Cost::MAX.m_kib || cost.t > Cost::MAX.t || cost.p > Cost::MAX.p {
        return Err(not_a_backup());
    }
    let salt = &blob[18..18 + SALT_LEN];
    let nonce: [u8; NONCE_LEN] = blob[18 + SALT_LEN..HEADER_LEN]
        .try_into()
        .expect("12 bytes");

    let key = derive_key(passphrase, salt, cost)?;
    let mut body = Zeroizing::new(blob[HEADER_LEN..].to_vec());
    let plain_len = cipher(&key)?
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(&blob[..HEADER_LEN]),
            body.as_mut_slice(),
        )
        .map_err(|_| {
            AppError::Message(
                "Couldn't decrypt this backup. The passphrase is wrong, or the file was changed."
                    .into(),
            )
        })?
        .len();
    body.truncate(plain_len);
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: Cost = Cost {
        m_kib: 64,
        t: 1,
        p: 1,
    };
    const PASS: &str = "correct horse battery";

    #[test]
    fn round_trips() {
        let blob = seal_with(PASS, b"{\"hk4e\":[]}", FAST).unwrap();
        assert_eq!(&blob[..4], MAGIC);
        assert_eq!(&*open(PASS, &blob).unwrap(), b"{\"hk4e\":[]}");
    }

    #[test]
    fn plaintext_never_appears_in_the_file() {
        let secret = b"UID 123456789 five-star Furina";
        let blob = seal_with(PASS, secret, FAST).unwrap();
        assert!(!blob.windows(secret.len()).any(|w| w == secret));
    }

    #[test]
    fn every_backup_uses_a_fresh_salt_and_nonce() {
        let a = seal_with(PASS, b"same", FAST).unwrap();
        let b = seal_with(PASS, b"same", FAST).unwrap();
        assert_ne!(a[18..HEADER_LEN], b[18..HEADER_LEN]);
        assert_ne!(a[HEADER_LEN..], b[HEADER_LEN..]);
    }

    #[test]
    fn wrong_passphrase_is_refused() {
        let blob = seal_with(PASS, b"data", FAST).unwrap();
        assert!(open("another passphrase", &blob).is_err());
    }

    #[test]
    fn any_changed_byte_is_refused() {
        let blob = seal_with(PASS, b"data", FAST).unwrap();
        for at in [5, 6, 20, HEADER_LEN - 1, HEADER_LEN, blob.len() - 1] {
            let mut bad = blob.clone();
            bad[at] ^= 1;
            assert!(open(PASS, &bad).is_err(), "byte {at}");
        }
        assert!(open(PASS, &blob[..blob.len() - 1]).is_err());
    }

    #[test]
    fn rejects_files_that_are_not_backups_or_ask_for_huge_memory() {
        assert!(open(PASS, b"").is_err());
        assert!(open(PASS, &[0u8; 100]).is_err());
        let mut blob = seal_with(PASS, b"data", FAST).unwrap();
        blob[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(open(PASS, &blob), Err(AppError::Format(_))));
    }

    #[test]
    fn passphrase_rules() {
        assert!(validate_passphrase("short").is_err());
        assert!(validate_passphrase(&"x".repeat(MAX_PASSPHRASE_CHARS + 1)).is_err());
        assert!(validate_passphrase("ten chars!!").is_ok());
        assert!(seal_with("short", b"x", FAST).is_err());
    }
}
