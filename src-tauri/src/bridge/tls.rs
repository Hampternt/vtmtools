// Self-signed TLS for the wss:// bridge listener. Generated once on first
// launch, persisted under the Tauri app data dir, loaded on subsequent
// launches. SAN is `localhost` only — the GM accepts the cert warning
// once in their Foundry browser; the trust persists for the session.
//
// Foundry hosted over HTTPS (Forge, Molten, any TLS-proxied self-host)
// can't dial ws:// from the page due to mixed-content rules, so the wss
// listener is the only path that works for hosted Foundry.

use std::path::Path;
use std::sync::Arc;

use tokio::fs;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

/// Returns a TlsAcceptor backed by a self-signed cert in the app data dir.
/// Generates and writes the cert on first call; loads it on subsequent calls.
pub async fn ensure_cert(app_data_dir: &Path) -> Result<TlsAcceptor, String> {
    install_crypto_provider();

    let cert_path = app_data_dir.join("bridge-cert.pem");
    let key_path = app_data_dir.join("bridge-key.pem");

    if !cert_path.exists() || !key_path.exists() {
        let key = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .map_err(|e| format!("rcgen generate: {e}"))?;
        fs::write(&cert_path, key.cert.pem())
            .await
            .map_err(|e| format!("write {}: {e}", cert_path.display()))?;
        fs::write(&key_path, key.key_pair.serialize_pem())
            .await
            .map_err(|e| format!("write {}: {e}", key_path.display()))?;
    }

    let cert_pem = fs::read(&cert_path)
        .await
        .map_err(|e| format!("read {}: {e}", cert_path.display()))?;
    let key_pem = fs::read(&key_path)
        .await
        .map_err(|e| format!("read {}: {e}", key_path.display()))?;

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_pem.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("parse certs: {e}"))?;
    if certs.is_empty() {
        return Err("no certs found in PEM".into());
    }

    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_pem.as_slice())
        .map_err(|e| format!("parse key: {e}"))?
        .ok_or_else(|| "no private key found in PEM".to_string())?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("server config: {e}"))?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// rustls 0.23 requires a CryptoProvider before any TLS use. Installing
/// twice is a no-op (returns Err) — sqlx may have already done it.
fn install_crypto_provider() {
    let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();
}
