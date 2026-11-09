//! The one place HTTP clients are configured. Wishtory uses rustls with the `ring` crypto provider
//! (the same library the signatures and the backup encryption use), which has to be installed once
//! per process before the first client is built.

use std::{sync::Once, time::Duration};

static TLS: Once = Once::new();

/// Installs the `ring` provider for rustls. Safe to call from anywhere, any number of times.
pub fn install_tls() {
    TLS.call_once(|| {
        // An error only means another provider was installed first, which is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// A client builder with Wishtory's user agent, HTTPS only. Callers add their own timeouts and
/// redirect policy.
pub fn client_builder() -> reqwest::ClientBuilder {
    install_tls();
    reqwest::Client::builder()
        .user_agent(concat!("Wishtory/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .connect_timeout(Duration::from_secs(10))
}
