#!/bin/sh
# /data/init.sh - Boot initialization script for OH2P device
# This script runs automatically on boot (supported by patched firmware)

# Setup SSH public key authentication
setup_ssh_keys() {
    if [ -f /data/.ssh/authorized_keys ]; then
        echo "[init.sh] Setting up SSH key authentication..."
        
        # CRITICAL: /root is on read-only SquashFS, so we need a bind mount
        # Dropbear looks for authorized_keys in /root/.ssh/ (from /etc/passwd)
        # not in $HOME/.ssh/ (even though Dropbear's HOME is set to /tmp)
        
        # Create temporary overlay directory for /root
        mkdir -p /tmp/root_overlay/.ssh
        
        # Copy authorized_keys from persistent storage
        cp /data/.ssh/authorized_keys /tmp/root_overlay/.ssh/authorized_keys
        
        # Set correct permissions (critical for SSH security)
        chmod 700 /tmp/root_overlay/.ssh
        chmod 600 /tmp/root_overlay/.ssh/authorized_keys
        
        # Bind mount the overlay to /root so Dropbear can find it
        mount --bind /tmp/root_overlay /root
        
        echo "[init.sh] SSH key authentication configured (bind mounted to /root)"
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
