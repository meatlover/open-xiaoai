#!/bin/bash

# Remote build script for open-xiaoai client
# Builds on awsjp-dev1 to bypass China firewall limitations

set -e

REMOTE_HOST="awsjp-dev1"
REMOTE_PATH="/root/open-xiaoai"
TARGET="armv7-unknown-linux-musleabihf"
BUILD_TYPE="${1:-release}"

echo "🌐 Starting remote build process..."
echo "📋 Remote host: $REMOTE_HOST"
echo "📋 Remote path: $REMOTE_PATH"
echo "📋 Target: $TARGET"
echo "📋 Build type: $BUILD_TYPE"

# Step 1: SSH to remote and update code
echo ""
echo "🔄 Step 1: Connecting to remote and updating code..."
ssh $REMOTE_HOST "
    set -e
    echo '📁 Navigating to project directory...'
    cd $REMOTE_PATH
    
    echo '📥 Pulling latest changes from dev branch...'
    git fetch origin
    git checkout dev
    git pull origin dev
    
    echo '✅ Code updated successfully'
"

# Step 2: Build on remote
echo ""
echo "🔨 Step 2: Building on remote machine..."
ssh $REMOTE_HOST "
    set -e
    echo '🚀 Starting cross build...'
    cd $REMOTE_PATH/packages/client-rust
    
    # Check if cross is installed
    if ! command -v cross &> /dev/null; then
        echo '⚙️  Installing cross...'
        cargo install cross
    fi
    
    echo '🔧 Building for $TARGET...'
    cross build --$BUILD_TYPE --target $TARGET
    
    echo '📊 Build results:'
    ls -lh target/$TARGET/$BUILD_TYPE/client
    file target/$TARGET/$BUILD_TYPE/client
    
    echo '✅ Remote build completed successfully!'
"

# Step 3: Copy binary back to local
echo ""
echo "📥 Step 3: Copying binary to local machine..."

# Ensure local target directory exists
LOCAL_TARGET_DIR="target/$TARGET/$BUILD_TYPE"
mkdir -p "$LOCAL_TARGET_DIR"

# Copy the built binary
echo "🔽 Downloading binary from remote..."
scp "$REMOTE_HOST:$REMOTE_PATH/packages/client-rust/target/$TARGET/$BUILD_TYPE/client" "$LOCAL_TARGET_DIR/client"

# Verify local binary
if [ -f "$LOCAL_TARGET_DIR/client" ]; then
    echo ""
    echo "✅ Build process completed successfully!"
    echo "📁 Local binary location: $LOCAL_TARGET_DIR/client"
    echo "📊 Local binary info:"
    ls -lh "$LOCAL_TARGET_DIR/client"
    file "$LOCAL_TARGET_DIR/client"
    
    # Copy to /tmp for easy access
    cp "$LOCAL_TARGET_DIR/client" /tmp/client
    echo "📋 Binary copied to /tmp/client for easy deployment"
else
    echo "❌ Failed to copy binary to local machine"
    exit 1
fi

echo ""
echo "🎯 Next steps:"
echo "  • Deploy to device: scp /tmp/client user@device:/data/open-xiaoai/client"
echo "  • Or use existing deploy script: ./deploy.sh"
echo ""
echo "🚀 Remote build process completed!"
