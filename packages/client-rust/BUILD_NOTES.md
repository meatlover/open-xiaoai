# Build Notes for mTLS-enabled Client

## Successfully Completed

### Server-Side mTLS Configuration
✅ Nginx configured with client certificate verification
✅ CA certificate installed and working (`/etc/pki/rbdev/jp-mgmt1-trusted.crt`)
✅ WebSocket proxy to backend (`:8082`) functional
✅ mTLS verified working with curl test:
```bash
curl --cert /tmp/xiaoai.crt --key /tmp/xiaoai.key -k https://utils.harmanota.com.cn/xiaoai/health
# Returns: {"status": "ok", "migpt": "not_initialized", "uptime": 16077.687134502}
```

### Client-Side mTLS Support
✅ Rust code updated with mTLS support:
- Added `native-tls` and `tokio-native-tls` dependencies
- Created `tls_connector.rs` module
- Environment variable configuration (CLIENT_CERT_PATH, CLIENT_KEY_PATH, CA_CERT_PATH)
✅ Client certificates deployed to Xiaomi device:
- `/data/open-xiaoai/client.crt` 
- `/data/open-xiaoai/client.key`
✅ x86_64 binary built successfully for testing

### Nginx Access Log Showing Successful mTLS
```
[2025-12-06T21:36:09+09:00] conn=104/1 src=20.243.16.180 host=utils.harmanota.com.cn 
req="GET /xiaoai/health HTTP/1.1" TLSv1.3/TLS_AES_256_GCM_SHA384 
ssl_sn=036E5664247FBDF0677E8B991BB0A1AA 
ssl_dn=CN=xiaoai 
ssl_idn=CN=APAC Remote Access CA 
h_host=utils.harmanota.com.cn h_agt=curl/7.76.1 
srv=utils.harmanota.com.cn st=200 reqt=0.011 
us=127.0.0.1:8082 us_st=200 us_connt=0.009 us_respt=0.011 
sent=293 us_sent=204 us_recv=301
```

## Cross-Compilation Challenge

### Issue
Cannot cross-compile for aarch64 due to GLIBC version mismatch:
- Host Rust toolchain requires GLIBC 2.28-2.34
- Cross container has older GLIBC 2.27
- Build scripts fail during host-side compilation phase

### Attempted Solutions
1. ✅ Installed Rust toolchain (1.91.1)
2. ✅ Installed cross compilation tool
3. ✅ Installed Docker/Podman
4. ❌ Cross-compile with default image - GLIBC error
5. ❌ Cross-compile with 0.2.5 image - GLIBC error

### Working Solution Options

#### Option 1: Build on System with Compatible GLIBC
Build on a system with GLIBC 2.27 or configure Rust to use an older GLIBC:
```bash
cd /root/open-xiaoai/packages/client-rust
cross build --release --target aarch64-unknown-linux-gnu
```

#### Option 2: Use GitHub Actions or CI
Set up automated builds in CI/CD with proper cross-compilation environment.

#### Option 3: Build Natively on aarch64 System
If you have access to an aarch64 Linux system:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cd /path/to/open-xiaoai/packages/client-rust
cargo build --release
# Binary will be at: target/release/client
```

#### Option 4: Use Pre-built Toolchain
Install specific gcc-aarch64-linux-gnu toolchain and configure cross to use it.

## Deployment Steps (Once Binary is Built)

1. Copy binary to device:
```bash
/root/bin/scp-to-xiaoai target/aarch64-unknown-linux-gnu/release/client /data/open-xiaoai/client
```

2. Start client with mTLS:
```bash
/root/bin/ssh-xiaoai 'export CLIENT_CERT_PATH=/data/open-xiaoai/client.crt && export CLIENT_KEY_PATH=/data/open-xiaoai/client.key && /data/open-xiaoai/client wss://utils.harmanota.com.cn/xiaoai/ws'
```

3. Monitor logs:
```bash
tail -f /var/log/nginx/utils-access.log
journalctl -u migpt-server -f
```

## Testing

### Test mTLS from Server
```bash
curl --cert /tmp/xiaoai.crt --key /tmp/xiaoai.key -k https://utils.harmanota.com.cn/xiaoai/health
```

### Test without Client Cert (should fail)
```bash
curl -k https://utils.harmanota.com.cn/xiaoai/health
# Should return: 400 Bad Request - No required SSL certificate was sent
```

## Files Modified
- `Cargo.toml` - Added TLS dependencies
- `src/bin/client.rs` - Added TLS configuration detection
- `src/services/connect/mod.rs` - Added tls_connector module
- `src/services/connect/tls_connector.rs` - New TLS configuration module
- `MTLS_SETUP.md` - Documentation
- `BUILD_NOTES.md` - This file

## Commit
```bash
cd /root/open-xiaoai
git add -A
git commit -m "Add mTLS support and build notes"
git push origin main
```
