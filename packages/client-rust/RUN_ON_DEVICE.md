# Running Open-XiaoAI Client with Voice Features on Mi Device

## Files you need:

1. **ARM Binary**: `/tmp/client` (already compiled)
2. **Config File**: `config-voice.json` (sample provided)

## Deployment Steps:

### 1. Copy files to Mi device
```bash
# Replace YOUR_MI_IP with your Mi device IP address
MI_IP="192.168.1.100"

# Copy binary
scp /tmp/client root@$MI_IP:/tmp/open-xiaoai-client

# Copy config  
scp config-voice.json root@$MI_IP:/tmp/config-voice.json
```

### 2. SSH into Mi device and run
```bash
ssh root@$MI_IP
cd /tmp
chmod +x open-xiaoai-client

# Run with voice features
./open-xiaoai-client config-voice.json
```

## Expected Output:

When running with voice features, you should see:
```
📋 Loading config from: config-voice.json
🏭 Running in production mode
🚀 Starting direct mode production client...
🎤 Voice features enabled
🎤 Starting voice-enabled direct mode
🐛 Debug: Voice config - wake_words: ["小爱老师", "土豆土豆"], interrupt_words: ["停止", "闭嘴", "安静"], original_agent: false
📝 Created keywords.txt with: ["小爱老师", "土豆土豆"]
📝 Created reply.txt with default responses
🎤 Starting simple KWS service for words: ["小爱老师", "土豆土豆"]
🎯 Starting custom wake word monitoring for: ["小爱老师", "土豆土豆"]
🛑 Starting interrupt word monitoring for: ["停止", "闭嘴", "安静"]
🎤 Wake word monitoring started
🛑 Interrupt word monitoring started
🔄 Voice monitoring active, press Ctrl+C to stop
```

## What it does:

1. **Creates KWS config files** in `/data/open-xiaoai/kws/`:
   - `keywords.txt` - Your custom wake words
   - `reply.txt` - Response phrases

2. **Monitors speech recognition** from `/tmp/mico_aivs_lab/instruction.log`

3. **Detects custom wake words** like "小爱老师" and "土豆土豆"

4. **Writes to KWS log** at `/tmp/open-xiaoai/kws.log` when wake words are detected

5. **Handles interrupts** with words like "停止", "闭嘴", "安静"

## Testing:

Once running, try saying:
- "小爱老师" or "土豆土豆" (should trigger wake word detection)
- "停止" or "闭嘴" (should trigger interrupt)

You'll see debug output showing the detection process.

## Config Options:

```json
{
  "voice": {
    "customWakeWords": ["小爱老师", "土豆土豆"],     // Your custom wake words
    "interruptWords": ["停止", "闭嘴", "安静"],        // Words to interrupt TTS
    "originalAgentEnabled": false,                    // Keep Mi's agent enabled?
    "debug": true                                     // Show debug output?
  }
}
```
