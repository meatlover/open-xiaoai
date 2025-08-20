#!/bin/bash

# Simple file transfer script for devices without scp
# Uses ssh and base64 encoding for reliable binary transfers

DEVICE_IP="192.168.143.211"
DEVICE_USER="root"
DEVICE_PASS="open-xiaoai"

# Function to copy file to device
copy_to_device() {
    local local_file="$1"
    local remote_path="$2"
    
    if [ ! -f "$local_file" ]; then
        echo "❌ Local file not found: $local_file"
        return 1
    fi
    
    echo "📤 Copying $local_file to $DEVICE_IP:$remote_path"
    
    # Create remote directory if needed
    remote_dir=$(dirname "$remote_path")
    sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP" "mkdir -p '$remote_dir'"
    
    # Transfer file using cat (simple and reliable)
    echo "🔄 Transferring file..."
    cat "$local_file" | sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP" "cat > '$remote_path' && chmod +x '$remote_path'"
    
    # Verify transfer with MD5 hash
    echo "🔍 Verifying transfer integrity..."
    local_md5=$(openssl md5 "$local_file" | cut -d'=' -f2 | tr -d ' ')
    remote_md5=$(sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP" "md5sum '$remote_path' 2>/dev/null | cut -d' ' -f1" || echo "")
    
    if [ "$local_md5" = "$remote_md5" ] && [ -n "$local_md5" ]; then
        local_size=$(wc -c < "$local_file" | tr -d ' ')
        echo "✅ Successfully copied $local_file ($local_size bytes, MD5: $local_md5)"
        return 0
    else
        echo "❌ Transfer verification failed"
        echo "   Local MD5:  $local_md5"
        echo "   Remote MD5: $remote_md5"
        return 1
    fi
}

# Function to copy file from device
copy_from_device() {
    local remote_path="$1"
    local local_file="$2"
    
    echo "📥 Copying $DEVICE_IP:$remote_path to $local_file"
    
    # Create local directory if needed
    local_dir=$(dirname "$local_file")
    mkdir -p "$local_dir"
    
    # Transfer file using cat
    echo "🔄 Transferring file..."
    sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP" "cat '$remote_path'" > "$local_file"
    
    # Verify transfer with MD5 hash
    echo "🔍 Verifying transfer integrity..."
    local_md5=$(openssl md5 "$local_file" | cut -d'=' -f2 | tr -d ' ')
    remote_md5=$(sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP" "md5sum '$remote_path' 2>/dev/null | cut -d' ' -f1" || echo "")
    
    if [ "$local_md5" = "$remote_md5" ] && [ -n "$local_md5" ]; then
        local_size=$(wc -c < "$local_file" | tr -d ' ')
        echo "✅ Successfully copied $remote_path ($local_size bytes, MD5: $local_md5)"
        return 0
    else
        echo "❌ Transfer verification failed"
        echo "   Local MD5:  $local_md5"
        echo "   Remote MD5: $remote_md5"
        return 1
    fi
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [to|from] <source> <destination>"
    echo ""
    echo "Examples:"
    echo "  $0 to /tmp/client /data/open-xiaoai/client"
    echo "  $0 from /data/logs/app.log ./logs/app.log"
    echo ""
    echo "Device: $DEVICE_USER@$DEVICE_IP"
}

# Main script
if [ $# -lt 3 ]; then
    show_usage
    exit 1
fi

case "$1" in
    "to")
        copy_to_device "$2" "$3"
        ;;
    "from")
        copy_from_device "$2" "$3"
        ;;
    *)
        echo "❌ Invalid operation: $1"
        show_usage
        exit 1
        ;;
esac
