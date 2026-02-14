# AI Bridge 扩展包

为 Xiaomi 智能音箱 Pro (OH2P) 提供 Python 3、WebSocket、mTLS 和 SOCKS5 代理支持的扩展包。

## 功能特性

- 🔐 **mTLS 认证** - 基于客户端证书的安全认证
- 🌐 **WebSocket 支持** - 全双工音频/文本流
- 🔒 **OpenSSL 3.x** - 现代 TLS 1.2+ 支持（从系统 OpenSSL 1.0.2 升级）
- 🧦 **SOCKS5 代理** - 受限网络的代理支持
- 📦 **在线/离线安装** - 支持从互联网或本地包安装
- 🔄 **自动更新** - 重新运行安装脚本即可升级
- 🚀 **自动启动** - 服务随系统启动自动运行
- 💾 **OTA 安全** - 存储在 `/data` 分区，OTA 升级后保留

## 系统要求

- **设备**: Xiaomi 智能音箱 Pro (OH2P) 或其他兼容 LEDE/OpenWrt 设备
- **架构**: `aarch64`
- **存储空间**: `/data` 分区需要 35 MB 可用空间
- **网络**: 在线安装需要网络访问

## 快速安装

### 在线安装（推荐）

```bash
curl -fsSL https://raw.githubusercontent.com/meatlover/open-xiaoai/main/packages/ai-bridge/install/install.sh | sh
```

### 使用 SOCKS5 代理安装

```bash
# 使用环境变量
curl -fsSL ... | SOCKS5_PROXY=socks5://user:pass@proxy:1080 sh

# 或使用命令行参数
curl -fsSL ... | sh -s -- --proxy socks5://proxy:1080 --socks5
```

### 离线安装

1. 下载离线安装包：
   ```bash
   wget https://github.com/meatlover/open-xiaoai/releases/download/v1.0.0/ai-bridge-offline-v1.0.0.tar.gz
   ```

2. 解压并安装：
   ```bash
   tar -xzf ai-bridge-offline-v1.0.0.tar.gz
   cd ai-bridge
   ./install-offline.sh
   ```

## 安装选项

```bash
./install.sh [选项]

选项:
  --offline     使用本地包（不需要网络）
  --socks5      安装 redsocks 以支持 SOCKS5 代理
  --proxy URL   使用 SOCKS5 代理下载
  --help        显示帮助信息

环境变量:
  SOCKS5_PROXY  SOCKS5 代理 URL
  SOCKS_PROXY   备用代理变量
  ALL_PROXY     通用代理变量
```

## 安装内容

### 核心包 (~35 MB)
- **Python 3.11** - 编程语言
- **OpenSSL 3.x** - TLS/mTLS 库和 CLI 工具
- **pip** - Python 包管理器
- **websockets** - WebSocket 客户端库
- **PyYAML** - 配置解析

### 可选包
- **redsocks** - 透明 SOCKS5/HTTP 代理隧道
- **ca-bundle** - CA 证书

### 配置文件
- `/data/etc/ai-bridge/config.yaml` - 服务配置
- `/data/etc/ai-bridge/env` - 环境变量
- `/data/etc/ssl/` - mTLS 证书（自动生成）
- `/data/etc/redsocks.conf` - 代理配置（如启用）

### 服务脚本
- `/data/bin/ai-bridge.py` - 主服务
- `/data/bin/ai-bridge-upgrade` - 升级脚本
- `/data/init.d/S99ai-bridge` - 启动脚本
- `/etc/rc.d/S99ai-bridge` - 启动软链接

## 使用说明

### 服务控制

```bash
# 启动
/data/init.d/S99ai-bridge start

# 停止
/data/init.d/S99ai-bridge stop

# 重启
/data/init.d/S99ai-bridge restart

# 查看状态
/data/init.d/S99ai-bridge status
```

### 代理控制（如已安装）

```bash
# 启动 SOCKS5 代理
/data/init.d/S99ai-bridge proxy-start

# 停止 SOCKS5 代理
/data/init.d/S99ai-bridge proxy-stop
```

### mTLS 证书管理

```bash
# 重新生成证书
/data/init.d/S99ai-bridge setup-mtls
```

### 升级

```bash
# 升级到最新版本
/data/bin/ai-bridge-upgrade

# 或重新运行安装器
curl -fsSL https://raw.githubusercontent.com/meatlover/open-xiaoai/main/packages/ai-bridge/install/install.sh | sh
```

## 配置说明

### WebSocket 服务器

编辑 `/data/etc/ai-bridge/config.yaml`：

```yaml
websocket:
  # Configure your WebSocket server endpoint
  # url: "wss://your-server.example.com:8765"
  reconnect:
    enabled: true
    max_attempts: 10
    delay: 5
```

### mTLS 设置

证书在首次启动时自动生成：
- CA 证书: `/data/etc/ssl/ca.crt`
- 客户端证书: `/data/etc/ssl/client.crt`
- 客户端密钥: `/data/etc/ssl/client.key`

将 `ca.crt` 复制到服务器以验证客户端。

### SOCKS5 代理

编辑 `/data/etc/redsocks.conf`：

```conf
redsocks {
    local_ip = 127.0.0.1;
    local_port = 12345;
    ip = your.proxy.host;
    port = 1080;
    type = socks5;
}
```

在环境变量中启用：
```bash
echo 'REDSOCKS_ENABLED=true' >> /data/etc/ai-bridge/env
```

## 架构

### 存储布局

```
/data/
├── opt/                    # Entware 安装目录
│   ├── bin/python3
│   ├── lib/libssl.so.3
│   └── ...
├── bin/
│   ├── ai-bridge.py       # 主服务
│   └── ai-bridge-upgrade  # 升级脚本
├── etc/
│   ├── ai-bridge/
│   │   ├── config.yaml    # 服务配置
│   │   └── env            # 环境变量
│   ├── ssl/               # mTLS 证书
│   └── redsocks.conf      # 代理配置
├── init.d/
│   └── S99ai-bridge       # 启动脚本
└── log/
    └── ai-bridge.log      # 服务日志
```

### 网络流程

```
[音频采集] --(PCM)--> [AI Bridge] --(WSS/mTLS)--> [远程 AI 服务器]
                                     <--(文本/音频)--
```

使用 SOCKS5 代理时：
```
[AI Bridge] --(SOCKS5)--> [redsocks@localhost:12345] --> [代理] --> [服务器]
```

## 安全

### mTLS 认证

1. 客户端在首次运行时生成 CA + 客户端证书
2. 客户端在 TLS 握手期间出示证书
3. 服务器根据 CA 验证客户端证书
4. 服务器出示自己的证书（由客户端验证）

### 证书详情
- CA: 4096 位 RSA，10 年有效期
- 客户端: 2048 位 RSA，1 年有效期
- TLS: 仅 1.2+（无 SSLv3、TLS 1.0/1.1）
- 加密套件: 仅现代安全套件

### SOCKS5 认证

redsocks 支持 SOCKS5 认证：
```conf
redsocks {
    ...
    login = "username";
    password = "password";
}
```

## 故障排除

### 查看日志

```bash
# 服务日志
tail -f /data/log/ai-bridge.log

# 系统日志
logread | grep ai-bridge

# dmesg
dmesg | grep -i error
```

### 常见问题

**1. 空间不足**
```bash
df -h /data  # 检查可用空间
# 需要至少 50MB 可用空间
```

**2. 权限拒绝**
```bash
# 确保脚本可执行
chmod +x /data/init.d/S99ai-bridge
chmod +x /data/bin/ai-bridge.py
```

**3. 找不到 Python**
```bash
# 添加 Entware 到 PATH
export PATH="/data/opt/bin:$PATH"
```

**4. mTLS 证书错误**
```bash
# 重新生成证书
/data/init.d/S99ai-bridge setup-mtls
```

**5. WebSocket 连接失败**
```bash
# 使用 curl 测试
curl -v --cacert /data/etc/ssl/ca.crt \
  https://your-server:8765
```

### 调试模式

启用调试日志：
```bash
# 编辑配置
sed -i 's/level: "INFO"/level: "DEBUG"/' /data/etc/ai-bridge/config.yaml

# 重启
/data/init.d/S99ai-bridge restart
```

## 开发

### 仓库结构

```
packages/ai-bridge/
├── install/
│   └── install.sh          # 主安装脚本
├── packages/
│   ├── manifest.json       # 包元数据
│   └── aarch64-k3.10/      # 下载的包
├── src/
│   └── ai-bridge/
│       ├── main.py         # 服务代码
│       └── config.yaml     # 配置模板
├── config/
│   └── init.d/
│       └── S99ai-bridge    # 启动脚本
└── .github/
    └── workflows/
        └── build-and-release.yml
```

### 构建离线包

```bash
# 下载包
./scripts/download-packages.sh

# 创建发布
tar -czf ai-bridge-offline-v1.0.0.tar.gz \
  install/ config/ src/ packages/manifest.json
```

## 许可证

MIT 许可证

## 致谢

- [Entware](https://entware.net/) - 嵌入式设备包仓库
- [OpenWrt](https://openwrt.org/) - 嵌入式设备 Linux 发行版
- [websockets](https://websockets.readthedocs.io/) - Python WebSocket 库
