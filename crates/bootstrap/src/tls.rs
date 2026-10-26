use std::sync::Once;

pub fn install_crypto_provider() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_client_builds_after_install() {
        install_crypto_provider();
        install_crypto_provider();
        reqwest::Client::builder()
            .build()
            .expect("reqwest client builds");
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }
}
