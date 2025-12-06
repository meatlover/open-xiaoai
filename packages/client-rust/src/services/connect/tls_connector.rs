use native_tls::{Identity, TlsConnector};
use std::fs::File;
use std::io::Read;

pub struct TlsConfig {
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ca_path: Option<String>,
}

impl TlsConfig {
    pub fn from_env() -> Self {
        Self {
            cert_path: std::env::var("CLIENT_CERT_PATH").ok(),
            key_path: std::env::var("CLIENT_KEY_PATH").ok(),
            ca_path: std::env::var("CA_CERT_PATH").ok(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.cert_path.is_some() && self.key_path.is_some()
    }
}

pub fn build_tls_connector(config: &TlsConfig) -> Result<TlsConnector, Box<dyn std::error::Error>> {
    let mut builder = TlsConnector::builder();

    // Load client certificate and private key if provided
    if let (Some(cert_path), Some(key_path)) = (&config.cert_path, &config.key_path) {
        let identity = load_identity(cert_path, key_path)?;
        builder.identity(identity);
    }

    // Load custom CA certificate if provided
    if let Some(ca_path) = &config.ca_path {
        let ca_cert = load_certificate(ca_path)?;
        builder.add_root_certificate(ca_cert);
    }

    // Accept invalid certificates if specified (for testing)
    if std::env::var("ACCEPT_INVALID_CERTS").is_ok() {
        builder.danger_accept_invalid_certs(true);
    }

    Ok(builder.build()?)
}

fn load_identity(cert_path: &str, key_path: &str) -> Result<Identity, Box<dyn std::error::Error>> {
    // Check if it's a PKCS#12 file (single file contains both cert and key)
    if cert_path.ends_with(".p12") || cert_path.ends_with(".pfx") {
        let mut file = File::open(cert_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let password = std::env::var("CLIENT_CERT_PASSWORD").unwrap_or_default();
        return Ok(Identity::from_pkcs12(&data, &password)?);
    }

    // For PEM format, we need to convert to PKCS#12
    // Read certificate
    let mut cert_file = File::open(cert_path)?;
    let mut cert_data = Vec::new();
    cert_file.read_to_end(&mut cert_data)?;

    // Read private key  
    let mut key_file = File::open(key_path)?;
    let mut key_data = Vec::new();
    key_file.read_to_end(&mut key_data)?;

    // Attempt to create identity from PEM
    // Note: native-tls requires PKCS#12 or PKCS#8 format
    // For PEM files, we try PKCS#8 DER encoding
    let identity = Identity::from_pkcs8(&cert_data, &key_data)?;
    
    Ok(identity)
}

fn load_certificate(ca_path: &str) -> Result<native_tls::Certificate, Box<dyn std::error::Error>> {
    let mut file = File::open(ca_path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    if ca_path.ends_with(".der") {
        Ok(native_tls::Certificate::from_der(&data)?)
    } else {
        Ok(native_tls::Certificate::from_pem(&data)?)
    }
}
