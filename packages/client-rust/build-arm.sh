#!/bin/bash

# ARM build script for open-xiaoai client
# Builds for armv7-unknown-linux-musleabihf by default

set -e

echo "🔧 Building open-xiaoai client for ARM (musl)..."

TARGET="armv7-unknown-linux-musleabihf"
BUILD_TYPE="${1:-release}"

echo "📋 Target: $TARGET"
echo "📋 Build type: $BUILD_TYPE"

# Check if cross is installed
if ! command -v cross &> /dev/null; then
    echo "❌ 'cross' tool not found. Installing..."
    cargo install cross
fi

# Build for ARM
echo "🚀 Building for $TARGET..."
cross build --$BUILD_TYPE --target $TARGET

# Show build results
echo "✅ Build completed!"
echo "📁 Binary location: target/$TARGET/$BUILD_TYPE/client"

# Show binary info
if [ -f "target/$TARGET/$BUILD_TYPE/client" ]; then
    echo "📊 Binary info:"
    ls -lh "target/$TARGET/$BUILD_TYPE/client"
    file "target/$TARGET/$BUILD_TYPE/client"
    
    # Copy to /tmp for easy access
    cp "target/$TARGET/$BUILD_TYPE/client" /tmp/client
    echo "📋 Binary copied to /tmp/client for easy deployment"
else
    echo "❌ Binary not found at expected location"
    exit 1
fi

echo "🎯 To deploy to device:"
echo "  scp /tmp/client user@device:/data/open-xiaoai/client"
echo "  scp config.json user@device:/data/open-xiaoai/config.json"
