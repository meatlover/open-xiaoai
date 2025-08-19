#!/bin/bash

# Quick release script using GitHub CLI
# Usage: ./quick-release.sh v1.0.0 "Release description"

set -e

VERSION="$1"
DESCRIPTION="${2:-Release $VERSION}"

if [ -z "$VERSION" ]; then
    echo "❌ Usage: $0 <version> [description]"
    echo "📝 Example: $0 v1.0.0 \"Initial release\""
    exit 1
fi

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI (gh) is not installed."
    echo "📥 Install: https://cli.github.com/"
    exit 1
fi

echo "🚀 Creating quick release: $VERSION"
echo "📝 Description: $DESCRIPTION"
echo ""

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "❌ Not in a git repository"
    exit 1
fi

# Check if there are uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "⚠️  Warning: You have uncommitted changes"
    read -p "🤔 Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "❌ Aborted"
        exit 1
    fi
fi

# Create and push tag
echo "🏷️  Creating git tag..."
git tag "$VERSION"
git push origin "$VERSION"

echo "⏳ Waiting for GitHub Actions to build binaries..."
echo "🌐 You can monitor the build at: https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\([^.]*\).*/\1/')/actions"

# Wait a bit for the workflow to start
sleep 10

echo ""
echo "✅ Release process initiated!"
echo ""
echo "📋 Next steps:"
echo "1. Wait for GitHub Actions to complete (usually 5-10 minutes)"
echo "2. Check the release at: https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\([^.]*\).*/\1/')/releases"
echo "3. Test the binaries on your Mi device"
echo ""
echo "🤖 The release will include:"
echo "   • http_client (for proxy mode)"
echo "   • http_server (for LLM integration)"
echo "   • multi_mode_client (if available)"
echo "   • config.template.json"
echo "   • boot.sh"
echo ""
echo "📱 Quick test command:"
echo "   curl -L https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\([^.]*\).*/\1/')/releases/download/$VERSION/boot.sh -o boot.sh"
