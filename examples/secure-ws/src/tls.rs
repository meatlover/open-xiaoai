use once_cell::sync::Lazy;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientCertVerifier, RootCertStore, ServerConfig};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TlsConfig {
    pub enable_tls: bool,
    pub require_client_auth: bool,
    pub cert_chain_pem: Option<Vec<u8>>, // PEM
    pub priv_key_pem: Option<Vec<u8>>,   // PEM
    pub client_ca_pem: Option<Vec<u8>>,  // PEM (for mTLS)
}

pub static TLS_CONFIG: Lazy<parking_lot::RwLock<TlsConfig>> = Lazy::new(|| {
    parking_lot::RwLock::new(TlsConfig {
        enable_tls: false,
        require_client_auth: false,
        cert_chain_pem: None,
        priv_key_pem: None,
        client_ca_pem: None,
    })
});

pub fn get_tls() -> TlsConfig { TLS_CONFIG.read().clone() }

pub fn set_tls(cfg: TlsConfig) { *TLS_CONFIG.write() = cfg; }

pub fn build_rustls(cfg: &TlsConfig) -> anyhow::Result<Option<Arc<ServerConfig>>> {
    if !cfg.enable_tls { return Ok(None); }

    let certs = if let Some(pem) = &cfg.cert_chain_pem {
        let mut rd = std::io::BufReader::new(&pem[..]);
        rustls_pemfile::certs(&mut rd)
            .into_iter()
            .map(|c| CertificateDer::from(c))
            .collect::<Vec<_>>()
    } else { vec![] };

    let key = if let Some(pem) = &cfg.priv_key_pem {
        // try pkcs8 then rsa
        let mut rd = std::io::BufReader::new(&pem[..]);
        if let Some(k) = rustls_pemfile::pkcs8_private_keys(&mut rd).pop() {
            Some(PrivateKeyDer::Pkcs8(k.into()))
        } else {
            let mut rd = std::io::BufReader::new(&pem[..]);
            rustls_pemfile::rsa_private_keys(&mut rd)
                .pop()
                .map(|k| PrivateKeyDer::Pkcs1(k.into()))
        }
    } else { None };

    let (Some(key), true) = (key, !certs.is_empty()) else {
        anyhow::bail!("missing cert/key for TLS");
    };

    let server = if cfg.require_client_auth {
        // Build client roots
        let mut roots = RootCertStore::empty();
        if let Some(pem) = &cfg.client_ca_pem {
            let mut rd = std::io::BufReader::new(&pem[..]);
            for der in rustls_pemfile::certs(&mut rd) {
                let _ = roots.add(CertificateDer::from(der));
            }
        }
        let verifier = rustls::server::WebPkiClientVerifier::builder(roots).build()?;
        rustls::ServerConfig::builder()
            .with_client_cert_verifier(Arc::new(verifier))
            .with_single_cert(certs, key)?
    } else {
        rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key)?
    };

    Ok(Some(Arc::new(server)))
}
