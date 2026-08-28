# End-to-End Testing Summary

## Current Status

### ✅ Completed
1. **Disk Space Expansion**
   - Extended partition from 68GB to 137GB
   - Expanded root filesystem: 12GB → 32GB (22GB free)
   - Expanded /var: 8GB → 38GB (34GB free)

2. **Server-Side mTLS Configuration** 
   - Nginx configured with client certificate verification
   - CA certificate installed and trusted
   - WebSocket proxy to backend operational
   - **Verified Working**: `curl --cert /tmp/xiaoai.crt --key /tmp/xiaoai.key -k https://utils.harmanota.com.cn/xiaoai/health` returns HTTP 200

3. **Client-Side mTLS Code**
   - Rust dependencies added (native-tls, tokio-native-tls)
   - TLS connector module implemented
   - Environment variable configuration support
   - Code committed (483ae76, 4a1d4bf)

4. **Certificates Deployed**
   - Client cert: `/data/open-xiaoai/client.crt`
   - Client key: `/data/open-xiaoai/client.key`
   - Startup script created with mTLS env vars

### ⚠️ Blocking Issue
**Cross-Compilation GLIBC Incompatibility**
- Host system (RHEL 9) has Rust toolchain compiled against GLIBC 2.34
- Build scripts compile for host, then try to run in Docker container
- Docker containers (Ubuntu 18.04/20.04) have older GLIBC (2.27/2.31)
- Even with Rust installed inside container, host's cargo is used

**Error Pattern:**
```
/build/target/release/build/libc-xxx/build-script-build: 
version `GLIBC_2.32' not found
version `GLIBC_2.33' not found  
version `GLIBC_2.34' not found
```

## End-to-End Test Plan

### Without mTLS (Current Binary)
The existing binary from gitee doesn't support mTLS but can test basic connectivity:

```bash
# 1. Temporarily disable mTLS on server
# Edit /root/sp-ota-cn-puppet/hieradata/node/jp-mgmt1.yaml
# Add under utils-xiaoai location:
location_cfg_append:
  ssl_verify_client: 'off'

# 2. Apply puppet
cd /root/sp-ota-cn-puppet && sudo puppet apply

# 3. Test client connection
/root/bin/ssh-xiaoai '/data/open-xiaoai/client wss://utils.harmanota.com.cn/xiaoai/ws'

# 4. Monitor logs
tail -f /var/log/nginx/utils-access.log
journalctl -u migpt-server -f
```

### With mTLS (Requires New Binary)
**Build Options:**

#### Option A: Build on Ubuntu 20.04 Machine
```bash
# On Ubuntu 20.04+ system
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -y
sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libssl-dev pkg-config

git clone https://github.com/meatlover/open-xiaoai.git
cd open-xiaoai/packages/client-rust
rustup target add aarch64-unknown-linux-gnu

cargo build --release --target aarch64-unknown-linux-gnu

# Deploy
scp target/aarch64-unknown-linux-gnu/release/client jp-mgmt1:/tmp/
ssh jp-mgmt1 "/root/bin/scp-to-xiaoai /tmp/client /data/open-xiaoai/client"
```

#### Option B: GitHub Actions CI/CD
Create `.github/workflows/build.yml` in repository:

```yaml
name: Build Open-XiaoAI Client

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: aarch64-unknown-linux-gnu
          override: true
      
      - name: Install cross-compilation tools
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
      
      - name: Build
        run: |
          cd packages/client-rust
          cargo build --release --target aarch64-unknown-linux-gnu
      
      - uses: actions/upload-artifact@v3
        with:
          name: client-aarch64
          path: packages/client-rust/target/aarch64-unknown-linux-gnu/release/client
```

#### Option C: Remote Build Server
```bash
# Transfer code to build server
rsync -av /root/open-xiaoai/ build-server:~/open-xiaoai/

# SSH and build  
ssh build-server
cd ~/open-xiaoai/packages/client-rust
cargo build --release --target aarch64-unknown-linux-gnu

# Copy back
scp target/aarch64-unknown-linux-gnu/release/client jp-mgmt1:/tmp/
```

### Complete E2E Test with mTLS

Once you have the mTLS-enabled binary:

```bash
# 1. Deploy new binary
/root/bin/scp-to-xiaoai /tmp/client /data/open-xiaoai/client

# 2. Ensure mTLS is enabled on server (should already be)
# /root/sp-ota-cn-puppet/hieradata/node/jp-mgmt1.yaml has:
# ssl_verify_client: 'on'
# NO location_cfg_append with 'off'

# 3. Start client with mTLS
/root/bin/ssh-xiaoai '
export CLIENT_CERT_PATH=/data/open-xiaoai/client.crt
export CLIENT_KEY_PATH=/data/open-xiaoai/client.key  
/data/open-xiaoai/client wss://utils.harmanota.com.cn/xiaoai/ws
'

# 4. Verify connection in logs
# Should see in /var/log/nginx/utils-access.log:
# ssl_sn=036E5664247FBDF0677E8B991BB0A1AA
# ssl_dn=CN=xiaoai
# ssl_idn=CN=APAC Remote Access CA
# st=101 (WebSocket upgrade)

# 5. Test voice command
# Speak to Xiaomi device: "小爱同学"
# Should connect to OpenAI via migpt-server
```

### Verification Commands

```bash
# Check nginx logs for mTLS
tail -20 /var/log/nginx/utils-access.log | grep xiaoai

# Check migpt-server logs
journalctl -u migpt-server -n 50

# Test from server with client cert
curl --cert /tmp/xiaoai.crt --key /tmp/xiaoai.key -k \
  https://utils.harmanota.com.cn/xiaoai/health

# Test WebSocket upgrade (should return 101)
curl --cert /tmp/xiaoai.crt --key /tmp/xiaoai.key -k -v \
  -H "Connection: Upgrade" -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  https://utils.harmanota.com.cn/xiaoai/ws
```

## Summary

**Server-side mTLS**: ✅ Fully functional  
**Client-side mTLS code**: ✅ Complete and committed  
**Binary deployment**: ⚠️ Blocked by build environment GLIBC issue  

**Next Steps**: Build aarch64 binary on Ubuntu 20.04+ system or use GitHub Actions, then deploy and test end-to-end.
