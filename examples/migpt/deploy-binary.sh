#!/bin/bash

# HTTP Server Deployment Script
# Usage: ./deploy-http-server.sh <remote-server-ip> [remote-user]

set -e

REMOTE_SERVER="$1"
REMOTE_USER="${2:-root}"
BINARY_PATH="./target/release/http_server"
REMOTE_PATH="/opt/xiaoai"
SERVICE_NAME="xiaoai-http-server"

if [ -z "$REMOTE_SERVER" ]; then
    echo "Usage: $0 <remote-server-ip> [remote-user]"
    echo "Example: $0 192.168.1.100 root"
    exit 1
fi

echo "🚀 Deploying HTTP server to $REMOTE_USER@$REMOTE_SERVER"

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at $BINARY_PATH"
    echo "Run: cargo build --release --bin http_server"
    exit 1
fi

echo "📦 Binary size: $(du -h $BINARY_PATH | cut -f1)"

# Create remote directory
echo "📁 Creating remote directory..."
ssh "$REMOTE_USER@$REMOTE_SERVER" "mkdir -p $REMOTE_PATH"

# Copy binary
echo "📤 Copying binary..."
scp "$BINARY_PATH" "$REMOTE_USER@$REMOTE_SERVER:$REMOTE_PATH/"

# Make executable
ssh "$REMOTE_USER@$REMOTE_SERVER" "chmod +x $REMOTE_PATH/http_server"

# Create systemd service
echo "⚙️  Creating systemd service..."
ssh "$REMOTE_USER@$REMOTE_SERVER" "cat > /etc/systemd/system/$SERVICE_NAME.service << 'EOF'
[Unit]
Description=XiaoAI HTTP Server
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$REMOTE_PATH
ExecStart=$REMOTE_PATH/http_server
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF"

# Enable and start service
echo "🔧 Enabling and starting service..."
ssh "$REMOTE_USER@$REMOTE_SERVER" "
    systemctl daemon-reload
    systemctl enable $SERVICE_NAME
    systemctl start $SERVICE_NAME
    systemctl status $SERVICE_NAME --no-pager
"

echo "✅ Deployment complete!"
echo "🌐 Server should be running on http://$REMOTE_SERVER:4399"
echo ""
echo "📋 Management commands:"
echo "  Check status: ssh $REMOTE_USER@$REMOTE_SERVER 'systemctl status $SERVICE_NAME'"
echo "  View logs:    ssh $REMOTE_USER@$REMOTE_SERVER 'journalctl -u $SERVICE_NAME -f'"
echo "  Restart:      ssh $REMOTE_USER@$REMOTE_SERVER 'systemctl restart $SERVICE_NAME'"
echo "  Stop:         ssh $REMOTE_USER@$REMOTE_SERVER 'systemctl stop $SERVICE_NAME'"
