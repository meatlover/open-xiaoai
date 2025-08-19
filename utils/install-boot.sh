#!/bin/bash

# Install boot script to Mi device
# Usage: ./install-boot.sh [device_ip]

set -e

DEVICE_IP="${1:-192.168.1.100}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BOOT_SCRIPT="$SCRIPT_DIR/boot.sh"

if [ ! -f "$BOOT_SCRIPT" ]; then
    echo "❌ boot.sh not found in $SCRIPT_DIR"
    exit 1
fi

echo "🔧 正在安装自动启动脚本到小爱音箱..."
echo "📱 设备地址: $DEVICE_IP"

# Copy boot script to device
echo "📤 上传 boot.sh 到设备..."
scp "$BOOT_SCRIPT" "root@$DEVICE_IP:/data/boot.sh"

# Make it executable
echo "🔧 设置执行权限..."
ssh "root@$DEVICE_IP" "chmod +x /data/boot.sh"

# Test the script
echo "🧪 测试脚本语法..."
ssh "root@$DEVICE_IP" "sh -n /data/boot.sh && echo '✅ 脚本语法检查通过'"

echo ""
echo "✅ 安装完成！"
echo ""
echo "📋 下一步："
echo "1. 重启设备: ssh root@$DEVICE_IP 'reboot'"
echo "2. 等待设备启动完成（约1-2分钟）"
echo "3. 检查客户端状态: ssh root@$DEVICE_IP 'ps | grep client'"
echo ""
echo "🔧 配置文件位置: /data/open-xiaoai/config.json"
echo "📋 日志查看: ssh root@$DEVICE_IP 'tail -f /var/log/messages'"
