# Unified Client

The `client` is a unified Rust client that supports both **direct LLM mode** and **proxy mode**, providing a single binary solution for open-xiaoai functionality.

## Features

- 🤖 **Direct LLM Mode**: Connect directly to OpenAI-compatible APIs
- 🔗 **Proxy Mode**: Connect via HTTP server with full feature parity
- ⚙️ **JSON Configuration**: Simple config file-based mode selection
- 📱 **Cross-platform**: Supports ARM (Xiaomi devices) and x86_64 platforms

## Quick Start

### Direct LLM Mode

```json
{
  "mode": "direct",
  "api_endpoint": "https://api.openai.com/v1",
  "api_key": "your-api-key",
  "model": "gpt-4"
}
```

```bash
./client config-direct.json --test
```

### Proxy Mode

```json
{
  "mode": "proxy",
  "proxy_endpoint": "http://your-server:4399"
}
```

```bash
./client config-proxy.json --test
```

## Configuration

### Direct Mode Configuration

```json
{
  "mode": "direct",
  "api_endpoint": "https://api.openai.com/v1",
  "api_key": "your-api-key", 
  "model": "gpt-4",
  "max_tokens": 1000,
  "temperature": 0.7
}
```

### Proxy Mode Configuration

```json
{
  "mode": "proxy",
  "proxy_endpoint": "http://your-server:4399",
  "heartbeat_interval": 30,
  "poll_interval": 1
}
```

## Usage

### Test Mode

```bash
# Test direct LLM connection
./client config-direct.json --test

# Test proxy connection  
./client config-proxy.json --test
```

### Production Mode

```bash
# Run in direct LLM mode
./client config-direct.json

# Run in proxy mode
./client config-proxy.json
```

## Migration Guide

### From http_client

The `client` in proxy mode provides **complete feature parity** with the old `http_client`:

| Feature | `http_client` | `client` (proxy) |
|---------|---------------|------------------|
| Server Registration | ✅ | ✅ |
| Event Polling | ✅ | ✅ |
| Heartbeat | ✅ | ✅ |
| Command Execution | ✅ | ✅ |
| Error Handling | ✅ | ✅ |

**Before:**
```bash
./http_client http://server:4399
```

**After:**
```json
{"mode": "proxy", "proxy_endpoint": "http://server:4399"}
```
```bash
./client config-proxy.json
```

### From multi_mode_client

Direct rename - functionality is identical:

**Before:**
```bash
./multi_mode_client config.json
```

**After:**
```bash
./client config.json
```

## Binary Sizes

- `client`: 1.8MB (proxy + direct modes)

## Architecture

The unified client automatically detects the mode from the configuration file:

- **Direct Mode**: Connects directly to LLM APIs (OpenAI, Anthropic, local models)
- **Proxy Mode**: Connects to HTTP server for centralized management

Both modes provide identical functionality to your Xiaomi device while supporting different deployment architectures.
