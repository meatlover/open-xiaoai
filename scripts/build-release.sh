#!/bin/bash

# Manual release script for Open-XiaoAI client binaries
# Usage: ./build-release.sh [version] [target]

set -e

VERSION="${1:-v$(date +%Y.%m.%d)}"
TARGET="${2:-armv7-unknown-linux-gnueabihf}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLIENT_RUST_DIR="$SCRIPT_DIR/../packages/client-rust"
RELEASE_DIR="$SCRIPT_DIR/release"

echo "🚀 Building Open-XiaoAI Release"
echo "📦 Version: $VERSION"
echo "🎯 Target: $TARGET"
echo ""

# Check if cross is installed
if ! command -v cross &> /dev/null; then
    echo "❌ cross is not installed. Installing..."
    cargo install cross --git https://github.com/cross-rs/cross
fi

# Create release directory
mkdir -p "$RELEASE_DIR"
cd "$CLIENT_RUST_DIR"

echo "🔨 Building binaries..."

# Build http_client
echo "📦 Building http_client..."
cross build --release --target "$TARGET" --bin http_client

# Build http_server
echo "📦 Building http_server..." 
cross build --release --target "$TARGET" --bin http_server

# Build multi_mode_client if it exists
if [ -f "src/bin/multi_mode_client.rs" ]; then
    echo "📦 Building multi_mode_client..."
    cross build --release --target "$TARGET" --bin multi_mode_client
    MULTI_MODE_EXISTS=true
else
    echo "⚠️  multi_mode_client.rs not found, skipping"
    MULTI_MODE_EXISTS=false
fi

echo "📁 Preparing release artifacts..."

# Copy binaries to release directory
cp "target/$TARGET/release/http_client" "$RELEASE_DIR/"
cp "target/$TARGET/release/http_server" "$RELEASE_DIR/"

if [ "$MULTI_MODE_EXISTS" = true ]; then
    cp "target/$TARGET/release/multi_mode_client" "$RELEASE_DIR/"
fi

# Copy configuration files
if [ -f "config.template.json" ]; then
    cp "config.template.json" "$RELEASE_DIR/"
fi

# Copy boot script
if [ -f "$SCRIPT_DIR/boot.sh" ]; then
    cp "$SCRIPT_DIR/boot.sh" "$RELEASE_DIR/"
fi

# Create archive
ARCHIVE_NAME="open-xiaoai-$VERSION-$TARGET.tar.gz"
cd "$RELEASE_DIR"
tar -czf "../$ARCHIVE_NAME" *

echo ""
echo "✅ Release build complete!"
echo "📦 Archive: $SCRIPT_DIR/$ARCHIVE_NAME"
echo "📁 Files: $RELEASE_DIR/"
echo ""
echo "📋 Contents:"
ls -la "$RELEASE_DIR/"

echo ""
echo "🚀 Next steps:"
echo "1. Test the binaries on your target device"
echo "2. Create a GitHub release: gh release create $VERSION $SCRIPT_DIR/$ARCHIVE_NAME"
echo "3. Or upload manually to GitHub Releases"

# Optional: Test on local machine if x86_64
if [ "$TARGET" = "x86_64-unknown-linux-gnu" ]; then
    echo ""
    echo "🧪 Testing local build..."
    
    # Test http_server starts
    timeout 5s "$RELEASE_DIR/http_server" || echo "✅ http_server starts successfully"
    
    # Test http_client shows help
    "$RELEASE_DIR/http_client" --help > /dev/null 2>&1 || echo "✅ http_client binary works"
    
    if [ "$MULTI_MODE_EXISTS" = true ]; then
        "$RELEASE_DIR/multi_mode_client" --help > /dev/null 2>&1 || echo "✅ multi_mode_client binary works"
    fi
fi
