# Open-XiaoAI

> [!NOTE]
> 本项目 fork 自 [idootop/open-xiaoai](https://github.com/idootop/open-xiaoai)。

- 🌐 **HTTP-Only 架构**: 完全移除 WebSocket，采用兼容性更强的 HTTP RESTful API
- 🔗 **灵活部署选项**: 支持通过服务器中转或客户端直连 LLM API，满足不同网络环境需求
- 🛡️ **企业级安全**: 集成 Cloudflare Access Service Token 认证机制


### 🚀 部署步骤

1. **刷机更新小爱音箱补丁固件，开启并 SSH 连接到小爱音箱** 👉 [教程](docs/flash.md)

2. **选择部署方式**

   #### 方式一：服务器中转模式（推荐）
   ```bash
   cd examples/migpt
   
   # 配置 LLM API（编辑 config.ts）
   # 支持 OpenAI、302.ai、DeepSeek 等兼容 API
   
   # 启动 HTTP 服务器
   cargo run --release --bin http_server
   ```

   #### 方式二：客户端直连 LLM 模式
   ```bash
   cd packages/client-rust
   
   # 配置 LLM API（编辑 config.json）
   # 支持 OpenAI、302.ai、DeepSeek 等兼容 API
   
   # 编译多模式客户端
   cargo build --release --bin client
   
   # 将客户端上传到小爱音箱
   scp target/release/client root@xiaomi-speaker:/data/
   scp config.json root@xiaomi-speaker:/data/
   ```

3. **在小爱音箱上运行客户端**

   #### 服务器中转模式：
   ```bash
   # 在小爱音箱设备上
   ./client config.json
   # 或使用环境变量
   export SERVER_URL="http://your-server:4399"
   ./client config.json
   ```

   #### 直连 LLM 模式：
   ```bash
   # 在小爱音箱设备上
   cd /data
   ./client config.json
   ```

4. **体验全新的 AI 能力** ✨
   - �️ 自然语言对话（完全替换小爱同学）
   - 🔊 实时语音交互
   - 🧠 多模态 AI 集成
   - 🔐 安全的认证机制（支持 Cloudflare Access）

### 🔧 配置说明

**多模式客户端配置** (`config.json`):
```json
{
  "mode": "direct",
  "openai": {
    "baseURL": "https://api.openai.com/v1",
    "apiKey": "your-api-key",
    "model": "gpt-4"
  },
  "server": {
    "url": "http://your-server:4399"
  }
}
```

**配置参数说明**:
- `mode`: 运行模式
  - `"direct"`: 直连 LLM API 模式
  - `"proxy"`: 通过服务器中转模式
- `openai`: LLM API 配置（直连模式使用）
- `server`: 服务器配置（中转模式使用）

**环境变量支持**:
- `SERVER_URL`: 服务器地址
- `CF_ACCESS_CLIENT_ID`: Cloudflare Access 客户端 ID
- `CF_ACCESS_CLIENT_SECRET`: Cloudflare Access 客户端密钥

### 📦 示例项目

>>>>>>> dev
目前提供以下演示:
- 👉 [小爱音箱接入 MiGPT（完美版）](examples/migpt/README.md) - 基于 HTTP 的稳定实现

以上皆为抛砖引玉，你也可以亲手编写自己想要的功能，一切由你定义！

### 📁 项目结构

```
open-xiaoai/
├── docs/                          # 文档和教程
├── examples/
├── examples/
│   └── migpt/                     # MiGPT 集成示例
│       ├── src/bin/
│       │   └── http_server.rs     # HTTP 服务端（LLM 集成）
│       ├── Dockerfile             # Docker 部署文件
│       ├── deploy-binary.sh       # 二进制文件部署脚本
│       └── deploy-docker.sh       # Docker 容器部署脚本
├── packages/
│   ├── client-rust/               # 核心 HTTP 客户端
│   │   ├── src/bin/
│   │   │   └── client.rs          # 统一客户端（支持直连/代理模式）
│   │   └── config.json            # LLM API 配置
│   ├── client-patch/              # 小爱音箱固件补丁
│   └── flash-tool/                # 刷机工具
└── workspace/                     # 开发和测试工具
    ├── e2e_test.py               # 端到端测试套件
    └── quick_test.sh             # 快速功能测试
```

## 免责声明

1. **适用范围**
   本项目为开源非营利项目，仅供学术研究或个人测试用途。严禁用于商业服务、网络攻击、数据窃取、系统破坏等违反《网络安全法》及使用者所在地司法管辖区的法律规定的场景。
2. **非官方声明**
   本项目由第三方开发者独立开发，与小米集团及其关联方（下称"权利方"）无任何隶属/合作关系，亦未获其官方授权/认可或技术支持。项目中涉及的商标、固件、云服务的所有权利归属小米集团。若权利方主张权益，使用者应立即主动停止使用并删除本项目。

继续下载或运行本项目，即表示您已完整阅读并同意[用户协议](agreement.md)，否则请立即终止使用并彻底删除本项目。

## License

[MIT](LICENSE) License © 2024-PRESENT Del Wang
