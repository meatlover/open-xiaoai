# 自定义唤醒词设置指南 (KWS Setup Guide)

> [!IMPORTANT]
> 本教程适用于已刷机的小爱音箱设备，基于 Sherpa-ONNX 语音识别引擎

## 快速开始

### 1. 设置基础文件结构

在小爱音箱上创建必要的目录和文件：

```shell
# 创建 KWS 目录
mkdir -p /data/open-xiaoai/kws

# 设置自定义唤醒词
cat <<EOF > /data/open-xiaoai/kws/keywords.txt
t iān m āo j īng l íng @天猫精灵
x iǎo d ù x iǎo d ù @小度小度
d òu b āo d òu b āo @豆包豆包
n ǐ h ǎo x iǎo zh ì @你好小智
EOF

# 设置唤醒提示语（可选）
cat <<EOF > /data/open-xiaoai/kws/reply.txt
主人你好，请问有什么吩咐？
EOF
```

### 2. 安装 KWS 服务

```shell
# 下载并运行 KWS 启动脚本
curl -sSfL https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-kws/init.sh | sh
```

### 3. 开机自启动（可选）

```shell
# 下载到 /data/init.sh 实现开机自启动
curl -L -o /data/init.sh https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-kws/init.sh
```

## 配置说明

### 唤醒词格式

`keywords.txt` 文件格式：
```
拼音格式 @显示名称
```

示例：
```
t iān m āo j īng l íng @天猫精灵
x iǎo d ù x iǎo d ù @小度小度
```

### 欢迎语配置

`reply.txt` 文件支持：
- 文字提示语
- 音频文件链接 (http/https/file://)
- 多条提示语（随机选择）

示例：
```txt
主人你好，请问有什么吩咐？
https://example.com/music.wav
file:///usr/share/sound-vendor/AiNiRobot/wakeup_ei_01.wav
```

## 调试和优化

### 调试脚本

运行调试脚本查看语音识别效果：

```shell
curl -sSfL https://gitee.com/idootop/artifacts/releases/download/open-xiaoai-kws/debug.sh | sh
```

调试输出示例：
```
Started! Please speak
0:tiānmāojīnglián 👈 天猫精灵
1:xiǎodùxiǎodù 👈 小度小度
```

根据调试结果调整 `keywords.txt` 中的拼音，提升识别准确率。

## 工作原理

1. **Sherpa-ONNX 引擎**：负责实时语音识别
2. **KWS 监控**：客户端监控 `/tmp/open-xiaoai/kws.log` 获取唤醒事件
3. **配置文件**：
   - `/data/open-xiaoai/kws/keywords.txt` - 唤醒词配置
   - `/data/open-xiaoai/kws/reply.txt` - 欢迎语配置

## 限制说明

- 仅支持中文（普通话）唤醒词
- 受设备算力限制，识别准确率可能不完美
- 不支持说话人识别
- 不支持方言或外语

## 故障排除

### 唤醒词识别不灵敏

1. 使用调试脚本检查识别效果
2. 根据实际识别结果调整拼音
3. 确保发音清晰、音量适中
4. 重启服务使配置生效

### 服务无法启动

1. 检查设备网络连接
2. 确认目录权限正确
3. 检查配置文件格式
4. 查看系统日志

### 配置不生效

1. 重启 KWS 服务
2. 重启小爱音箱
3. 检查文件路径和格式
