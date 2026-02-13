#!/bin/bash
# Install essential Linux utilities to OH2P device
# Usage: ./install-utilities.sh <device-ip>

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SSH_OPTS="-o HostKeyAlgorithms=+ssh-rsa -o StrictHostKeyChecking=no -o ConnectTimeout=10"
SSH_USER="root"
TEMP_DIR="/tmp/oh2p-utils-$$"

# Binary URLs (ARM v7 static binaries)
BUSYBOX_URL="https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox_ARMV7L"
CURL_URL="https://github.com/moparisthebest/static-curl/releases/download/v8.4.0/curl-amd64"
WGET_URL="https://github.com/ernw/static-toolbox/releases/download/1.0.1/wget-armv7"

# Helper functions
error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    cleanup
    exit 1
}

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

cleanup() {
    if [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
}

usage() {
    cat << EOF
Install essential Linux utilities to OH2P device

Usage: $0 <device-ip> [options]

Arguments:
  device-ip       IP address of the OH2P device (e.g., 192.168.31.140)

Options:
  --busybox-only  Install only BusyBox (saves space)
  --skip-busybox  Skip BusyBox installation
  --curl-only     Install only curl
  --wget-only     Install only wget

Example:
  $0 192.168.31.140
  $0 192.168.31.140 --busybox-only

Notes:
  - Requires ~2-5MB free space in /data
  - Device must have SSH access configured
  - Downloads ARM v7 static binaries
EOF
    exit 1
}

# Parse arguments
INSTALL_BUSYBOX=true
INSTALL_CURL=true
INSTALL_WGET=true

DEVICE_IP="$1"
shift || usage

while [ $# -gt 0 ]; do
    case "$1" in
        --busybox-only)
            INSTALL_CURL=false
            INSTALL_WGET=false
            ;;
        --skip-busybox)
            INSTALL_BUSYBOX=false
            ;;
        --curl-only)
            INSTALL_BUSYBOX=false
            INSTALL_WGET=false
            ;;
        --wget-only)
            INSTALL_BUSYBOX=false
            INSTALL_CURL=false
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
    shift
done

[ -z "$DEVICE_IP" ] && usage

# Validate tools
command -v ssh >/dev/null 2>&1 || error "ssh command not found. Please install OpenSSH client."
command -v scp >/dev/null 2>&1 || error "scp command not found. Please install OpenSSH client."
command -v curl >/dev/null 2>&1 || error "curl command not found. Please install curl."

# Setup
trap cleanup EXIT
mkdir -p "$TEMP_DIR"
cd "$TEMP_DIR"

info "Installing utilities to device: $DEVICE_IP"

# Function to execute SSH command
ssh_exec() {
    ssh $SSH_OPTS "$SSH_USER@$DEVICE_IP" "$@"
}

# Test SSH connectivity
step "Testing SSH connection..."
if ! ssh_exec "echo 'Connected'" >/dev/null 2>&1; then
    error "Cannot connect to device at $DEVICE_IP"
fi
info "✓ SSH connection successful"

# Check architecture
step "Verifying device architecture..."
ARCH=$(ssh_exec "uname -m")
info "Device architecture: $ARCH"
if [[ "$ARCH" != "armv7l" ]]; then
    warn "Expected armv7l, got $ARCH. Binaries may not work!"
fi

# Check disk space
step "Checking available disk space..."
DISK_INFO=$(ssh_exec "df -h /data | tail -n1")
DISK_AVAILABLE=$(echo "$DISK_INFO" | awk '{print $4}')
DISK_USED=$(echo "$DISK_INFO" | awk '{print $3}')
info "Disk space in /data: $DISK_USED used, $DISK_AVAILABLE available"

# Create /data/bin directory
step "Creating /data/bin directory..."
ssh_exec "mkdir -p /data/bin"
info "✓ Created /data/bin"

# Download and install BusyBox
if [ "$INSTALL_BUSYBOX" = true ]; then
    step "Downloading BusyBox..."
    info "URL: $BUSYBOX_URL"
    curl -fsSL -o busybox "$BUSYBOX_URL" || error "Failed to download BusyBox"
    chmod +x busybox
    SIZE=$(du -h busybox | cut -f1)
    info "✓ Downloaded BusyBox ($SIZE)"
    
    step "Uploading BusyBox to device..."
    scp $SSH_OPTS busybox "$SSH_USER@$DEVICE_IP:/data/bin/busybox" || error "Failed to upload BusyBox"
    ssh_exec "chmod +x /data/bin/busybox"
    info "✓ Installed BusyBox"
    
    # Test BusyBox
    BUSYBOX_VERSION=$(ssh_exec "/data/bin/busybox --help 2>&1 | head -n1")
    info "✓ BusyBox version: $BUSYBOX_VERSION"
fi

# Download and install curl
if [ "$INSTALL_CURL" = true ]; then
    step "Downloading curl..."
    warn "Note: Using amd64 curl as ARM static build may not be available"
    warn "If curl doesn't work on device, use wget or busybox wget instead"
    info "URL: $CURL_URL"
    # Skip curl for now as ARM builds are tricky
    warn "Skipping curl - use busybox wget or native wget instead"
fi

# Download and install wget
if [ "$INSTALL_WGET" = true ]; then
    step "Downloading wget..."
    info "URL: $WGET_URL"
    curl -fsSL -o wget "$WGET_URL" || warn "Failed to download wget, continuing..."
    
    if [ -f wget ]; then
        chmod +x wget
        SIZE=$(du -h wget | cut -f1)
        info "✓ Downloaded wget ($SIZE)"
        
        step "Uploading wget to device..."
        scp $SSH_OPTS wget "$SSH_USER@$DEVICE_IP:/data/bin/wget" || warn "Failed to upload wget"
        ssh_exec "chmod +x /data/bin/wget"
        info "✓ Installed wget"
        
        # Test wget
        WGET_VERSION=$(ssh_exec "/data/bin/wget --version 2>&1 | head -n1" || echo "unknown")
        info "✓ wget version: $WGET_VERSION"
    fi
fi

# Create symlinks for common BusyBox utilities
if [ "$INSTALL_BUSYBOX" = true ]; then
    step "Creating symlinks for common utilities..."
    ssh_exec "cd /data/bin && for util in tar gzip gunzip bzip2 unzip vi less grep sed awk head tail sort uniq wc find; do ln -sf busybox \$util 2>/dev/null; done"
    info "✓ Created symlinks"
fi

# Verify PATH setup in init.sh
step "Verifying PATH configuration..."
if ssh_exec "grep -q '/data/bin' /data/init.sh" 2>/dev/null; then
    info "✓ /data/bin is in PATH (via /data/init.sh)"
else
    warn "/data/init.sh doesn't setup PATH for /data/bin"
    warn "Run deploy-ssh-key.sh first or manually add to /data/init.sh:"
    warn "  export PATH=\"/data/bin:\$PATH\""
fi

# Display final status
DISK_INFO_AFTER=$(ssh_exec "df -h /data | tail -n1")
DISK_AVAILABLE_AFTER=$(echo "$DISK_INFO_AFTER" | awk '{print $4}')
DISK_USED_AFTER=$(echo "$DISK_INFO_AFTER" | awk '{print $3}')

echo ""
info "=========================================="
info "Utility installation completed!"
info "=========================================="
echo ""
info "Disk space in /data:"
info "  Before: $DISK_USED used, $DISK_AVAILABLE available"
info "  After:  $DISK_USED_AFTER used, $DISK_AVAILABLE_AFTER available"
echo ""
info "Installed utilities:"
ssh_exec "ls -lh /data/bin/"
echo ""
info "Test utilities:"
echo "  ssh $SSH_USER@$DEVICE_IP"
echo "  busybox --help"
echo "  busybox --list    # List all available commands"
echo "  wget --version"
echo ""
info "Available via symlinks:"
echo "  tar, gzip, gunzip, bzip2, unzip, vi, less, grep, sed, awk, head, tail, sort, uniq, wc, find"
echo ""
warn "Note: Utilities persist across reboots (stored in /data)"
warn "Make sure /data/bin is in PATH (via /data/init.sh)"
