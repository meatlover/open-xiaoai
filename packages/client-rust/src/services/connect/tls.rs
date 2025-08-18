use rustls::pki_types::PrivateKeyDer;
use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TlsClientConfig {
    pub enable_tls: bool,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>, 
    pub ca_cert_path: Option<String>,
    pub server_name: Option<String>,
}

impl Default for TlsClientConfig {
    fn default() -> Self {
        Self {
            enable_tls: false,
            client_cert_path: None,
            client_key_path: None,
            ca_cert_path: None,
            server_name: None,
        }
    }
}

impl TlsClientConfig {
    pub fn from_env() -> Self {
        let enable_tls = std::env::var("CLIENT_TLS_ENABLED")
            .ok()
            .map(|v| v == "true")
            .unwrap_or(false);
            
        let client_cert_path = std::env::var("CLIENT_CERT_PATH").ok();
        let client_key_path = std::env::var("CLIENT_KEY_PATH").ok();
        let ca_cert_path = std::env::var("CLIENT_CA_PATH").ok();
        let server_name = std::env::var("CLIENT_TLS_SERVER_NAME").ok();

        Self {
            enable_tls,
            client_cert_path,
            client_key_path,
            ca_cert_path,
            server_name,
        }
    }
}

pub fn build_tls_connector(config: &TlsClientConfig) -> anyhow::Result<Option<Arc<ClientConfig>>> {
    if !config.enable_tls {
        return Ok(None);
    }

    // Initialize crypto provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install rustls crypto provider"))?;

    let mut root_store = RootCertStore::empty();

    // Add custom CA if provided
    if let Some(ca_path) = &config.ca_cert_path {
        let ca_pem = std::fs::read(ca_path)
            .map_err(|e| anyhow::anyhow!("Failed to read CA cert from {}: {}", ca_path, e))?;
        
        let mut reader = std::io::BufReader::new(&ca_pem[..]);
        for cert in rustls_pemfile::certs(&mut reader) {
            let cert = cert?;
            root_store.add(cert)?;
        }
        println!("✅ Loaded CA certificate from {}", ca_path);
    } else {
        // Add system root certificates - use empty store for cross-compilation compatibility
        // In production, you should provide a CA certificate path
        println!("⚠️  Using empty root store - provide CLIENT_CA_PATH for server verification");
    }

    // Add client certificate if both cert and key are provided
    if let (Some(cert_path), Some(key_path)) = (&config.client_cert_path, &config.client_key_path) {
        let cert_pem = std::fs::read(cert_path)
            .map_err(|e| anyhow::anyhow!("Failed to read client cert from {}: {}", cert_path, e))?;
        
        let key_pem = std::fs::read(key_path)
            .map_err(|e| anyhow::anyhow!("Failed to read client key from {}: {}", key_path, e))?;

        // Parse certificates
        let mut cert_reader = std::io::BufReader::new(&cert_pem[..]);
        let certs = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()?;

        // Parse private key
        let mut key_reader = std::io::BufReader::new(&key_pem[..]);
        let mut pkcs8_keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
            .collect::<Result<Vec<_>, _>>()?;
        
        let key = if let Some(key) = pkcs8_keys.pop() {
            PrivateKeyDer::Pkcs8(key)
        } else {
            // Try RSA format
            let mut key_reader = std::io::BufReader::new(&key_pem[..]);
            let mut rsa_keys = rustls_pemfile::rsa_private_keys(&mut key_reader)
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(key) = rsa_keys.pop() {
                PrivateKeyDer::Pkcs1(key)
            } else {
                return Err(anyhow::anyhow!("No private key found"));
            }
        };

        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(certs, key)?;

        println!("✅ Loaded client certificate from {} and key from {}", cert_path, key_path);
        
        Ok(Some(Arc::new(client_config)))
    } else {
        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Some(Arc::new(client_config)))
    }
}
