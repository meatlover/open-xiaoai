# mTLS (Mutual TLS) Support for Open-XiaoAI Client

## Overview

The Rust client now supports mTLS (mutual TLS authentication) where the client presents a certificate to the server during the TLS handshake. This is useful for enhanced security when connecting to servers that require client certificate authentication.

## Features

- **Optional mTLS**: The client works both with and without mTLS configuration
- **Multiple certificate formats**: Supports PEM, DER, and PKCS#12 (.p12/.pfx) formats
- **Custom CA certificates**: Can use custom CA certificates for server verification
- **Environment-based configuration**: Easy configuration through environment variables

## Configuration

The client checks for the following environment variables:

### Required for mTLS

- `CLIENT_CERT_PATH`: Path to the client certificate file
- `CLIENT_KEY_PATH`: Path to the client private key file

### Optional

- `CA_CERT_PATH`: Path to custom CA certificate (for server verification)
- `CLIENT_CERT_PASSWORD`: Password for PKCS#12 (.p12/.pfx) files
- `ACCEPT_INVALID_CERTS`: Set to any value to skip certificate verification (testing only)

## Usage Examples

### Basic WSS connection (no mTLS)

```bash
/data/open-xiaoai/client wss://utils.harmanota.com.cn/xiaoai/ws
```

### With mTLS using PEM files

```bash
export CLIENT_CERT_PATH=/data/open-xiaoai/client.pem
export CLIENT_KEY_PATH=/data/open-xiaoai/client-key.pem
/data/open-xiaoai/client wss://secure.example.com/ws
```

### With mTLS using PKCS#12 file

```bash
export CLIENT_CERT_PATH=/data/open-xiaoai/client.p12
export CLIENT_KEY_PATH=/data/open-xiaoai/client.p12  # Same file for p12
export CLIENT_CERT_PASSWORD=mypassword
/data/open-xiaoai/client wss://secure.example.com/ws
```

### With custom CA certificate

```bash
export CLIENT_CERT_PATH=/data/open-xiaoai/client.pem
export CLIENT_KEY_PATH=/data/open-xiaoai/client-key.pem
export CA_CERT_PATH=/data/open-xiaoai/ca.pem
/data/open-xiaoai/client wss://secure.example.com/ws
```

## Certificate Preparation

### Converting certificates to PKCS#12 format

If you have PEM certificates and want to convert to PKCS#12:

```bash
openssl pkcs12 -export -out client.p12 \
  -inkey client-key.pem \
  -in client.pem \
  -certfile ca.pem \
  -password pass:mypassword
```

### Extracting from PKCS#12 to PEM

```bash
# Extract certificate
openssl pkcs12 -in client.p12 -out client.pem -clcerts -nokeys

# Extract private key
openssl pkcs12 -in client.p12 -out client-key.pem -nocerts -nodes
```

## Server Configuration

### Nginx with mTLS

If your nginx server requires client certificates globally but you want to disable it for specific paths:

```nginx
server {
    listen 443 ssl;
    server_name example.com;
    
    # Global mTLS settings
    ssl_verify_client on;
    ssl_client_certificate /path/to/ca.crt;
    
    # Disable mTLS for specific location
    location /xiaoai/ {
        ssl_verify_client off;  # Allow connections without client cert
        proxy_pass http://backend;
    }
}
```

## Implementation Details

### Dependencies Added

```toml
tokio-tungstenite = { version = "0.26.2", features = ["native-tls"] }
tokio-native-tls = "0.3"
native-tls = "0.2"
```

### Files Modified

- `Cargo.toml`: Added TLS dependencies
- `src/bin/client.rs`: Updated connection logic to support mTLS
- `src/services/connect/mod.rs`: Added tls_connector module
- `src/services/connect/tls_connector.rs`: New module for TLS configuration

### How It Works

1. On startup, the client checks environment variables for certificate paths
2. If `CLIENT_CERT_PATH` and `CLIENT_KEY_PATH` are set, mTLS is enabled
3. The TLS connector loads the certificates and builds a custom TLS configuration
4. WebSocket connection uses the custom connector for the TLS handshake
5. If no certificates are configured, falls back to basic TLS (server verification only)

## Building

To build the client with mTLS support:

```bash
cd packages/client-rust
cargo build --release --target armv7-unknown-linux-gnueabihf
```

Or for aarch64 (OH2P model):

```bash
cargo build --release --target aarch64-unknown-linux-gnu
```

## Troubleshooting

### Connection fails with certificate error

- Check that certificate and key files are readable
- Verify certificate is valid and not expired
- Check certificate format matches the file extension

### "Failed to build TLS connector" error

- Verify `CLIENT_CERT_PATH` and `CLIENT_KEY_PATH` point to valid files
- Check file permissions
- For PKCS#12 files, ensure `CLIENT_CERT_PASSWORD` is correct

### Server rejects connection

- Verify the server expects the client certificate CA
- Check that the certificate hasn't been revoked
- Ensure the certificate CN or SAN matches what the server expects

## Security Notes

- Store private keys securely with appropriate file permissions (chmod 600)
- Use strong passwords for PKCS#12 files
- Never commit certificates or keys to version control
- Only use `ACCEPT_INVALID_CERTS` for testing, never in production
- Consider using certificate rotation and expiration monitoring
