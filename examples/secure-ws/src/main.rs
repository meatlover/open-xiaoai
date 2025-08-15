mod server;
mod tls;

#[tokio::main]
async fn main() {
    // Configure TLS from environment variables if present
    // SECURE_WS_ENABLE_TLS=true|false
    // SECURE_WS_REQUIRE_CLIENT_AUTH=true|false
    // SECURE_WS_CERT_CHAIN_PEM, SECURE_WS_PRIV_KEY_PEM, SECURE_WS_CLIENT_CA_PEM as PEM strings
    let enable_tls = std::env::var("SECURE_WS_ENABLE_TLS").ok().map(|v| v == "true").unwrap_or(false);
    let require_client_auth = std::env::var("SECURE_WS_REQUIRE_CLIENT_AUTH").ok().map(|v| v == "true").unwrap_or(false);
    let cert_chain_pem = std::env::var("SECURE_WS_CERT_CHAIN_PEM").ok().map(|s| s.into_bytes());
    let priv_key_pem = std::env::var("SECURE_WS_PRIV_KEY_PEM").ok().map(|s| s.into_bytes());
    let client_ca_pem = std::env::var("SECURE_WS_CLIENT_CA_PEM").ok().map(|s| s.into_bytes());

    tls::set_tls(tls::TlsConfig { enable_tls, require_client_auth, cert_chain_pem, priv_key_pem, client_ca_pem });
    server::AppServer::run().await;
}
