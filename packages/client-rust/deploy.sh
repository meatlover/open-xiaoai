#!/bin/bash

# Deployment script for Mi device
MI_DEVICE_IP=${1:-"192.168.1.100"}  # Default IP, change as needed
MI_USER="root"

echo "🚀 Deploying to Mi device at $MI_DEVICE_IP"

# Copy binary to device
echo "📦 Copying client binary..."
scp /tmp/client $MI_USER@$MI_DEVICE_IP:/tmp/open-xiaoai-client

# Copy config file
echo "📋 Copying config file..."
scp config.json $MI_USER@$MI_DEVICE_IP:/tmp/config.json

# Make binary executable
echo "🔧 Making binary executable..."
ssh $MI_USER@$MI_DEVICE_IP "chmod +x /tmp/open-xiaoai-client"

echo "✅ Deployment complete!"
echo ""
echo "🎯 To run on the device:"
echo "   ssh $MI_USER@$MI_DEVICE_IP"
echo "   cd /tmp"
echo "   ./open-xiaoai-client config.json"
echo ""
echo "🧪 To run in test mode:"
echo "   ./open-xiaoai-client config.json --test"
echo ""
echo "🐛 To run with voice features and debug:"
echo "   ./open-xiaoai-client config.json --voice --debug"
