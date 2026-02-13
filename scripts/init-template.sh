#!/bin/sh
# /data/init.sh - Boot initialization script for OH2P device
# This script runs automatically on boot (supported by patched firmware)

# Setup SSH public key authentication
setup_ssh_keys() {
    if [ -f /data/.ssh/authorized_keys ]; then
        echo "[init.sh] Setting up SSH key authentication..."
        
        # Create root SSH directory if it doesn't exist
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
# Example: /usr/sbin/tts_play.sh '初始化成功'
