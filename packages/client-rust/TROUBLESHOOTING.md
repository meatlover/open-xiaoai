# Custom Wake Word Troubleshooting Guide

## Issue: Custom wake words not being detected

### Step 1: Copy the updated files to your device

```bash
# Copy the updated client binary
scp /root/open-xiaoai/packages/client-rust/target/armv7-unknown-linux-musleabihf/release/client root@OH2P:/data/open-xiaoai/

# Copy the debug script
scp /root/open-xiaoai/packages/client-rust/debug_wake_words.sh root@OH2P:/data/open-xiaoai/
```

### Step 2: Run the debug script on your device

```bash
# On your Mi device (OH2P):
cd /data/open-xiaoai
chmod +x debug_wake_words.sh
./debug_wake_words.sh
```

**What to expect:**
- The script will monitor the instruction log for 10 seconds
- Say your custom wake word "小爱老师" during this time
- You should see LOG entries if speech is being processed

### Step 3: Test different scenarios

#### Scenario A: Test original wake word first
```bash
# Say "小爱同学" first, then say "小爱老师"
# This tests if you need to activate the system first
```

#### Scenario B: Test if any speech is detected
```bash
# Say anything (like "你好") and see if it appears in the logs
# This tests if speech recognition is working at all
```

#### Scenario C: Test the updated client
```bash
# Run the updated client with maximum debugging
./client config.json --debug
```

### Step 4: Check the debug output

**If you see no LOG entries when speaking:**
- The microphone or audio system isn't working
- The instruction log isn't being written to
- You may need to say "小爱同学" first to activate the system

**If you see LOG entries but no "Found speech recognition event":**
- The log format is different than expected
- Speech isn't being parsed as RecognizeResult events

**If you see speech events but custom wake words aren't detected:**
- The wake word matching logic has issues
- The speech text format is different

### Step 5: Alternative approaches to try

#### Option 1: Test with simpler wake words
Edit your config.json to use simpler wake words:
```json
{
  "voice": {
    "customWakeWords": ["老师", "助手"],
    "interruptWords": ["停止"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}
```

#### Option 2: Disable wake word requirement temporarily
```json
{
  "voice": {
    "customWakeWords": ["小爱老师"],
    "interruptWords": ["停止"],
    "wakeWordEnabled": false,
    "originalAgentEnabled": false
  }
}
```

#### Option 3: Force wake word detection
Add this to test wake word activation manually:
```bash
# On the device, manually trigger a wake word event
echo "$(date +%s)@小爱老师" >> /tmp/open-xiaoai/kws.log
```

### Step 6: Enhanced debugging commands

```bash
# Check if the audio services are running
ps aux | grep mico

# Check instruction log size and recent activity
ls -la /tmp/mico_aivs_lab/instruction.log
tail -n 20 /tmp/mico_aivs_lab/instruction.log

# Monitor instruction log in real-time while speaking
tail -f /tmp/mico_aivs_lab/instruction.log | grep -i "text\|recognize"

# Check if original wake words work
tail -f /tmp/open-xiaoai/kws.log
```

### Common Issues and Solutions

#### Issue: No speech recognition at all
**Solution:** The microphone might be muted or the audio service isn't running
```bash
# Check microphone status
ubus call mediaplayer player_get_status

# Restart audio service if needed
/etc/init.d/mico_aivs_lab restart
```

#### Issue: Only works after saying "小爱同学"
**Solution:** This is normal behavior. Custom wake words might only work after the system is activated
**Workaround:** Set `wakeWordEnabled: false` to process all speech

#### Issue: Speech is detected but custom wake words don't match
**Solution:** The speech recognition might be returning simplified Chinese or different text
**Debug:** Look at the exact text in the instruction log and adjust your wake words accordingly

### Expected Debug Output

When working correctly, you should see:
```
🐛 Custom wake word monitor starting for: ["小爱老师"]
🐛 Instruction log (potential speech): {"header":...,"payload":{"results":[{"text":"小爱老师","confidence":0.9}]}}
🐛 Found RecognizeResultPayload: is_final=true, 1 results
🐛 Speech result 0: '小爱老师' (confidence: 0.9)
🎯 Custom wake word '小爱老师' detected in speech: '小爱老师'
🎯 Custom wake word detected: '小爱老师'
```

### Next Steps

1. Run the debug script and share the output
2. Try the updated client binary with `--debug`
3. Test different wake word scenarios
4. If still not working, we can modify the detection logic based on what you see in the logs

The key is to first understand what format the speech recognition is using, then adjust our detection logic accordingly.
