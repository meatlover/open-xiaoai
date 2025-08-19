# Boot Script for Mi Device Auto-Start

This directory contains scripts to automatically start the Open-XiaoAI client on Mi device boot.

## Files

- `boot.sh` - Main boot script that runs on Mi device startup
- `install-boot.sh` - Helper script to install boot.sh to Mi device
- `README.md` - This documentation

## Features

### 🚀 Auto-Detection Mode
The boot script automatically detects the running mode from `config.json`:
- **Direct Mode** (`"mode": "direct"`): Downloads and runs `multi_mode_client`
- **Proxy Mode** (`"mode": "proxy"`): Downloads and runs `http_client`

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
ssh root@your-device-ip "ps | grep -E '(http_client|multi_mode_client)'"

# Check logs
ssh root@your-device-ip "tail -f /var/log/messages | grep open-xiaoai"
```

### Manual Script Execution

```bash
# Run script manually for testing
ssh root@your-device-ip "/data/boot.sh"

# Check if process started
ssh root@your-device-ip "ps | grep -E '(http_client|multi_mode_client)'"
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
   ssh root@your-device-ip "cd /data/open-xiaoai && ./http_client http://your-server:4399"
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
