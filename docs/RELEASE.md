# Release and Distribution Guide

This guide explains how to build, release, and distribute Open-XiaoAI client binaries.

## 🚀 Automated Releases (Recommended)

### GitHub Actions Release

The project includes automated GitHub Actions workflows that build and release binaries:

#### Trigger a Release

**Method 1: Git Tag (Recommended)**
```bash
# Create and push a tag
git tag v1.0.0
git push origin v1.0.0

# This automatically triggers the release workflow
```

**Method 2: Manual Workflow Dispatch**
1. Go to GitHub Actions tab in your repository
2. Select "Build and Release Client Binaries" workflow
3. Click "Run workflow"
4. Enter the desired tag (e.g., `v1.0.0`)
5. Click "Run workflow"

#### What Gets Built

The automated workflow builds binaries for multiple architectures:

- **ARM (Mi Devices)**:
  - `armv7-unknown-linux-gnueabihf` (Most Mi smart speakers)
  - `arm-unknown-linux-gnueabihf` (Older ARM devices)
  - `aarch64-unknown-linux-gnu` (ARM64 devices)

- **x86_64 (Testing)**:
  - `x86_64-unknown-linux-gnu` (Local development)

#### Release Assets

Each release includes:

- `http_client` - HTTP client for proxy mode
- `http_server` - HTTP server for LLM integration  
- `multi_mode_client` - Multi-mode client (if available)
- `config.template.json` - Configuration template
- `boot.sh` - Auto-start script for Mi devices
- `open-xiaoai-{arch}.tar.gz` - Architecture-specific packages

## 🔨 Manual Building

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cross for cross-compilation
cargo install cross --git https://github.com/cross-rs/cross

# Install target architectures
rustup target add armv7-unknown-linux-gnueabihf
rustup target add arm-unknown-linux-gnueabihf
rustup target add aarch64-unknown-linux-gnu
```

### Using Build Script

```bash
# Build for ARMv7 (most Mi devices)
./scripts/build-release.sh v1.0.0 armv7-unknown-linux-gnueabihf

# Build for x86_64 (testing)
./scripts/build-release.sh v1.0.0 x86_64-unknown-linux-gnu

# Build with auto-generated version
./scripts/build-release.sh
```

### Manual Commands

```bash
cd packages/client-rust

# Build for ARMv7 (most compatible with Mi devices)
cross build --release --target armv7-unknown-linux-gnueabihf --bin http_client
cross build --release --target armv7-unknown-linux-gnueabihf --bin http_server

# Build for local testing
cargo build --release --bin http_client
cargo build --release --bin http_server
```

## 📦 Distribution Options

### 1. GitHub Releases (Primary)

**Automatic Distribution:**
- Binaries are automatically uploaded to GitHub Releases
- Users can download via direct URLs
- Our `boot.sh` script downloads from GitHub by default

**URLs:**
```
https://github.com/meatlover/open-xiaoai/releases/latest/download/http_client
https://github.com/meatlover/open-xiaoai/releases/latest/download/http_server
https://github.com/meatlover/open-xiaoai/releases/latest/download/multi_mode_client
```

### 2. Gitee Mirror (China Users)

For users in China who can't access GitHub:

```bash
# Example upload to Gitee (manual)
curl -X POST "https://gitee.com/your-username/artifacts/releases" \
  -H "Authorization: token YOUR_GITEE_TOKEN" \
  -F "file=@http_client"
```

### 3. Custom Artifactory/CDN

You can upload binaries to your own CDN or artifactory:

```bash
# Example: Upload to AWS S3
aws s3 cp release/ s3://your-bucket/open-xiaoai/latest/ --recursive

# Example: Upload to your server
rsync -av release/ user@your-server:/var/www/downloads/open-xiaoai/
```

## 🔧 Configuration for Different Sources

### Update boot.sh for Custom Sources

Edit `utils/boot.sh` to use your custom download URL:

```bash
# Change this line
DOWNLOAD_BASE_URL="https://your-cdn.com/open-xiaoai"

# Or use environment variable
DOWNLOAD_BASE_URL="${OPEN_XIAOAI_CDN:-https://github.com/meatlover/open-xiaoai/releases/latest/download}"
```

### Multi-Source Support

The current `boot.sh` includes fallback sources:

```bash
# Primary: GitHub
curl -L -o "$CLIENT_BIN" "$DOWNLOAD_BASE_URL/$CLIENT_NAME" || {
    # Fallback: Gitee
    curl -L -o "$CLIENT_BIN" "https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-client/$CLIENT_NAME"
}
```

## 🧪 Testing Releases

### Local Testing

```bash
# Test on local machine (x86_64)
./scripts/build-release.sh v1.0.0 x86_64-unknown-linux-gnu

# Test binaries
cd release/
./http_server &
./http_client http://localhost:4399
```

### Device Testing

```bash
# Upload to Mi device for testing
scp release/http_client root@192.168.1.100:/data/
ssh root@192.168.1.100 "chmod +x /data/http_client"

# Test on device
ssh root@192.168.1.100 "/data/http_client http://your-server:4399"
```

### Automated Testing

The GitHub Actions workflow includes basic testing:

```yaml
- name: Run tests
  working-directory: packages/client-rust
  run: cargo test --verbose
```

## 📋 Release Checklist

### Before Release

- [ ] All tests pass locally
- [ ] Version number updated in relevant files
- [ ] CHANGELOG.md updated
- [ ] Documentation updated
- [ ] Cross-compilation tested

### Release Process

- [ ] Create git tag: `git tag v1.0.0`
- [ ] Push tag: `git push origin v1.0.0`
- [ ] Wait for GitHub Actions to complete
- [ ] Verify release assets are uploaded
- [ ] Test download URLs
- [ ] Update boot.sh if needed

### After Release

- [ ] Test auto-download on Mi device
- [ ] Update documentation with new version
- [ ] Announce release in community
- [ ] Monitor for issues

## 🔍 Troubleshooting

### Build Issues

**Cross-compilation fails:**
```bash
# Update cross
cargo install cross --git https://github.com/cross-rs/cross --force

# Clear cache
cargo clean
```

**Missing dependencies:**
```bash
# Install build tools
sudo apt-get update
sudo apt-get install -y gcc-arm-linux-gnueabihf gcc-aarch64-linux-gnu
```

### GitHub Actions Issues

**Workflow not triggering:**
- Check if tag follows `v*` pattern
- Ensure `.github/workflows/release.yml` is in main branch
- Check repository permissions

**Build failures:**
- Check GitHub Actions logs
- Verify all dependencies are available
- Test locally first

### Download Issues

**GitHub releases not accessible:**
- Set up Gitee mirror
- Use custom CDN
- Check corporate firewall settings

**Boot script fails:**
- Verify download URLs are correct
- Check device internet connectivity
- Test manual download

## 🔗 External Resources

- [Cross-compilation guide](https://github.com/cross-rs/cross)
- [GitHub Actions documentation](https://docs.github.com/en/actions)
- [Rust target platform support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
