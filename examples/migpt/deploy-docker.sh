#!/bin/bash

# Docker deployment script for HTTP server
# Usage: ./deploy-docker.sh <remote-server-ip> [remote-user]

set -e

REMOTE_SERVER="$1"
REMOTE_USER="${2:-root}"
IMAGE_NAME="xiaoai-http-server"
CONTAINER_NAME="xiaoai-http-server"

if [ -z "$REMOTE_SERVER" ]; then
    echo "Usage: $0 <remote-server-ip> [remote-user]"
    echo "Example: $0 192.168.1.100 root"
    exit 1
fi

echo "🐳 Building and deploying HTTP server via Docker to $REMOTE_USER@$REMOTE_SERVER"

# Build Docker image locally
echo "🔨 Building Docker image..."
docker build -t $IMAGE_NAME .

# Save image to tar file
echo "📦 Saving Docker image..."
docker save $IMAGE_NAME | gzip > /tmp/${IMAGE_NAME}.tar.gz

echo "📤 Copying Docker image to remote server..."
scp /tmp/${IMAGE_NAME}.tar.gz "$REMOTE_USER@$REMOTE_SERVER:/tmp/"

echo "🚀 Deploying on remote server..."
ssh "$REMOTE_USER@$REMOTE_SERVER" "
    # Load the image
    docker load < /tmp/${IMAGE_NAME}.tar.gz
    
    # Stop and remove existing container if it exists
    docker stop $CONTAINER_NAME 2>/dev/null || true
    docker rm $CONTAINER_NAME 2>/dev/null || true
    
    # Run the new container
    docker run -d \
        --name $CONTAINER_NAME \
        --restart unless-stopped \
        -p 4399:4399 \
        $IMAGE_NAME
    
    # Check status
    docker ps | grep $CONTAINER_NAME
    
    # Clean up
    rm /tmp/${IMAGE_NAME}.tar.gz
"

# Clean up local tar file
rm /tmp/${IMAGE_NAME}.tar.gz

echo "✅ Docker deployment complete!"
echo "🌐 Server should be running on http://$REMOTE_SERVER:4399"
echo ""
echo "📋 Management commands:"
echo "  Check status: ssh $REMOTE_USER@$REMOTE_SERVER 'docker ps | grep $CONTAINER_NAME'"
echo "  View logs:    ssh $REMOTE_USER@$REMOTE_SERVER 'docker logs -f $CONTAINER_NAME'"
echo "  Restart:      ssh $REMOTE_USER@$REMOTE_SERVER 'docker restart $CONTAINER_NAME'"
echo "  Stop:         ssh $REMOTE_USER@$REMOTE_SERVER 'docker stop $CONTAINER_NAME'"
