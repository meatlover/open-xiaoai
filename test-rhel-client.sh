#!/bin/bash
# Test RHEL x86_64 client with Service Token authentication

echo "=== Testing RHEL x86_64 client with Service Token ==="
echo "Usage: $0 <client_id> <client_secret>"
echo ""

if [ $# -ne 2 ]; then
    echo "Please provide CLIENT_ID and CLIENT_SECRET as arguments"
    echo "Example: $0 'your-client-id' 'your-client-secret'"
    exit 1
fi

CLIENT_ID="$1"
CLIENT_SECRET="$2"

echo "🔍 Testing Service Token authentication:"
echo "Client ID: ${CLIENT_ID:0:10}..."
echo "Client Secret: ${CLIENT_SECRET:0:10}..."
echo ""

echo "🚀 Starting RHEL x86_64 client..."
CLIENT_TLS_ENABLED=true \
CF_ACCESS_CLIENT_ID="$CLIENT_ID" \
CF_ACCESS_CLIENT_SECRET="$CLIENT_SECRET" \
timeout 30s /tmp/client-rhel-x86_64 wss://awsjp-dev1.harmanota.com.cn

echo ""
echo "Test completed."
