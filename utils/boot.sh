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
HTTP_CLIENT_BIN="$WORK_DIR/http_client"
MULTI_MODE_CLIENT_BIN="$WORK_DIR/multi_mode_client"

if [ ! -d "$WORK_DIR" ]; then
    mkdir -p "$WORK_DIR"
fi

# Read configuration to determine mode
if [ -f "$CONFIG_FILE" ]; then
    CLIENT_MODE=$(grep -o '"mode"[[:space:]]*:[[:space:]]*"[^"]*"' "$CONFIG_FILE" | sed 's/.*"mode"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/' || echo "proxy")
fi

echo "🔧 检测到运行模式: $CLIENT_MODE"

# Download appropriate client based on mode
if [ "$CLIENT_MODE" = "direct" ]; then
    CLIENT_BIN="$MULTI_MODE_CLIENT_BIN"
    CLIENT_NAME="multi_mode_client"
else
    CLIENT_BIN="$HTTP_CLIENT_BIN"
    CLIENT_NAME="http_client"
fi

# Download client if not exists
if [ ! -f "$CLIENT_BIN" ]; then
    echo "🔥 正在下载 $CLIENT_NAME 补丁程序..."
    curl -L -# -o "$CLIENT_BIN" "$DOWNLOAD_BASE_URL/$CLIENT_NAME" || {
        echo "❌ 下载失败，尝试从备用源下载..."
        curl -L -# -o "$CLIENT_BIN" "https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-client/$CLIENT_NAME"
    }
    chmod +x "$CLIENT_BIN"
    echo "✅ $CLIENT_NAME 补丁程序下载完毕"
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

echo "🔥 正在启动 $CLIENT_NAME 补丁程序..."

# Kill existing client processes
kill -9 $(ps | grep -E "(open-xiaoai|http_client|multi_mode_client)" | grep -v grep | awk '{print $1}') > /dev/null 2>&1 || true

# Start appropriate client based on mode
if [ "$CLIENT_MODE" = "direct" ]; then
    echo "🚀 启动直连模式客户端..."
    cd "$WORK_DIR"
    "$CLIENT_BIN" > /dev/null 2>&1 &
else
    echo "🚀 启动代理模式客户端..."
    # Read server URL from config
    SERVER_URL=$(grep -o '"url"[[:space:]]*:[[:space:]]*"[^"]*"' "$CONFIG_FILE" | sed 's/.*"url"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/' || echo "http://127.0.0.1:4399")
    
    # Support legacy server.txt for backward compatibility
    if [ -f "$WORK_DIR/server.txt" ]; then
        SERVER_URL=$(cat "$WORK_DIR/server.txt")
    fi
    
    echo "🌐 连接服务器: $SERVER_URL"
    "$CLIENT_BIN" "$SERVER_URL" > /dev/null 2>&1 &
fi

echo "✅ 客户端已启动，PID: $!"
