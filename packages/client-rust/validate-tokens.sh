#!/bin/bash

# Script to validate tokens for 土豆土豆
echo "🔍 Checking available tokens for '土豆土豆' phonetics..."

TOKENS_FILE="/data/open-xiaoai/kws/models/tokens.txt"

if [ ! -f "$TOKENS_FILE" ]; then
    echo "❌ Tokens file not found: $TOKENS_FILE"
    exit 1
fi

echo "📋 Available tokens:"
cat "$TOKENS_FILE"

echo ""
echo "🔍 Checking specific tokens needed for 土豆土豆:"

# Check individual tokens
NEEDED_TOKENS=("t" "ǔ" "d" "òu" "ā" "ī" "ì")

for token in "${NEEDED_TOKENS[@]}"; do
    if grep -q "^$token " "$TOKENS_FILE"; then
        echo "✅ Token '$token' found"
    else
        echo "❌ Token '$token' NOT found"
    fi
done

echo ""
echo "💡 Working example keywords that succeed:"
echo "n ǐ h ǎo x iǎo zh ì @你好小智"
echo "d òu b āo d òu b āo @豆包豆包"

echo ""
echo "🎯 Suggested alternatives for 土豆土豆:"
echo "Option 1 (simplified): t ǔ d ā t ǔ d ā @土豆土豆"
echo "Option 2 (using working tokens): create with available combinations"
