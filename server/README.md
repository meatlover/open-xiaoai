# AI-Brain (Phase 1)

Minimal Python WebSocket server that proxies streaming tokens from a local
LM Studio OpenAI-compatible endpoint.

## Requirements

- Python 3.10+
- LM Studio running locally with OpenAI-compatible API enabled

## Setup

```bash
python3 -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

## Run

### Default (listens on all network interfaces)

```bash
source .venv/bin/activate
export LLM_BASE_URL="http://127.0.0.1:1234"
export LLM_MODEL="local-model"
python server.py
```

The server binds to `0.0.0.0:9000` by default, making it accessible from:
- localhost: `ws://127.0.0.1:9000`
- LAN (192.168.1.0/24): `ws://192.168.1.x:9000` (replace x with your host IP)

### Custom configuration

```bash
export WS_HOST="0.0.0.0"        # Bind address (default: 0.0.0.0)
export WS_PORT="9000"            # Port (default: 9000)
export LLM_BASE_URL="http://127.0.0.1:1234"
export LLM_MODEL="local-model"
export LOG_LEVEL="INFO"          # DEBUG, INFO, WARNING, ERROR
python server.py
```

### LAN access notes

- Ensure firewall allows port 9000
- Find your host IP: `ifconfig | grep "inet "` (macOS/Linux) or `ipconfig` (Windows)
- Clients on 192.168.1.0/24 can connect to `ws://<your-ip>:9000`

## WebSocket Protocol

Client -> AI-Brain:

```json
{"type": "user_input", "session_id": "...", "text": "Hello"}
```

AI-Brain -> Client:

```json
{"type": "llm_token", "token": "Hello"}
```

```json
{"type": "llm_end"}
```
