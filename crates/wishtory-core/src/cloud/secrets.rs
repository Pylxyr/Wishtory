//! Where the two secrets live: the Google refresh token and the backup passphrase.
//!
//! They go in the operating system's credential store: Windows Credential Manager, or the Secret
//! Service on Linux (KWallet, GNOME Keyring and others provide it). Wishtory never falls back to a
//! plain file: if the system has no credential store, cloud backup says so and stays off.

use crate::error::AppError;
use zeroize::Zeroizing;

pub const SERVICE: &str = "app.wishtory.desktop";
pub const REFRESH_TOKEN: &str = "google-refresh-token";
pub const PASSPHRASE: &str = "backup-passphrase";

pub trait SecretStore: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<String>>, AppError>;
    fn set(&self, name: &str, value: &str) -> Result<(), AppError>;
    /// Deleting something that is not there is fine.
    fn delete(&self, name: &str) -> Result<(), AppError>;
}

/// The real thing.
pub struct OsKeyring;

fn unavailable(e: keyring::Error) -> AppError {
    AppError::Message(format!(
        "Wishtory keeps your Google sign-in in your system's password storage (Windows Credential \
         Manager, or a Secret Service keyring such as KWallet or GNOME Keyring), and it is not \
         available: {e}"
    ))
}

fn entry(name: &str) -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(SERVICE, name).map_err(unavailable)
}

impl SecretStore for OsKeyring {
    fn get(&self, name: &str) -> Result<Option<Zeroizing<String>>, AppError> {
        match entry(name)?.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(unavailable(e)),
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<(), AppError> {
        entry(name)?.set_password(value).map_err(unavailable)
    }

    fn delete(&self, name: &str) -> Result<(), AppError> {
        match entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(unavailable(e)),
        }
    }
}

#[cfg(test)]
pub use memory::MemoryStore;

#[cfg(test)]
mod memory {
    use super::*;
    use std::{collections::HashMap, sync::Mutex};

    /// An in-memory stand-in for tests.
    #[derive(Default)]
    pub struct MemoryStore(Mutex<HashMap<String, String>>);

    impl SecretStore for MemoryStore {
        fn get(&self, name: &str) -> Result<Option<Zeroizing<String>>, AppError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .map(Zeroizing::new))
        }
        fn set(&self, name: &str, value: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().insert(name.into(), value.into());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().remove(name);
            Ok(())
        }
    }

    impl MemoryStore {
        pub fn is_empty(&self) -> bool {
            self.0.lock().unwrap().is_empty()
        }
    }
}
