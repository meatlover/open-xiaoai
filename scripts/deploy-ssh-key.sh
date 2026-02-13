#!/bin/bash
# Deploy SSH public key to Xiaomi Smart Speaker Pro (OH2P)
# Usage: ./deploy-ssh-key.sh <device-ip> <public-key-file>

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SSH_OPTS="-o HostKeyAlgorithms=+ssh-rsa -o StrictHostKeyChecking=no -o ConnectTimeout=10"
SSH_USER="root"
DEFAULT_PASSWORD="open-xiaoai"

# Helper functions
error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    exit 1
}

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

usage() {
    cat << EOF
Deploy SSH public key authentication to OH2P device

Usage: $0 <device-ip> <public-key-file>

Arguments:
  device-ip       IP address of the OH2P device (e.g., 192.168.31.140)
  public-key-file Path to your SSH public key (e.g., ~/.ssh/id_rsa.pub)

Example:
  $0 192.168.31.140 ~/.ssh/xiaoai_key.pub

Notes:
  - Device must be running patched firmware with SSH enabled
  - Default password is: $DEFAULT_PASSWORD
  - You will be prompted for the SSH password during setup
EOF
    exit 1
}

# Validate arguments
[ $# -ne 2 ] && usage

DEVICE_IP="$1"
PUBKEY_FILE="$2"

# Expand tilde in path
PUBKEY_FILE="${PUBKEY_FILE/#\~/$HOME}"

# Validate inputs
[ -z "$DEVICE_IP" ] && error "Device IP cannot be empty"
[ ! -f "$PUBKEY_FILE" ] && error "Public key file not found: $PUBKEY_FILE"

# Validate public key format
if ! grep -qE '^(ssh-rsa|ssh-ed25519|ecdsa-sha2-nistp256) ' "$PUBKEY_FILE"; then
    error "Invalid public key format in: $PUBKEY_FILE"
fi

info "Deploying SSH key to device: $DEVICE_IP"
info "Using public key: $PUBKEY_FILE"

# Function to execute SSH command
ssh_exec() {
    ssh $SSH_OPTS "$SSH_USER@$DEVICE_IP" "$@"
}

# Test SSH connectivity
info "Testing SSH connection..."
if ! ssh_exec "echo 'Connected successfully'" >/dev/null 2>&1; then
    error "Cannot connect to device at $DEVICE_IP. Please check:\n  - Device is powered on and connected to network\n  - IP address is correct\n  - SSH password is correct (default: $DEFAULT_PASSWORD)"
fi
info "✓ SSH connection successful"

# Detect SSH daemon type
info "Detecting SSH daemon..."
DAEMON_TYPE=$(ssh_exec "ps | grep -E 'dropbear|sshd' | grep -v grep | head -n1" | awk '{print $5}')
if echo "$DAEMON_TYPE" | grep -q "dropbear"; then
    info "✓ Detected Dropbear SSH daemon"
    SSH_DAEMON="dropbear"
elif echo "$DAEMON_TYPE" | grep -q "sshd"; then
    info "✓ Detected OpenSSH daemon"
    SSH_DAEMON="openssh"
else
    warn "Could not detect SSH daemon type, assuming Dropbear"
    SSH_DAEMON="dropbear"
fi

# Check available disk space in /data
info "Checking available disk space..."
DISK_AVAILABLE=$(ssh_exec "df -h /data | tail -n1 | awk '{print \$4}'")
info "Available space in /data: $DISK_AVAILABLE"

# Create SSH directory structure
info "Creating SSH directory structure..."
ssh_exec "mkdir -p /data/.ssh && chmod 700 /data/.ssh"
info "✓ Created /data/.ssh"

# Upload public key
info "Uploading public key..."
cat "$PUBKEY_FILE" | ssh_exec "cat > /data/.ssh/authorized_keys && chmod 600 /data/.ssh/authorized_keys"
info "✓ Public key uploaded"

# Verify uploaded key
KEY_FINGERPRINT=$(ssh-keygen -lf "$PUBKEY_FILE" 2>/dev/null | awk '{print $2}')
info "✓ Key fingerprint: $KEY_FINGERPRINT"

# Create init.sh script
info "Creating boot initialization script..."
ssh_exec 'cat > /data/init.sh << '\''EOF'\''
#!/bin/sh
# /data/init.sh - Boot initialization script for OH2P device
# This script runs automatically on boot (supported by patched firmware)

# Setup SSH public key authentication
setup_ssh_keys() {
    if [ -f /data/.ssh/authorized_keys ]; then
        echo "[init.sh] Setting up SSH key authentication..."
        
        # Create root SSH directory if it does not exist
        mkdir -p /root/.ssh 2>/dev/null
        
        # Copy authorized_keys from persistent storage to root
        # We use cp instead of symlink because Dropbear may not follow symlinks
        cp /data/.ssh/authorized_keys /root/.ssh/authorized_keys 2>/dev/null
        
        # Set correct permissions (critical for SSH security)
        chmod 700 /root/.ssh 2>/dev/null
        chmod 600 /root/.ssh/authorized_keys 2>/dev/null
        
        echo "[init.sh] SSH key authentication configured"
    fi
}

# Setup PATH for utilities in /data/bin
setup_path() {
    if [ -d /data/bin ]; then
        export PATH="/data/bin:$PATH"
        echo "[init.sh] Added /data/bin to PATH"
    fi
}

# Main initialization
echo "[init.sh] Starting device initialization..."

setup_ssh_keys
setup_path

echo "[init.sh] Initialization complete"

# User customizations can be added below this line
# Example: /usr/sbin/tts_play.sh '\''初始化成功'\''
EOF
'
ssh_exec "chmod +x /data/init.sh"
info "✓ Created /data/init.sh"

# Apply configuration immediately (without reboot)
info "Applying SSH key configuration..."
ssh_exec "sh /data/init.sh"
info "✓ Configuration applied"

# Verify setup
info "Verifying SSH key setup..."
ssh_exec "ls -la /root/.ssh/authorized_keys /data/.ssh/authorized_keys" >/dev/null 2>&1
if [ $? -eq 0 ]; then
    info "✓ SSH key files verified"
else
    error "SSH key setup verification failed"
fi

# Display final status
echo ""
info "=========================================="
info "SSH key deployment completed successfully!"
info "=========================================="
echo ""
info "Next steps:"
echo "  1. Test key authentication:"
echo "     ssh -i ${PUBKEY_FILE%.pub} $SSH_OPTS $SSH_USER@$DEVICE_IP"
echo ""
echo "  2. If successful, add to ~/.ssh/config:"
echo "     Host xiaoai"
echo "       HostName $DEVICE_IP"
echo "       User $SSH_USER"
echo "       IdentityFile ${PUBKEY_FILE%.pub}"
echo "       HostKeyAlgorithms +ssh-rsa"
echo ""
echo "  3. Then connect with: ssh xiaoai"
echo ""
warn "Note: Password authentication is still enabled (recommended for recovery)"
warn "The configuration persists across reboots via /data/init.sh"
