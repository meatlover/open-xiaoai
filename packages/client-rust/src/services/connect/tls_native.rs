use native_tls::{TlsConnector, Identity};

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
            // Default to enabling TLS connector with relaxed validation to improve
            // compatibility on embedded targets that may lack full CA bundles.
            enable_tls: true,
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
            .unwrap_or(true);
            
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

// For native-tls, we'll use a simpler approach
// Client certificate support in native-tls is more limited
pub fn build_tls_connector(_config: &TlsClientConfig) -> Result<Option<native_tls::TlsConnector>, Box<dyn std::error::Error>> {
    if !_config.enable_tls {
        return Ok(None);
    }

    let mut builder = native_tls::TlsConnector::builder();
    
    println!("🔍 TLS config loading for arch: {}", std::env::consts::ARCH);
    
    // Get actual OpenSSL version info
    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    println!("🔍 OpenSSL: vendored (static) - modern version embedded");
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    println!("🔍 OpenSSL: system library");
    
    // Check if we can get OpenSSL version from openssl crate  
    if cfg!(any(target_arch = "arm", target_arch = "aarch64")) {
        println!("🔍 Using vendored OpenSSL for ARM compatibility");
        // Try to get the actual OpenSSL version
        #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
        {
            let version = openssl::version::version();
            println!("🔍 Embedded OpenSSL version: {}", version);
        }
    }
    
    // Try PEM identity (PKCS#8 key) if provided via env/config
    if let (Some(cert_path), Some(key_path)) = (
        _config.client_cert_path.as_ref(),
        _config.client_key_path.as_ref(),
    ) {
        println!("🔍 Loading PEM cert: {}, key: {}", cert_path, key_path);
        match (std::fs::read(cert_path), std::fs::read(key_path)) {
            (Ok(cert_bytes), Ok(key_bytes)) => {
                println!("🔍 Certificate size: {} bytes, Key size: {} bytes", cert_bytes.len(), key_bytes.len());
                
                // Check cert and key headers for debugging
                let cert_str = String::from_utf8_lossy(&cert_bytes[..std::cmp::min(100, cert_bytes.len())]);
                let key_str = String::from_utf8_lossy(&key_bytes[..std::cmp::min(100, key_bytes.len())]);
                println!("🔍 Cert header: {}", cert_str.lines().next().unwrap_or(""));
                println!("🔍 Key header: {}", key_str.lines().next().unwrap_or(""));
                
                match native_tls::Identity::from_pkcs8(&cert_bytes, &key_bytes) {
                    Ok(identity) => {
                        println!("🔐 Using client identity (PEM PKCS#8) for mTLS: cert={}, key=***", cert_path);
                        println!("🔍 Target arch: {}", std::env::consts::ARCH);
                        
                        // Debug: Show certificate subject/issuer for verification
                        if let Ok(cert_pem) = String::from_utf8(cert_bytes.clone()) {
                            if let Some(subject_line) = cert_pem.lines().find(|line| line.contains("Subject:") || line.starts_with("subject=")) {
                                println!("🔍 Cert subject: {}", subject_line);
                            }
                        }
                        
                        builder.identity(identity);
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to load PEM identity (expect 'BEGIN CERTIFICATE' and 'BEGIN PRIVATE KEY'): {}", e);
                        eprintln!("   If your key is 'BEGIN RSA PRIVATE KEY', convert to PKCS#8: openssl pkcs8 -topk8 -inform PEM -outform PEM -nocrypt -in client.key -out client_pkcs8.key");
                        eprintln!("🔍 Error details: {:?}", e);
                    }
                }
            }
            (Err(e1), Err(e2)) => {
                eprintln!("⚠️  Failed to read cert={} ({}) or key={} ({})", cert_path, e1, key_path, e2);
            }
            (Err(e), _) => eprintln!("⚠️  Failed to read cert {}: {}", cert_path, e),
            (_, Err(e)) => eprintln!("⚠️  Failed to read key {}: {}", key_path, e),
        }
    }

    // If a PKCS#12 identity is provided, use it for mTLS (Cloudflare Access mTLS, etc.)
    if let Ok(p12_path) = std::env::var("CLIENT_P12_PATH") {
        let password = std::env::var("CLIENT_P12_PASSWORD").unwrap_or_default();
        println!("🔍 Loading PKCS#12: {}", p12_path);
        let bytes = std::fs::read(&p12_path).map_err(Box::new)?;
        println!("🔍 PKCS#12 size: {} bytes", bytes.len());
        match native_tls::Identity::from_pkcs12(&bytes, &password) {
            Ok(identity) => {
                println!("🔐 Using client identity (PKCS#12) for mTLS: {}", p12_path);
                builder.identity(identity);
            }
            Err(e) => {
                eprintln!("⚠️  Failed to load PKCS#12 identity from {}: {}", p12_path, e);
                eprintln!("🔍 Error details: {:?}", e);
            }
        }
    }

    // For Mi stereo devices, we might need to be less strict about certificates
    builder.danger_accept_invalid_certs(true);
    builder.danger_accept_invalid_hostnames(true);
    
    // Force minimum TLS version for compatibility with older OpenSSL
    use native_tls::Protocol;
    builder.min_protocol_version(Some(Protocol::Tlsv12));
    
    // Add extra debugging for OpenSSL version 
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }
    
    println!("🔍 TLS config: accept_invalid_certs=true, accept_invalid_hostnames=true");
    println!("🔍 Forced minimum TLS version: 1.2 (for old OpenSSL compatibility)");
    println!("🔍 Target arch: {}", std::env::consts::ARCH);
    
    // Check if this is an ARM build with very old OpenSSL
    if cfg!(target_arch = "arm") {
        println!("⚠️  ARM detected: Using compatibility mode for embedded OpenSSL");
        // For very old OpenSSL, we may need to disable certain features
        if let Ok(tls_debug) = std::env::var("TLS_DEBUG") {
            if tls_debug == "1" {
                println!("🔍 TLS_DEBUG=1: Will enable verbose TLS debugging");
            }
        }
    }
    
    println!("⚠️  Note: Using native-tls with relaxed certificate validation for ARM compatibility");
    
    let connector = builder.build().map_err(Box::new)?;
    println!("🔍 TLS connector built successfully");
    Ok(Some(connector))
}
