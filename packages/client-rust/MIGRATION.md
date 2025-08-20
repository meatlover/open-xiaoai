# Migration Guide: Adding Voice Interaction Features

This guide helps you upgrade your existing open-xiaoai client configuration to use the new voice interaction features.

## Quick Migration

### Step 1: Add Voice Configuration
Add this section to your existing `config.json`:

```json
{
  // ... your existing config ...
  "voice": {
    "customWakeWords": ["小智"],
    "interruptWords": ["停止", "暂停"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}
```

### Step 2: Test the Configuration
```bash
./client config.json --test --debug
```

### Step 3: Deploy and Test Voice Commands
1. Deploy the updated client to your device
2. Say your custom wake word (e.g., "小智")
3. Give a voice command
4. Try interrupting with a stop word

## Detailed Migration Examples

### From Basic Configuration

**Before:**
```json
{
  "mode": "direct",
  "openai": {
    "baseURL": "https://api.openai.com/v1",
    "apiKey": "your-api-key",
    "model": "gpt-3.5-turbo"
  },
  "prompt": {
    "system": "你是一个智能助手"
  }
}
```

**After:**
```json
{
  "mode": "direct",
  "openai": {
    "baseURL": "https://api.openai.com/v1",
    "apiKey": "your-api-key",
    "model": "gpt-3.5-turbo"
  },
  "prompt": {
    "system": "你是一个智能助手"
  },
  "voice": {
    "customWakeWords": ["小智", "智能助手"],
    "interruptWords": ["停止", "暂停", "闭嘴"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}
```

### From Proxy Mode Configuration

**Before:**
```json
{
  "mode": "proxy",
  "serverProxy": {
    "baseURL": "http://localhost:3000"
  },
  "prompt": {
    "system": "你是小米音箱的智能助手"
  }
}
```

**After:**
```json
{
  "mode": "proxy",
  "serverProxy": {
    "baseURL": "http://localhost:3000"
  },
  "prompt": {
    "system": "你是小米音箱的智能助手"
  },
  "voice": {
    "customWakeWords": ["智能助手"],
    "interruptWords": ["停止", "暂停"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}
```

## Configuration Strategies

### Strategy 1: Parallel Mode (Recommended for first-time users)
Both your assistant and Mi's original assistant work side by side:

```json
{
  "voice": {
    "customWakeWords": ["小智"],
    "interruptWords": ["停止"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}
```

**Pros:**
- Safe fallback to original functionality
- Easy to test and compare
- Preserves existing user habits

**Cons:**
- Potential confusion about which assistant will respond
- May need to learn different wake words

### Strategy 2: Replacement Mode (For advanced users)
Your assistant replaces the original Mi assistant:

```json
{
  "voice": {
    "customWakeWords": ["小爱", "助手"],
    "interruptWords": ["停止", "闭嘴"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": false
  }
}
```

**Pros:**
- Single, consistent assistant experience
- Can use familiar wake words
- Full control over all interactions

**Cons:**
- Loses access to Mi's built-in skills
- Requires more setup and testing

### Strategy 3: Always-On Mode (For specific use cases)
No wake words required, processes all speech:

```json
{
  "voice": {
    "interruptWords": ["停止"],
    "wakeWordEnabled": false,
    "originalAgentEnabled": false
  }
}
```

**Pros:**
- Most natural conversation flow
- No need to remember wake words
- Best for single-user scenarios

**Cons:**
- Higher resource usage
- May activate unexpectedly
- Privacy considerations

## Testing Your Migration

### 1. Validate Configuration
```bash
# Test configuration loading
./client config.json --test

# Test with debug output
./client config.json --test --debug
```

### 2. Test Wake Word Detection
1. Deploy to device
2. Say your custom wake word
3. Check logs for detection confirmation
4. Give a test command

### 3. Test Interrupt Functionality
1. Ask a long question that generates a lengthy response
2. Say an interrupt word mid-response
3. Verify the response stops immediately
4. Confirm the system returns to listening mode

### 4. Test Dual Mode (if enabled)
1. Test original Mi wake word still works
2. Test your custom wake words work
3. Verify they don't interfere with each other

## Common Migration Issues

### Issue: Wake Words Not Detected
**Symptoms:** Custom wake words don't trigger the assistant

**Solutions:**
1. Check `wakeWordEnabled` is `true`
2. Verify wake words are in the correct array format
3. Test with debug mode to see detection events
4. Try simpler, more distinct wake words

### Issue: Both Assistants Responding
**Symptoms:** Both your assistant and Mi's assistant respond to commands

**Solutions:**
1. Set `originalAgentEnabled: false` to disable Mi's assistant
2. Choose wake words that don't conflict with "小爱同学"
3. Adjust the timing of your commands

### Issue: Interrupts Not Working
**Symptoms:** Interrupt words don't stop TTS playback

**Solutions:**
1. Verify interrupt words are correctly configured
2. Check that TTS is actually playing when you interrupt
3. Try different interrupt words
4. Enable debug mode to see interrupt detection

### Issue: Configuration Not Loading
**Symptoms:** Client starts but ignores voice configuration

**Solutions:**
1. Validate JSON syntax with a validator
2. Check file permissions
3. Verify the config file path is correct
4. Look for syntax errors in the log output

## Rollback Plan

If you need to revert to the previous behavior:

### Option 1: Disable Voice Features
```json
{
  // ... your existing config ...
  "voice": {
    "wakeWordEnabled": false,
    "originalAgentEnabled": true
  }
}
```

### Option 2: Remove Voice Section
Simply remove the entire `"voice": { ... }` section from your config file.

### Option 3: Use Previous Binary
Keep a backup of your previous client binary:
```bash
cp ./target/release/client ./target/release/client.backup
# Use the backup if needed
```

## Performance Considerations

### Resource Usage
The new voice features add:
- 3 additional monitoring tasks (wake words, instructions, interrupts)
- Real-time file monitoring
- State management overhead

### Optimization Tips
1. Use fewer, shorter wake words for better performance
2. Keep interrupt word lists concise
3. Monitor system resources if running on constrained devices
4. Consider disabling debug mode in production

## Support and Troubleshooting

### Enable Debug Logging
Always test new configurations with debug mode first:
```bash
./client config.json --debug
```

### Check Log Files
Monitor these files for troubleshooting:
- `/tmp/open-xiaoai/kws.log` - Wake word events
- `/tmp/mico_aivs_lab/instruction.log` - Voice instructions
- `/tmp/open-xiaoai/interrupt.log` - Interrupt detection

### Common Debug Output
Look for these messages:
- `🎯 Custom wake word detected: 'word'` - Your wake word was recognized
- `🛑 Interrupt word detected: 'word'` - Interrupt was triggered
- `🔊 Starting TTS response (ID: 123)` - TTS started
- `🛑 TTS interrupted during playback` - Interrupt succeeded

## Next Steps

After successful migration:
1. Customize wake words to match your preferences
2. Fine-tune interrupt words based on usage patterns
3. Experiment with different configuration strategies
4. Consider automating deployment for multiple devices
5. Share your configuration with the community

## Getting Help

If you encounter issues during migration:
1. Check the debug output first
2. Review the VOICE_FEATURES.md documentation
3. Test with a minimal configuration
4. Report issues with detailed logs and configuration examples
