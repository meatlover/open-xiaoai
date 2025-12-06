# Build Workaround for GLIBC Incompatibility

## Problem

Cross-compilation on RHEL 9 fails because:
- Host Rust toolchain (v1.91.1) was compiled against GLIBC 2.34
- Build scripts are compiled for the host, then executed inside Docker container
- Docker containers have older GLIBC (2.27), cannot run host-compiled build scripts

## Solutions

### Option 1: Build on Ubuntu 20.04+ (Recommended)

On a Ubuntu 20.04 or newer system:

```bash
# Install dependencies
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -y
sudo apt-get install -y build-essential pkg-config libssl-dev

# Clone and build
git clone https://github.com/meatlover/open-xiaoai.git
cd open-xiaoai/packages/client-rust
cargo build --release --target aarch64-unknown-linux-gnu

# Binary will be at: target/aarch64-unknown-linux-gnu/release/client
```

### Option 2: GitHub Actions (Automated)

Create `.github/workflows/build.yml`:

```yaml
name: Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: aarch64-unknown-linux-gnu
      - name: Build
        run: |
          cd packages/client-rust
          cargo build --release --target aarch64-unknown-linux-gnu
      - uses: actions/upload-artifact@v3
        with:
          name: client-aarch64
          path: packages/client-rust/target/aarch64-unknown-linux-gnu/release/client
```

### Option 3: Native Build on Xiaomi Device

If you have shell access to the device:

```bash
# On Xiaomi device (if possible)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
rustup target add aarch64-unknown-linux-gnu

# Clone and build
git clone https://github.com/meatlover/open-xiaoai.git
cd open-xiaoai/packages/client-rust
cargo build --release

# Binary: target/release/client
```

### Option 4: Remote Build Server

Use a remote machine with compatible GLIBC:

```bash
# Transfer source code
rsync -av /root/open-xiaoai/ user@build-server:~/open-xiaoai/

# SSH and build
ssh user@build-server
cd ~/open-xiaoai/packages/client-rust
cargo build --release --target aarch64-unknown-linux-gnu

# Copy back
scp target/aarch64-unknown-linux-gnu/release/client your-server:/root/open-xiaoai/packages/client-rust/target/aarch64-unknown-linux-gnu/release/
```

## Deployment

Once binary is built:

```bash
/root/bin/scp-to-xiaoai target/aarch64-unknown-linux-gnu/release/client /data/open-xiaoai/client
```

## Testing with mTLS

```bash
/root/bin/ssh-xiaoai '
CLIENT_CERT_PATH=/data/open-xiaoai/client.crt \
CLIENT_KEY_PATH=/data/open-xiaoai/client.key \
/data/open-xiaoai/client wss://utils.harmanota.com.cn/xiaoai/ws
'
```
