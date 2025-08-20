#!/bin/bash

# Setup script for Original Sherpa-ONNX KWS with custom wake word "土豆土豆"
MI_DEVICE_IP=${1:-"192.168.143.211"}  # Default IP from your setup
MI_USER="root"
MI_PASSWORD="open-xiaoai"

# SSH command with your specific method
SSH_CMD="sshpass -p '$MI_PASSWORD' ssh -o HostKeyAlgorithms=+ssh-rsa $MI_USER@$MI_DEVICE_IP"
SCP_CMD="sshpass -p '$MI_PASSWORD' scp -o HostKeyAlgorithms=+ssh-rsa"

echo "🚀 Setting up Sherpa-ONNX KWS on Mi device at $MI_DEVICE_IP"
echo "📋 Wake word: 土豆土豆"
echo "💬 Reply: 请说"

# Test connection first
echo ""
echo "🔌 Testing connection..."
if ! $SSH_CMD "echo 'Connection successful'"; then
    echo "❌ Failed to connect to Mi device. Please check:"
    echo "   - IP address: $MI_DEVICE_IP"
    echo "   - Password: $MI_PASSWORD"
    echo "   - Device is accessible"
    exit 1
fi

# Step 1: Install the original KWS system
echo ""
echo "📦 Step 1: Installing KWS system on device..."
$SSH_CMD "curl -L -o /data/init.sh https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-kws/init.sh && chmod +x /data/init.sh"

# Step 2: Copy our custom keyword configuration
echo ""
echo "📝 Step 2: Setting up custom wake word configuration..."
$SCP_CMD tudou-keywords.txt $MI_USER@$MI_DEVICE_IP:/tmp/custom-keywords.txt
$SCP_CMD custom-reply.txt $MI_USER@$MI_DEVICE_IP:/tmp/custom-reply.txt
$SCP_CMD validate-tokens.sh $MI_USER@$MI_DEVICE_IP:/tmp/validate-tokens.sh

# Step 3: Install and configure
echo ""
echo "🔧 Step 3: Installing KWS models and configuring custom wake word..."
$SSH_CMD << 'EOF'
# Run the init script to download models and setup base system
echo "📥 Downloading KWS models (this may take a few minutes)..."
/data/init.sh --no-monitor

# Create config directory if it doesn't exist
mkdir -p /data/open-xiaoai/kws

# First, validate available tokens
chmod +x /tmp/validate-tokens.sh
echo "🔍 Validating tokens for custom wake word..."
/tmp/validate-tokens.sh

# Try our custom keyword first
echo "📝 Attempting to set up custom wake word: 土豆土豆"
cp /tmp/custom-keywords.txt /data/open-xiaoai/kws/keywords.txt

# Test if the keywords work by trying to start KWS briefly
echo "🧪 Testing keyword configuration..."
if timeout 5s /data/open-xiaoai/kws/kws --model-type=zipformer2 --tokens="/data/open-xiaoai/kws/models/tokens.txt" --encoder="/data/open-xiaoai/kws/models/encoder.onnx" --decoder="/data/open-xiaoai/kws/models/decoder.onnx" --joiner="/data/open-xiaoai/kws/models/joiner.onnx" --keywords-file="/data/open-xiaoai/kws/keywords.txt" --provider=cpu --num-threads=1 --chunk-size=1024 noop 2>/tmp/kws-test.log; then
    echo "✅ Custom keyword configuration successful!"
else
    echo "❌ Custom keyword failed, falling back to working example..."
    echo "d òu b āo d òu b āo @豆包豆包" > /data/open-xiaoai/kws/keywords.txt
    echo "⚠️  Using '豆包豆包' as wake word instead"
    echo "💡 Run /tmp/validate-tokens.sh to debug token issues"
fi

# Setup custom reply
echo "💬 Setting up custom reply: 请说"
cp /tmp/custom-reply.txt /data/open-xiaoai/kws/reply.txt

echo "✅ Configuration complete!"
echo ""
echo "📋 Current configuration:"
echo "Wake word file:"
cat /data/open-xiaoai/kws/keywords.txt
echo ""
echo "Reply file:"
cat /data/open-xiaoai/kws/reply.txt
EOF

echo ""
echo "🔥 Step 4: Starting KWS service..."
$SSH_CMD << 'EOF'
# Start the KWS service
echo "🎤 Starting Sherpa-ONNX KWS service with custom wake word..."
/data/init.sh > /tmp/kws.log 2>&1 &

echo "⏱️  Waiting for KWS service to initialize..."
sleep 10

echo "📊 KWS service status:"
ps | grep kws | grep -v grep || echo "❌ KWS service not running"

echo ""
echo "📄 Recent KWS logs:"
tail -20 /tmp/kws.log 2>/dev/null || echo "No logs yet"
EOF

echo ""
echo "✅ Setup complete!"
echo ""
echo "🎯 What was configured:"
echo "   📍 KWS models installed in /data/open-xiaoai/kws/"
echo "   🎤 Wake word: 土豆土豆 (t ǔ d òu t ǔ d òu)"
echo "   💬 Reply: 请说"
echo "   🔄 KWS service started in background"
echo ""
echo "🧪 Testing:"
echo "   1. Say '土豆土豆' near the device"
echo "   2. Device should respond with '请说'"
echo "   3. Check logs: $SSH_CMD 'tail -f /tmp/kws.log'"
echo ""
echo "� Next: Start your client to monitor KWS events:"
echo "   $SSH_CMD"
echo "   cd /tmp"
echo "   ./client config-kws.json"
echo ""
echo "�🔧 Troubleshooting:"
echo "   • If not working, reboot the device: $SSH_CMD 'reboot'"
echo "   • Check process: $SSH_CMD 'ps | grep kws'"
echo "   • View config: $SSH_CMD 'cat /data/open-xiaoai/kws/keywords.txt'"
