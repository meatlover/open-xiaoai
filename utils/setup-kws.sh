#!/bin/bash

# KWS Setup Utility for Open-XiaoAI
# Sets up custom wake words on Mi speakers

set -e

DEVICE_IP="${1:-192.168.143.211}"
DEVICE_USER="${2:-root}"
DEVICE_PASS="${3:-open-xiaoai}"

echo "🎯 Open-XiaoAI KWS Setup Utility"
echo "================================"
echo "📱 Device: $DEVICE_USER@$DEVICE_IP"
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
    sshpass -p "$DEVICE_PASS" scp -o HostKeyAlgorithms=+ssh-rsa "$local_file" "$DEVICE_USER@$DEVICE_IP:$remote_path"
}

# Function to setup basic KWS structure
setup_kws_structure() {
    echo "📁 Setting up KWS directory structure..."
    run_on_device "mkdir -p /data/open-xiaoai/kws"
    
    echo "✅ KWS directory created"
}

# Function to setup default wake words
setup_default_keywords() {
    echo "🗣️  Setting up default wake words..."
    
    run_on_device "cat <<EOF > /data/open-xiaoai/kws/keywords.txt
t iān m āo j īng l íng @天猫精灵
x iǎo d ù x iǎo d ù @小度小度
d òu b āo d òu b āo @豆包豆包
n ǐ h ǎo x iǎo zh ì @你好小智
EOF"
    
    echo "✅ Default wake words configured"
}

# Function to setup default replies
setup_default_replies() {
    echo "💬 Setting up default replies..."
    
    run_on_device "cat <<EOF > /data/open-xiaoai/kws/reply.txt
主人你好，请问有什么吩咐？
EOF"
    
    echo "✅ Default replies configured"
}

# Function to install KWS service
install_kws_service() {
    echo "⚙️  Installing KWS service..."
    
    run_on_device "curl -sSfL https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-kws/init.sh | sh"
    
    echo "✅ KWS service installed"
}

# Function to enable auto-start
enable_autostart() {
    echo "🚀 Enabling auto-start..."
    
    run_on_device "curl -L -o /data/init.sh https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-kws/init.sh"
    
    echo "✅ Auto-start enabled"
}

# Function to show status
show_status() {
    echo ""
    echo "📊 Current KWS Status:"
    echo "===================="
    
    echo "📁 Directory structure:"
    run_on_device "ls -la /data/open-xiaoai/kws/ 2>/dev/null || echo 'KWS directory not found'"
    
    echo ""
    echo "🗣️  Current wake words:"
    run_on_device "cat /data/open-xiaoai/kws/keywords.txt 2>/dev/null || echo 'No keywords file found'"
    
    echo ""
    echo "💬 Current replies:"
    run_on_device "cat /data/open-xiaoai/kws/reply.txt 2>/dev/null || echo 'No replies file found'"
}

# Function to run debug mode
run_debug() {
    echo "🐛 Starting KWS debug mode..."
    echo "📝 This will show real-time voice recognition results"
    echo "🗣️  Please speak after seeing 'Started! Please speak'"
    echo ""
    
    run_on_device "curl -sSfL https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-kws/debug.sh | sh"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [DEVICE_IP] [DEVICE_USER] [DEVICE_PASS] [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  setup     - Complete KWS setup (default)"
    echo "  status    - Show current KWS status"
    echo "  debug     - Run KWS debug mode"
    echo "  keywords  - Setup default keywords only"
    echo "  replies   - Setup default replies only"
    echo "  service   - Install KWS service only"
    echo "  autostart - Enable auto-start only"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Setup on default device"
    echo "  $0 192.168.1.100                     # Setup on specific IP"
    echo "  $0 192.168.1.100 root password setup # Full setup with custom credentials"
    echo "  $0 192.168.1.100 root password debug # Run debug mode"
}

# Parse command
COMMAND="${4:-setup}"

case "$COMMAND" in
    "setup")
        echo "🎯 Starting complete KWS setup..."
        setup_kws_structure
        setup_default_keywords
        setup_default_replies
        install_kws_service
        enable_autostart
        show_status
        echo ""
        echo "🎉 KWS setup completed successfully!"
        echo "📝 You can now reboot the device or run debug mode to test"
        ;;
    "status")
        show_status
        ;;
    "debug")
        run_debug
        ;;
    "keywords")
        setup_kws_structure
        setup_default_keywords
        echo "✅ Keywords setup completed"
        ;;
    "replies")
        setup_kws_structure
        setup_default_replies
        echo "✅ Replies setup completed"
        ;;
    "service")
        install_kws_service
        echo "✅ Service installation completed"
        ;;
    "autostart")
        enable_autostart
        echo "✅ Auto-start enabled"
        ;;
    "help"|"-h"|"--help")
        show_usage
        ;;
    *)
        echo "❌ Unknown command: $COMMAND"
        show_usage
        exit 1
        ;;
esac
