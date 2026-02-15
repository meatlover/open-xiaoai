# Quick Reference: Phase 1 Testing

## Prerequisites

✅ **LM Studio** running on port 1234 with OpenAI-compatible API enabled  
✅ **AI-Brain server** running on port 9000  
✅ **Rust toolchain** installed (for building) OR use pre-built binary  

## Quick Start (5 minutes)

### 1. Start LM Studio
- Download and install LM Studio
- Load a model (e.g., Mistral, Llama)
- Enable "Local Server" on port 1234

### 2. Start AI-Brain Server
```bash
cd server
source .venv/bin/activate  # or: .venv\Scripts\activate on Windows

# Set environment variables
export LLM_BASE_URL="http://127.0.0.1:1234"
export LLM_MODEL="local-model"

# Start server
python server.py
```

Expected output:
```
Starting AI-Brain WS on 0.0.0.0:9000
```

### 3. Build Rust Client (if not already built)
```bash
cd packages/client-rust

# Native build (for testing on your machine)
cargo build --release

# OR for device (ARMv7)
cross build --release --target armv7-unknown-linux-gnueabihf
```

### 4. Run E2E Test
```bash
cd packages/client-rust
./test_e2e.sh
```

Expected output:
```
🧪 AI-Brain Phase 1 - E2E Test
==============================
1. Checking LM Studio...
✅ LM Studio is running
2. Checking AI-Brain server...
✅ AI-Brain server is running on port 9000
...
✅ All tests passed!
```

## Manual Testing

### Test 1: Basic connectivity
```bash
# Install websocat if needed: brew install websocat
echo '{"type":"user_input","session_id":"test","text":"hello"}' | websocat ws://localhost:9000
```

Expected: You should see LLM tokens streaming back

### Test 2: Simulate device ASR
```bash
# Create log directory
mkdir -p /tmp/mico_aivs_lab

# Start client
cd packages/client-rust
./target/release/client ws://127.0.0.1:9000

# In another terminal, simulate ASR event
echo '{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"test","id":"1"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":"What is the weather","confidence":0.95}]}}' >> /tmp/mico_aivs_lab/instruction.log
```

Expected client output:
```
✅ 已启动
✅ 已连接: "ws://127.0.0.1:9000"
🔥 ASR final result: What is the weather
```

Expected server output:
```
Client connected: ('127.0.0.1', 54321)
```

## Troubleshooting

### LM Studio not responding
```bash
curl http://127.0.0.1:1234/v1/models
```
Should return JSON with available models

### AI-Brain not running
```bash
lsof -i :9000
```
Should show Python process listening

### Client can't connect
Check firewall, ensure server is bound to 0.0.0.0, not 127.0.0.1

### No ASR events detected
- Check log file exists: `ls -la /tmp/mico_aivs_lab/instruction.log`
- Check log format matches expected JSON structure
- Look for errors in client output

## Device Deployment

### 1. Copy binary to device
```bash
scp -o HostKeyAlgorithms=+ssh-rsa \
    target/armv7-unknown-linux-gnueabihf/release/client \
    root@<device-ip>:/data/open-xiaoai/client
```

### 2. Set server address
```bash
ssh -o HostKeyAlgorithms=+ssh-rsa root@<device-ip>
echo 'ws://<your-server-ip>:9000' > /data/open-xiaoai/server.txt
```

### 3. Run on device
```bash
chmod +x /data/open-xiaoai/client
/data/open-xiaoai/client $(cat /data/open-xiaoai/server.txt)
```

### 4. Monitor logs
```bash
# On device
tail -f /tmp/mico_aivs_lab/instruction.log

# Check if ASR events are being generated
```

## Network Setup for LAN Access

If server is on different machine than device:

1. **Find your server IP**:
   ```bash
   # macOS/Linux
   ifconfig | grep "inet "
   
   # Windows
   ipconfig
   ```

2. **Update device config**:
   ```bash
   echo 'ws://192.168.1.x:9000' > /data/open-xiaoai/server.txt
   ```

3. **Ensure firewall allows port 9000**:
   ```bash
   # macOS
   # System Settings → Network → Firewall → Allow port 9000
   
   # Linux
   sudo ufw allow 9000
   ```

## Files Reference

- **Implementation**: `packages/client-rust/src/services/ai_brain.rs`
- **Modified client**: `packages/client-rust/src/bin/client.rs`
- **E2E test**: `packages/client-rust/test_e2e.sh`
- **Integration guide**: `packages/client-rust/AI_BRAIN_INTEGRATION.md`
- **Phase summary**: `PHASE1_SUMMARY.md`
- **Server docs**: `server/README.md` and `server/ADMIN.md`

## What's Working (Phase 1)

✅ XiaoAi ASR → Rust client  
✅ Rust client → AI-Brain (user_input messages)  
✅ AI-Brain → LM Studio (OpenAI API)  
✅ LM Studio → AI-Brain (token streaming)  
✅ AI-Brain → Rust client (llm_token messages)  
✅ Session management (UUID per connection)  
✅ Final ASR result filtering  
✅ Non-final result filtering  
✅ Empty text filtering  

## What's NOT Working Yet (Future Phases)

❌ Interrupt/cancel generation  
❌ Partial/streaming ASR input  
❌ Audio playback of LLM responses  
❌ TLS/WSS encryption  
❌ Multi-session management  
❌ Cloud LLM providers  

See `server/implementation-plan.md` for roadmap.
