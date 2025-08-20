#!/bin/bash

# Voice Interaction Features Demo Script
# This script demonstrates the new voice interaction features

echo "🎤 Voice Interaction Features Demo"
echo "=================================="
echo

# Check if client binary exists
if [[ ! -f "./target/release/client" ]]; then
    echo "❌ Client binary not found. Please run 'cargo build --release' first."
    exit 1
fi

# Check if config file exists
if [[ ! -f "config.test.json" ]]; then
    echo "❌ Test config file not found. Creating one..."
    cat > config.test.json << 'EOF'
{
  "mode": "direct",
  "openai": {
    "baseURL": "https://api.openai.com/v1",
    "apiKey": "sk-test-key-replace-with-real-key",
    "model": "gpt-3.5-turbo",
    "timeout": 30,
    "maxTokens": 500,
    "temperature": 0.7
  },
  "prompt": {
    "system": "你是一个智能助手，请给出简短的回答。"
  },
  "voice": {
    "customWakeWords": ["小智", "智能助手", "hey assistant"],
    "interruptWords": ["停止", "暂停", "闭嘴", "stop", "pause"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}
EOF
    echo "✅ Created config.test.json - please update the API key before running"
fi

echo "📋 Available demo options:"
echo "1. Test configuration loading"
echo "2. Test with debug output (shows voice feature events)"
echo "3. Run in production mode (requires real device)"
echo "4. Show configuration examples"
echo

read -p "Choose an option (1-4): " choice

case $choice in
    1)
        echo "🧪 Testing configuration loading..."
        ./target/release/client config.test.json --test
        ;;
    2)
        echo "🐛 Testing with debug output..."
        echo "Note: This will show detailed voice interaction events"
        ./target/release/client config.test.json --test --debug
        ;;
    3)
        echo "🚀 Production mode (device required)..."
        echo "Note: This requires deployment to a real XiaoAi device"
        echo "Make sure you have:"
        echo "  - Valid OpenAI API key in config"
        echo "  - Device with audio system access"
        echo "  - Proper file system permissions"
        echo
        read -p "Continue? (y/N): " confirm
        if [[ $confirm == "y" || $confirm == "Y" ]]; then
            ./target/release/client config.test.json --debug
        else
            echo "Cancelled."
        fi
        ;;
    4)
        echo "📖 Configuration Examples:"
        echo
        echo "🔹 Dual Agent Mode (both assistants work together):"
        cat << 'EOF'
{
  "voice": {
    "customWakeWords": ["小智"],
    "interruptWords": ["停止"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}

Usage:
- Say "小爱同学" → Mi's original assistant
- Say "小智" → Your LLM assistant
- Say "停止" → Interrupt any response
EOF
        echo
        echo "🔹 Custom Only Mode (replace original assistant):"
        cat << 'EOF'
{
  "voice": {
    "customWakeWords": ["小智", "助手"],
    "interruptWords": ["停止", "闭嘴"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": false
  }
}

Usage:
- Say "小智" or "助手" → Your LLM assistant
- Original Mi assistant is disabled
- Say "停止" or "闭嘴" → Interrupt response
EOF
        echo
        echo "🔹 Always Listening Mode (no wake words needed):"
        cat << 'EOF'
{
  "voice": {
    "interruptWords": ["停止"],
    "wakeWordEnabled": false,
    "originalAgentEnabled": false
  }
}

Usage:
- Say any command directly (no wake word needed)
- Say "停止" → Interrupt response
- Higher resource usage but most natural
EOF
        ;;
    *)
        echo "❌ Invalid option"
        exit 1
        ;;
esac

echo
echo "✅ Demo completed!"
echo
echo "📚 Additional Resources:"
echo "  - VOICE_FEATURES.md: Complete feature documentation"
echo "  - MIGRATION.md: Upgrade guide from previous versions"
echo "  - config.template.json: Template with all options"
echo
echo "🏗️  To build for deployment:"
echo "  cargo build --release"
echo
echo "🚀 To deploy to device:"
echo "  scp ./target/release/client user@device:/path/to/client"
echo "  scp config.json user@device:/path/to/config.json"
