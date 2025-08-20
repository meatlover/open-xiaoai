# Open-XiaoAI Utilities

This directory contains utility scripts for managing Open-XiaoAI deployments and features.

## Scripts Overview

### 🎙️ KWS (Keyword Wake-up Service) Scripts

- `deploy-kws.sh` - Complete KWS deployment script
- `setup-kws.sh` - Basic KWS setup on Mi device
- `manage-wake-words.sh` - Wake words and replies management
- `keywords.py` - Chinese text to pinyin keywords generator

### 🚀 Build and Deployment Scripts

- `remote-build-client.sh` - Remote build script for Rust client
- `device-copy.sh` - File transfer utility for Mi devices

### 🔄 Boot and Auto-Start Scripts

- `boot.sh` - Main boot script that runs on Mi device startup
- `install-boot.sh` - Helper script to install boot.sh to Mi device

## Quick Start Guide

### 🎙️ Deploy KWS (Customized Wake Words)

```bash
# Complete KWS deployment
./deploy-kws.sh

# This will:
# - Check dependencies and connectivity
# - Install Sherpa-ONNX KWS system
# - Configure default wake words (天猫精灵, 小度小度, etc.)
# - Enable auto-start
# - Verify deployment
```

### ⚙️ Manage Custom Wake Words

```bash
# Download current config from device
./manage-wake-words.sh download

# Edit keywords locally
./manage-wake-words.sh edit-keywords

# Upload keywords to device
./manage-wake-words.sh upload-keywords

# Test with debug mode
./manage-wake-words.sh debug
```

### 🔨 Build and Deploy Client

```bash
# Build remotely (bypass China firewall)
./remote-build-client.sh release

# Deploy to device
./device-copy.sh to /tmp/client /data/open-xiaoai/client

# Run on device
ssh root@192.168.143.211 "cd /data/open-xiaoai && ./client config.json --debug"
```

## Environment Variables

All scripts support these environment variables:
- `DEVICE_IP` - Device IP address (default: 192.168.143.211)
- `DEVICE_USER` - Device username (default: root)
- `DEVICE_PASS` - Device password (default: open-xiaoai)

## Boot Script Features

### 🚀 Auto-Detection Mode
The boot script automatically detects the running mode from `config.json`:
- **Direct Mode** (`"mode": "direct"`): Runs unified `client` with direct LLM API calls
- **Proxy Mode** (`"mode": "proxy"`): Runs unified `client` with server proxy communication

### 📥 Auto-Download
- Automatically downloads the appropriate client binary from GitHub releases
- Falls back to Gitee mirror if GitHub is unavailable
- Downloads default config if none exists

### 🔄 Process Management
- Kills existing client processes before starting new ones
- Runs client in background with proper PID tracking

### 🔧 Configuration Support
- Reads configuration from `/data/open-xiaoai/config.json`
- Supports both new JSON config and legacy `server.txt` for backward compatibility
- Creates default config if none exists

## Usage

### Method 1: Using Install Script (Recommended)

```bash
# Install to device at 192.168.1.100 (default)
./install-boot.sh

# Install to device at custom IP
./install-boot.sh 192.168.1.50
```

### Method 2: Manual Installation

```bash
# Copy boot script to device
scp boot.sh root@your-device-ip:/data/boot.sh

# Make executable
ssh root@your-device-ip "chmod +x /data/boot.sh"

# Test the script
ssh root@your-device-ip "sh -n /data/boot.sh"
```

### Method 3: Direct Upload via SSH

```bash
# SSH to your Mi device
ssh root@your-device-ip

# Download boot script directly
curl -L https://raw.githubusercontent.com/meatlover/open-xiaoai/main/utils/boot.sh -o /data/boot.sh
chmod +x /data/boot.sh
```

## Configuration

### Create Config File (Direct Mode)

```bash
ssh root@your-device-ip
mkdir -p /data/open-xiaoai
cat > /data/open-xiaoai/config.json << 'EOF'
{
  "mode": "direct",
  "openai": {
    "baseURL": "https://api.302.ai/v1",
    "apiKey": "your-api-key-here",
    "model": "gpt-4"
  }
}
EOF
```

### Create Config File (Proxy Mode)

```bash
ssh root@your-device-ip
mkdir -p /data/open-xiaoai
cat > /data/open-xiaoai/config.json << 'EOF'
{
  "mode": "proxy",
  "server": {
    "url": "http://your-server:4399"
  }
}
EOF
```

## Testing

### Check if Script is Working

```bash
# Reboot device
ssh root@your-device-ip "reboot"

# Wait 1-2 minutes, then check if client is running
ssh root@your-device-ip "ps | grep client"

# Check logs
ssh root@your-device-ip "tail -f /var/log/messages | grep open-xiaoai"
```

### Manual Script Execution

```bash
# Run script manually for testing
ssh root@your-device-ip "/data/boot.sh"

# Check if process started
ssh root@your-device-ip "ps | grep client"
```

## Troubleshooting

### Script Not Running on Boot

1. **Check if script exists and is executable:**
   ```bash
   ssh root@your-device-ip "ls -la /data/boot.sh"
   ```

2. **Check script syntax:**
   ```bash
   ssh root@your-device-ip "sh -n /data/boot.sh"
   ```

3. **Run script manually:**
   ```bash
   ssh root@your-device-ip "/data/boot.sh"
   ```

### Download Issues

1. **Check network connectivity:**
   ```bash
   ssh root@your-device-ip "ping -c 3 github.com"
   ```

2. **Check if GitHub is accessible:**
   ```bash
   ssh root@your-device-ip "curl -I https://github.com"
   ```

3. **Use manual download:**
   ```bash
   # Download client manually and place in /data/open-xiaoai/
   ```

### Client Not Starting

1. **Check configuration:**
   ```bash
   ssh root@your-device-ip "cat /data/open-xiaoai/config.json"
   ```

2. **Check client binary:**
   ```bash
   ssh root@your-device-ip "ls -la /data/open-xiaoai/"
   ```

3. **Run client manually:**
   ```bash
   ssh root@your-device-ip "cd /data/open-xiaoai && ./client config.json"
   ```

## Advanced Usage

### Custom Download URLs

Edit the boot.sh script to use custom download URLs:

```bash
# Change this line in boot.sh
DOWNLOAD_BASE_URL="https://your-custom-server.com/releases"
```

### Custom Work Directory

```bash
# Change this line in boot.sh
WORK_DIR="/your/custom/path"
```

### Debug Mode

To see detailed output, modify the first line in boot.sh:

```bash
# Remove the redirect to see output
# exec > /dev/null 2>&1
```

Then check logs with:
```bash
ssh root@your-device-ip "tail -f /var/log/messages"
```
