#!/bin/bash
set -e

# Build the Docker image with Rust installed inside
echo "Building Ubuntu 20.04 build environment..."
docker build -f Dockerfile.build -t open-xiaoai-builder:ubuntu20.04 .

# Run the build
echo "Compiling for aarch64..."
docker run --rm \
    -v "$(pwd):/build" \
    -w /build \
    open-xiaoai-builder:ubuntu20.04 \
    cargo build --release --target aarch64-unknown-linux-gnu

echo "✅ Build complete!"
echo "Binary location: target/aarch64-unknown-linux-gnu/release/client"
echo ""
echo "To deploy to Xiaomi device, run:"
echo "  /root/bin/scp-to-xiaoai target/aarch64-unknown-linux-gnu/release/client /data/open-xiaoai/client"
