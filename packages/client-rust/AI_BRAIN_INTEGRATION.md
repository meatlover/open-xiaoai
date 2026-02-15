# AI-Brain Integration (Phase 1)

This document describes the integration between the XiaoAi Rust client and the AI-Brain server for Phase 1 of the implementation plan.

## Overview

The Rust client now sends ASR (Automatic Speech Recognition) final results directly to the AI-Brain server using the `user_input` protocol message format defined in `server/implementation-plan.md`.

## Changes Made

### New Module: `src/services/ai_brain.rs`

This module provides:

1. **UserInputMessage** - Data structure for AI-Brain `user_input` messages
   ```rust
   {
     "type": "user_input",
     "session_id": "<uuid>",
     "text": "Hello world"
   }
   ```

2. **extract_asr_text()** - Parses instruction events and extracts final ASR text
   - Filters for `SpeechRecognizer.RecognizeResult` events
   - Only processes events where `is_final == true`
   - Extracts the first result's text field
   - Returns `None` for empty text or non-final results

3. **send_user_input()** - Sends user_input messages to AI-Brain server via WebSocket

4. **new_session_id()** - Generates unique session IDs using UUID v4

### Modified: `src/bin/client.rs`

- Added session ID tracking per connection
- Enhanced instruction monitor callback to:
  1. Send original instruction event (backward compatibility)
  2. Parse ASR final results
  3. Send `user_input` messages to AI-Brain when final text is detected

## Protocol Flow

```
XiaoAi Device → Rust Client → AI-Brain Server → LM Studio
    (ASR)         (WS)            (HTTP/SSE)

1. XiaoAi writes ASR results to /tmp/mico_aivs_lab/instruction.log
2. Rust client monitors the file and detects final results
3. Client sends: {"type":"user_input", "session_id":"...", "text":"..."}
4. AI-Brain proxies to LM Studio and streams tokens back
5. Client receives: {"type":"llm_token", "token":"..."}
6. Client receives: {"type":"llm_end"}
```

## Building

### Prerequisites

- Rust toolchain (1.70+)
- `cross` for ARM cross-compilation: `cargo install cross`
- Docker (for `cross`)

### Build for Device (ARMv7)

```bash
cd packages/client-rust
cross build --release --target armv7-unknown-linux-gnueabihf
```

The binary will be at: `target/armv7-unknown-linux-gnueabihf/release/client`

### Build for Local Testing (native)

```bash
cd packages/client-rust
cargo build --release
```

## Testing

### Unit Tests

```bash
cd packages/client-rust
cargo test
```

Tests included:
- `test_extract_asr_text_final` - Verifies final ASR text extraction
- `test_extract_asr_text_not_final` - Verifies non-final events are ignored
- `test_extract_asr_text_empty` - Verifies empty text is ignored

### Local Integration Test

1. Start AI-Brain server:
   ```bash
   cd server
   source .venv/bin/activate
   export LLM_BASE_URL="http://127.0.0.1:1234"
   export LLM_MODEL="local-model"
   python server.py
   ```

2. Start LM Studio with local server enabled (port 1234)

3. Simulate ASR event by creating a test instruction log:
   ```bash
   mkdir -p /tmp/mico_aivs_lab
   echo '{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"test","id":"1"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":"Hello AI","confidence":0.95}]}}' > /tmp/mico_aivs_lab/instruction.log
   ```

4. Run client (pointing to local AI-Brain):
   ```bash
   ./target/release/client ws://127.0.0.1:9000
   ```

5. Append new ASR results to the log:
   ```bash
   echo '{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"test","id":"2"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":"What is the weather","confidence":0.92}]}}' >> /tmp/mico_aivs_lab/instruction.log
   ```

### Expected Output

**Client console:**
```
✅ 已启动
✅ 已连接: "ws://127.0.0.1:9000"
🔥 ASR final result: Hello AI
🔥 ASR final result: What is the weather
```

**Server console:**
```
2026-02-15 12:34:56 INFO Client connected: ('127.0.0.1', 54321)
2026-02-15 12:34:57 INFO Received: {'type': 'user_input', 'session_id': '...', 'text': 'Hello AI'}
2026-02-15 12:34:58 INFO Received: {'type': 'user_input', 'session_id': '...', 'text': 'What is the weather'}
```

## Device Deployment

1. Build for ARMv7:
   ```bash
   cross build --release --target armv7-unknown-linux-gnueabihf
   ```

2. Copy to device:
   ```bash
   # Using dd + ssh
   dd if=target/armv7-unknown-linux-gnueabihf/release/client \
   | ssh -o HostKeyAlgorithms=+ssh-rsa root@<device-ip> "dd of=/data/open-xiaoai/client"
   
   # Or using scp
   scp -o HostKeyAlgorithms=+ssh-rsa \
       target/armv7-unknown-linux-gnueabihf/release/client \
       root@<device-ip>:/data/open-xiaoai/client
   ```

3. Set AI-Brain server address on device:
   ```bash
   ssh -o HostKeyAlgorithms=+ssh-rsa root@<device-ip>
   echo 'ws://<ai-brain-host>:9000' > /data/open-xiaoai/server.txt
   ```

4. Run on device:
   ```bash
   chmod +x /data/open-xiaoai/client
   /data/open-xiaoai/client $(cat /data/open-xiaoai/server.txt)
   ```

## Phase 1 Checklist

- [x] Rust client monitors ASR instruction log
- [x] Extracts final ASR text from SpeechRecognizer events
- [x] Sends `user_input` messages in AI-Brain protocol format
- [x] Maintains session ID per connection
- [x] Backward compatible (still sends instruction events)
- [ ] End-to-end test with real device + AI-Brain + LM Studio
- [ ] Verify token streaming back to client (Phase 1 scope: client just logs, doesn't play audio yet)

## Next Steps (Phase 2+)

- Add interrupt handling when new input arrives
- Support `user_partial` for streaming ASR input
- Add TTS audio playback from `llm_token` responses
- Implement state machine (IDLE → GENERATING → INTERRUPTED)

## References

- `server/implementation-plan.md` - Full phase roadmap
- `server/README.md` - AI-Brain server setup
- `server/ADMIN.md` - Admin guide and troubleshooting
