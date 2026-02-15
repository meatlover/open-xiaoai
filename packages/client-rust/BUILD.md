# Building the Rust Client

This document describes how to build and deploy the XiaoAi Rust client using GitHub Actions.

## Quick Start

### 1. Trigger Build

Push to GitHub to trigger the build workflow:

```bash
git add .
git commit -m "Add AI-Brain Phase 1 implementation"
git push origin main
```

Or manually trigger via GitHub UI:
- Go to **Actions** tab → **Build Rust Client** → **Run workflow**

### 2. Download Built Binary

After ~5-10 minutes, download the artifact:

**Option A: Via GitHub UI**
1. Go to **Actions** tab
2. Click on the latest workflow run
3. Download **xiaoai-client-deployment** artifact (includes deploy script)

**Option B: Via GitHub CLI**
```bash
gh run list --workflow=build-rust-client.yml --limit 1
gh run download <run-id> -n xiaoai-client-deployment
```

### 3. Deploy to Device

Extract the deployment package:
```bash
tar -xzf xiaoai-client-armv7.tar.gz
```

Deploy using the included script:
```bash
./deploy.sh <device-ip> <server-ip>
```

Or manually:
```bash
scp client root@<device-ip>:/data/open-xiaoai/client
ssh root@<device-ip> 'chmod +x /data/open-xiaoai/client'
echo 'ws://<server-ip>:9000' | ssh root@<device-ip> 'cat > /data/open-xiaoai/server.txt'
```

### 4. Test on Device

```bash
ssh root@<device-ip>
/data/open-xiaoai/client $(cat /data/open-xiaoai/server.txt)
```

Then speak to your XiaoAi device. You should see ASR events being forwarded to the AI-Brain server.

## Build Artifacts

The workflow produces:

### ARMv7 Device Binary
- **Name**: `xiaoai-client-armv7` 
- **Target**: `armv7-unknown-linux-gnueabihf` (for XiaoAi device)
- **Size**: ~2-5 MB (stripped)
- **Retention**: 30 days

### Deployment Package
- **Name**: `xiaoai-client-deployment`
- **Contents**: 
  - `client` - ARMv7 binary
  - `deploy.sh` - Quick deployment script
  - `AI_BRAIN_INTEGRATION.md` - Integration guide
- **Format**: `xiaoai-client-armv7.tar.gz`
- **Retention**: 30 days

### Native Test Binaries (optional)
- **Name**: `xiaoai-client-ubuntu-latest` / `xiaoai-client-macos-latest`
- **Purpose**: Local testing without device
- **Retention**: 7 days

## Workflow Details

### Triggers

The build runs on:
- **Push** to `main`, `dev`, or `feat/*` branches (when Rust files change)
- **Pull requests** to `main` (when Rust files change)
- **Manual trigger** via Actions UI (workflow_dispatch)

### Build Steps

1. **Install Rust toolchain** - stable with armv7 target
2. **Install cross** - cross-compilation tool using Docker
3. **Build ARMv7 binary** - via `cross build --release --target armv7-unknown-linux-gnueabihf`
4. **Strip binary** - remove debug symbols to reduce size
5. **Create deployment package** - bundle binary + docs + deploy script
6. **Upload artifacts** - store for download

### Testing

Native builds (Ubuntu/macOS) also run tests:
```bash
cargo test --release
```

This validates:
- ASR text extraction logic
- Session ID generation
- Message serialization

## Troubleshooting

### Build fails with "cross: command not found"
- The workflow installs `cross` automatically - check the workflow logs
- May need to update cross-rs repository URL if it changes

### Binary too large (>10 MB)
- Ensure strip step runs successfully
- Check that `--release` flag is used (optimizations enabled)

### Device reports "Exec format error"
- Verify target is `armv7-unknown-linux-gnueabihf` not `armv7l` or other variants
- Check with: `file client` (should show "ARM, EABI5")

### Cannot download artifact
- Artifacts expire after retention period (7-30 days)
- Re-run the workflow to rebuild
- Ensure you have repository access (private repos need auth)

## Local Development (Alternative)

If you need faster iteration for development:

### macOS (M-series)
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build native (won't work on device, but good for testing logic)
cd packages/client-rust
cargo build --release
cargo test
```

### Cross-compilation (requires Docker)
```bash
# Install cross
cargo install cross --git https://github.com/cross-rs/cross

# Build for device
cd packages/client-rust
cross build --release --target armv7-unknown-linux-gnueabihf
```

## Next Steps

After deploying:
1. **Start AI-Brain server**: See `server/README.md`
2. **Start LM Studio**: Configure OpenAI-compatible endpoint on port 1234
3. **Test end-to-end**: Speak to device and verify LLM responses
4. **Monitor logs**: Check `/tmp/mico_aivs_lab/instruction.log` on device

For full integration details, see `packages/client-rust/AI_BRAIN_INTEGRATION.md`.
