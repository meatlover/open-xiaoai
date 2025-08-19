#! /bin/sh

exec > /dev/null 2>&1

cat << 'EOF'

▄▖      ▖▖▘    ▄▖▄▖
▌▌▛▌█▌▛▌▚▘▌▀▌▛▌▌▌▐ 
▙▌▙▌▙▖▌▌▌▌▌█▌▙▌▛▌▟▖
  ▌                 

v2.0.0  by: https://github.com/meatlover/open-xiaoai

EOF

set -e

echo "🤫 等待网络连接中..."

sleep 5

# Configuration
DOWNLOAD_BASE_URL="https://github.com/meatlover/open-xiaoai/releases/latest/download"
WORK_DIR="/data/open-xiaoai"
CONFIG_FILE="$WORK_DIR/config.json"

# Determine which client to use based on configuration
CLIENT_MODE="proxy"  # Default to proxy mode
CLIENT_BIN="$WORK_DIR/client"

if [ ! -d "$WORK_DIR" ]; then
    mkdir -p "$WORK_DIR"
fi

# Read configuration to determine mode
if [ -f "$CONFIG_FILE" ]; then
    CLIENT_MODE=$(grep -o '"mode"[[:space:]]*:[[:space:]]*"[^"]*"' "$CONFIG_FILE" | sed 's/.*"mode"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/' || echo "proxy")
fi

echo "🔧 检测到运行模式: $CLIENT_MODE"

# Download client if not exists
if [ ! -f "$CLIENT_BIN" ]; then
    echo "🔥 正在下载 client 补丁程序..."
    curl -L -# -o "$CLIENT_BIN" "$DOWNLOAD_BASE_URL/client" || {
        echo "❌ 下载失败，尝试从备用源下载..."
        curl -L -# -o "$CLIENT_BIN" "https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-client/client"
    }
    chmod +x "$CLIENT_BIN"
    echo "✅ client 补丁程序下载完毕"
fi

# Download default config if not exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "🔧 正在下载默认配置文件..."
    curl -L -# -o "$CONFIG_FILE" "$DOWNLOAD_BASE_URL/config.template.json" || {
        echo "⚠️  配置文件下载失败，使用默认配置"
        cat > "$CONFIG_FILE" << 'CONFIG_EOF'
{
  "mode": "proxy",
  "openai": {
    "baseURL": "https://api.openai.com/v1",
    "apiKey": "your-api-key-here",
    "model": "gpt-4"
  },
  "server": {
    "url": "http://127.0.0.1:4399"
  }
}
CONFIG_EOF
    }
fi

echo "🔥 正在启动 client 补丁程序..."

# Kill existing client processes
kill -9 $(ps | grep -E "(open-xiaoai|client)" | grep -v grep | awk '{print $1}') > /dev/null 2>&1 || true

# Start client with appropriate config
echo "🚀 启动客户端..."
cd "$WORK_DIR"
"$CLIENT_BIN" "$CONFIG_FILE" > /dev/null 2>&1 &

echo "✅ 客户端已启动，PID: $!"
