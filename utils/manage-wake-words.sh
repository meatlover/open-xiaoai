#!/bin/bash

# Wake Words Management Utility
# Manages custom wake words and replies for Open-XiaoAI KWS

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKSPACE_DIR="$PROJECT_ROOT/workspace"

DEVICE_IP="${DEVICE_IP:-192.168.143.211}"
DEVICE_USER="${DEVICE_USER:-root}"
DEVICE_PASS="${DEVICE_PASS:-open-xiaoai}"

echo "🎙️  Open-XiaoAI Wake Words Manager"
echo "================================="
echo ""

# Function to run command on device
run_on_device() {
    local cmd="$1"
    sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP" "$cmd"
}

# Function to copy file to device
copy_to_device() {
    local local_file="$1"
    local remote_path="$2"
    
    if [ ! -f "$local_file" ]; then
        echo "❌ Local file not found: $local_file"
        return 1
    fi
    
    echo "📤 Copying $local_file to device:$remote_path"
    sshpass -p "$DEVICE_PASS" scp -o HostKeyAlgorithms=+ssh-rsa "$local_file" "$DEVICE_USER@$DEVICE_IP:$remote_path"
    echo "✅ File copied successfully"
}

# Function to copy file from device
copy_from_device() {
    local remote_path="$1"
    local local_file="$2"
    
    mkdir -p "$(dirname "$local_file")"
    
    echo "📥 Copying device:$remote_path to $local_file"
    sshpass -p "$DEVICE_PASS" scp -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP:$remote_path" "$local_file"
    echo "✅ File copied successfully"
}

# Function to edit keywords locally
edit_keywords() {
    local keywords_file="$WORKSPACE_DIR/keywords.txt"
    
    # Copy current keywords from device if they exist
    if run_on_device "test -f /data/open-xiaoai/kws/keywords.txt"; then
        copy_from_device "/data/open-xiaoai/kws/keywords.txt" "$keywords_file"
    else
        # Use template if no existing file
        cp "$WORKSPACE_DIR/keywords-template.txt" "$keywords_file"
    fi
    
    echo "📝 Opening keywords file for editing..."
    echo "💡 File location: $keywords_file"
    echo ""
    echo "📋 Current keywords:"
    cat "$keywords_file"
    echo ""
    echo "🔧 Edit the file and run '$0 upload-keywords' to apply changes"
}

# Function to edit replies locally
edit_replies() {
    local replies_file="$WORKSPACE_DIR/replies.txt"
    
    # Copy current replies from device if they exist
    if run_on_device "test -f /data/open-xiaoai/kws/reply.txt"; then
        copy_from_device "/data/open-xiaoai/kws/reply.txt" "$replies_file"
    else
        # Use template if no existing file
        cp "$WORKSPACE_DIR/replies-template.txt" "$replies_file"
    fi
    
    echo "📝 Opening replies file for editing..."
    echo "💡 File location: $replies_file"
    echo ""
    echo "📋 Current replies:"
    cat "$replies_file"
    echo ""
    echo "🔧 Edit the file and run '$0 upload-replies' to apply changes"
}

# Function to upload keywords
upload_keywords() {
    local keywords_file="$WORKSPACE_DIR/keywords.txt"
    
    if [ ! -f "$keywords_file" ]; then
        echo "❌ Keywords file not found: $keywords_file"
        echo "💡 Run '$0 edit-keywords' first"
        return 1
    fi
    
    # Ensure directory exists on device
    run_on_device "mkdir -p /data/open-xiaoai/kws"
    
    copy_to_device "$keywords_file" "/data/open-xiaoai/kws/keywords.txt"
    
    echo "🔄 Restarting KWS service..."
    # Try to restart if service is running
    run_on_device "pkill -f sherpa || true"
    
    echo "✅ Keywords uploaded and service restarted"
    echo "💡 Test with: $0 debug"
}

# Function to upload replies
upload_replies() {
    local replies_file="$WORKSPACE_DIR/replies.txt"
    
    if [ ! -f "$replies_file" ]; then
        echo "❌ Replies file not found: $replies_file"
        echo "💡 Run '$0 edit-replies' first"
        return 1
    fi
    
    # Ensure directory exists on device
    run_on_device "mkdir -p /data/open-xiaoai/kws"
    
    copy_to_device "$replies_file" "/data/open-xiaoai/kws/reply.txt"
    
    echo "✅ Replies uploaded successfully"
}

# Function to download current config
download_config() {
    echo "📥 Downloading current configuration from device..."
    
    mkdir -p "$WORKSPACE_DIR"
    
    if run_on_device "test -f /data/open-xiaoai/kws/keywords.txt"; then
        copy_from_device "/data/open-xiaoai/kws/keywords.txt" "$WORKSPACE_DIR/keywords.txt"
    else
        echo "⚠️  No keywords file found on device"
    fi
    
    if run_on_device "test -f /data/open-xiaoai/kws/reply.txt"; then
        copy_from_device "/data/open-xiaoai/kws/reply.txt" "$WORKSPACE_DIR/replies.txt"
    else
        echo "⚠️  No replies file found on device"
    fi
    
    echo "✅ Configuration downloaded to $WORKSPACE_DIR"
}

# Function to show current status
show_status() {
    echo "📊 Device Status:"
    echo "==============="
    
    echo "📱 Device: $DEVICE_USER@$DEVICE_IP"
    echo ""
    
    echo "📁 KWS Directory:"
    run_on_device "ls -la /data/open-xiaoai/kws/ 2>/dev/null || echo 'Directory not found'"
    echo ""
    
    echo "🗣️  Current Keywords:"
    run_on_device "cat /data/open-xiaoai/kws/keywords.txt 2>/dev/null || echo 'No keywords file'"
    echo ""
    
    echo "💬 Current Replies:"
    run_on_device "cat /data/open-xiaoai/kws/reply.txt 2>/dev/null || echo 'No replies file'"
    echo ""
    
    echo "📊 Local Workspace:"
    echo "=================="
    echo "📁 Workspace: $WORKSPACE_DIR"
    
    if [ -f "$WORKSPACE_DIR/keywords.txt" ]; then
        echo "🗣️  Local keywords file exists"
    else
        echo "🗣️  No local keywords file"
    fi
    
    if [ -f "$WORKSPACE_DIR/replies.txt" ]; then
        echo "💬 Local replies file exists"
    else
        echo "💬 No local replies file"
    fi
}

# Function to run debug mode
run_debug() {
    echo "🐛 Starting KWS debug mode on device..."
    echo "📝 This will show real-time voice recognition results"
    echo "🗣️  Please speak after seeing 'Started! Please speak'"
    echo ""
    
    run_on_device "curl -sSfL https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-kws/debug.sh | sh"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  status           - Show current status"
    echo "  download         - Download current config from device"
    echo "  edit-keywords    - Edit keywords locally"
    echo "  edit-replies     - Edit replies locally"
    echo "  upload-keywords  - Upload keywords to device"
    echo "  upload-replies   - Upload replies to device"
    echo "  debug           - Run debug mode on device"
    echo ""
    echo "Environment Variables:"
    echo "  DEVICE_IP       - Device IP address (default: 192.168.143.211)"
    echo "  DEVICE_USER     - Device username (default: root)"
    echo "  DEVICE_PASS     - Device password (default: open-xiaoai)"
    echo ""
    echo "Examples:"
    echo "  $0 status                    # Show current status"
    echo "  $0 download                  # Download current config"
    echo "  $0 edit-keywords             # Edit keywords locally"
    echo "  $0 upload-keywords           # Upload keywords to device"
    echo "  DEVICE_IP=192.168.1.100 $0 status  # Use different device IP"
}

# Parse command
COMMAND="${1:-status}"

case "$COMMAND" in
    "status")
        show_status
        ;;
    "download")
        download_config
        ;;
    "edit-keywords")
        edit_keywords
        ;;
    "edit-replies")
        edit_replies
        ;;
    "upload-keywords")
        upload_keywords
        ;;
    "upload-replies")
        upload_replies
        ;;
    "debug")
        run_debug
        ;;
    "help"|"-h"|"--help")
        show_usage
        ;;
    *)
        echo "❌ Unknown command: $COMMAND"
        echo ""
        show_usage
        exit 1
        ;;
esac
