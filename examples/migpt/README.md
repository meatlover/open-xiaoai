# Open-XiaoAI x MiGPT-Next

[Open-XiaoAI](https://github.com/idootop/open-xiaoai) 的 Node.js 版 Server 端，用来演示小爱音箱接入[MiGPT](https://github.com/idootop/mi-gpt)（完美版）。

相比原版的 `MiGPT` 和 `MiGPT-Next` 项目，该版本可以完美打断小爱音箱的回复，响应延迟更低，效果更完美 👍

## 快速开始

> [!NOTE]
> 继续下面的操作之前，你需要先在小爱音箱上启动运行 Rust 补丁程序 [👉 教程](../../packages/client-rust/README.md)

首先，克隆仓库代码到本地。

```shell
# 克隆代码
git clone https://github.com/idootop/open-xiaoai.git

# 进入当前项目根目录
cd examples/migpt
```

然后把 `config.ts` 文件里的配置修改成你自己的。

```typescript
export const kOpenXiaoAIConfig = {
  openai: {
    model: "gpt-4.1-mini",
    baseURL: "https://api.openai.com/v1",
    apiKey: "sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxx",
  },
  prompt: {
    system: "你是一个智能助手，请根据用户的问题给出回答。",
  },
  async onMessage(engine, { text }) {
    if (text === "测试") {
      return { text: "你好，很高兴认识你！" };
    }
  },
};
```

### HTTP 服务器运行

除了 Node.js 版本的 MiGPT 服务器，本目录还包含独立的 HTTP 服务器实现：

```shell
# 启动 HTTP 服务器（Rust 版本）
cargo run --release --bin http_server

# 部署方式一：Docker 容器部署
./deploy-docker.sh <remote-server-ip>

# 部署方式二：直接部署二进制文件
./deploy-binary.sh <remote-server-ip>
```

### Docker 运行

[![Docker Image Version](https://img.shields.io/docker/v/idootop/open-xiaoai-migpt?color=%23086DCD&label=docker%20image)](https://hub.docker.com/r/idootop/open-xiaoai-migpt)

推荐使用以下命令，直接 Docker 一键运行。

```shell
docker run -it --rm -p 4399:4399 -v $(pwd)/config.ts:/app/config.ts idootop/open-xiaoai-migpt:latest
```

### HTTP 服务器运行

除了 Node.js 版本的 MiGPT 服务器，本目录还包含独立的 HTTP 服务器实现：

```shell
# 启动 HTTP 服务器（Rust 版本）
cargo run --release --bin http_server

# 部署方式一：Docker 容器部署
./deploy-docker.sh <remote-server-ip>

# 部署方式二：直接部署二进制文件
./deploy-binary.sh <remote-server-ip>
```

### 编译运行

> [!TIP]
> 如果你是一名开发者，想要修改源代码实现自己想要的功能，可以按照下面的步骤，自行编译运行该项目。

为了能够正常编译运行该项目，你需要安装以下依赖环境：

- Node.js v22.x: https://nodejs.org/zh-cn/download
- Rust: https://www.rust-lang.org/learn/get-started

准备好开发环境后，按以下步骤即可正常启动该项目。

```bash
# 启用 PNPM 包管理工具
corepack enable && corepack install

# 安装依赖
pnpm install

# 编译运行
pnpm dev
```

## 注意事项

1. 默认 HTTP Server 服务端口为 `4399`，运行前请确保该端口未被其他程序占用。

2. HTTP 服务器专注于 LLM 集成，不处理音频流。如需音频功能，请使用原始的 WebSocket 版本或扩展当前实现。

> [!NOTE]
> 本项目只是一个简单的演示程序，抛砖引玉。如果你想要更多的功能，比如唤醒词识别、语音转文字、连续对话等（甚至是对接 OpenAI 的 [Realtime API](https://platform.openai.com/docs/guides/realtime)），可参考本项目代码自行实现。
