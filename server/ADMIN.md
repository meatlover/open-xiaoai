# AI-Brain Admin Guide

## Quick Reference

### Start the server
```bash
source .venv/bin/activate
export LLM_BASE_URL="http://127.0.0.1:1234"
export LLM_MODEL="local-model"
python server.py
```

### Stop the server
```bash
# Find process ID
ps aux | grep "python server.py"

# Kill the process
kill <PID>

# Or force kill if unresponsive
kill -9 <PID>
```

### Check if server is running
```bash
# Check if port 9000 is listening
lsof -i :9000

# Or on Linux
netstat -tuln | grep 9000
```

---

## Key Management Points

### 1. Environment Variables

Critical configuration via environment variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `WS_HOST` | `0.0.0.0` | WebSocket bind address (0.0.0.0 = all interfaces) |
| `WS_PORT` | `9000` | WebSocket port |
| `LLM_BASE_URL` | `http://127.0.0.1:1234` | LM Studio API endpoint |
| `LLM_MODEL` | `local-model` | Model name for LM Studio |
| `LLM_API_KEY` | (empty) | Optional API key for auth |
| `LOG_LEVEL` | `INFO` | Logging verbosity (DEBUG/INFO/WARNING/ERROR) |

**Best practice**: Create a `.env` file or startup script with these values.

---

### 2. Dependencies

Install/upgrade dependencies:
```bash
source .venv/bin/activate
pip install -r requirements.txt

# Upgrade all packages
pip install --upgrade -r requirements.txt
```

If you modify `requirements.txt`, reinstall:
```bash
pip install -r requirements.txt
```

---

### 3. Log Monitoring

Logs are written to stdout with timestamps. To capture logs:

```bash
# Run with log file
python server.py > server.log 2>&1 &

# Tail logs in real-time
tail -f server.log

# Enable debug logging
export LOG_LEVEL="DEBUG"
python server.py
```

**What to look for**:
- `Starting AI-Brain WS on 0.0.0.0:9000` = server started successfully
- `Client connected: ('192.168.1.x', port)` = device connected
- `LLM streaming error` = LM Studio connection failed

---

### 4. Health Checks

#### Basic connectivity test
```bash
# Using websocat (install: brew install websocat)
echo '{"type":"user_input","text":"hello"}' | websocat ws://localhost:9000

# Using Python
python -c "
import asyncio, websockets, json
async def test():
    async with websockets.connect('ws://localhost:9000') as ws:
        await ws.send(json.dumps({'type':'user_input','text':'test'}))
        print(await ws.recv())
asyncio.run(test())
"
```

#### Check LM Studio connectivity
```bash
curl http://127.0.0.1:1234/v1/models
```

---

### 5. Common Issues & Solutions

| Issue | Symptom | Solution |
|-------|---------|----------|
| **Port already in use** | `Address already in use` error | Kill existing process: `lsof -i :9000` then `kill <PID>` |
| **LM Studio not running** | `LLM streaming error: Connection refused` | Start LM Studio and enable local server |
| **Wrong model name** | `LLM streaming error: Model not found` | Check available models: `curl http://127.0.0.1:1234/v1/models` |
| **Firewall blocking** | Clients can't connect from LAN | Open port 9000 in firewall, check `WS_HOST=0.0.0.0` |
| **Module not found** | Import errors on startup | Reinstall dependencies: `pip install -r requirements.txt` |
| **High memory usage** | Server becomes slow | Each connection keeps full conversation history; restart server periodically |

---

### 6. Production Deployment

For long-term deployment, consider:

#### Using systemd (Linux)
Create `/etc/systemd/system/ai-brain.service`:
```ini
[Unit]
Description=AI-Brain WebSocket Server
After=network.target

[Service]
Type=simple
User=youruser
WorkingDirectory=/path/to/server
Environment="LLM_BASE_URL=http://127.0.0.1:1234"
Environment="LLM_MODEL=local-model"
ExecStart=/path/to/server/.venv/bin/python server.py
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

Then:
```bash
sudo systemctl daemon-reload
sudo systemctl enable ai-brain
sudo systemctl start ai-brain
sudo systemctl status ai-brain
```

#### Using screen/tmux (simple)
```bash
screen -S ai-brain
source .venv/bin/activate
python server.py
# Press Ctrl+A, then D to detach

# Reattach later
screen -r ai-brain
```

---

### 7. Security Considerations

**Current limitations (Phase 1)**:
- No authentication
- No rate limiting
- No TLS/WSS support
- Session memory grows unbounded

**Recommendations**:
- Run on trusted LAN only (192.168.1.0/24)
- Use firewall rules to restrict access
- For internet exposure, add reverse proxy with TLS (nginx + certbot)
- Monitor resource usage and restart periodically

---

### 8. Upgrades & Maintenance

#### Update the server code
```bash
git pull  # If using git
source .venv/bin/activate
pip install -r requirements.txt
# Restart server
```

#### Backup session state
Phase 1 has no persistent state. Sessions are lost on restart.

#### Rolling restart (zero-downtime)
Not supported in Phase 1. Clients will disconnect during restart.

---

### 9. Performance Notes

- **Concurrent connections**: Unlimited (asyncio-based)
- **Memory**: ~10-50 KB per active session (conversation history)
- **CPU**: Low (I/O bound, mostly waiting on LLM)
- **Bottleneck**: LM Studio response speed

**Monitoring**:
```bash
# Check memory usage
ps aux | grep "python server.py"

# Check open connections
lsof -i :9000 | wc -l
```

---

## Quick Troubleshooting Checklist

1. ✅ Is LM Studio running? `curl http://127.0.0.1:1234/v1/models`
2. ✅ Is server running? `lsof -i :9000`
3. ✅ Are logs showing errors? Check stdout/log file
4. ✅ Can you connect locally? `websocat ws://localhost:9000`
5. ✅ Is firewall blocking? Check port 9000 rules
6. ✅ Is `WS_HOST` set to `0.0.0.0`? Check env vars
7. ✅ Are dependencies installed? `pip list | grep websockets`
