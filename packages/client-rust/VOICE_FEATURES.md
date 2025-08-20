# Voice Interaction Features

This document describes the two new voice interaction features implemented in the open-xiaoai client:

## 1. Customized Wake-up Words

### Overview
The client now supports custom wake words while preserving the original Mi agent functionality. This allows users to:
- Define their own wake words (e.g., "小智", "智能助手", "hey assistant")
- Continue using the original "小爱同学" wake word for Mi's default functionality
- Choose whether to run both systems in parallel or disable the original agent

### Configuration
Add the following to your `config.json`:

```json
{
  "voice": {
    "customWakeWords": ["小智", "智能助手", "hey assistant"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}
```

### Parameters
- `customWakeWords`: Array of custom wake words that will trigger your LLM assistant
- `wakeWordEnabled`: If `false`, all voice instructions will be processed without requiring a wake word
- `originalAgentEnabled`: If `false`, the original Mi agent will be interrupted when custom wake words are detected

### How it Works
1. The system monitors the original wake word file (`/tmp/open-xiaoai/kws.log`) for Mi's built-in wake words
2. It also monitors voice instructions (`/tmp/mico_aivs_lab/instruction.log`) for custom wake words
3. When a custom wake word is detected:
   - The wake word state is activated
   - Optionally interrupts the original Mi agent (if `originalAgentEnabled: false`)
   - Subsequent voice instructions are processed by your LLM

## 2. Interrupt Words

### Overview
The client supports interrupt words that can stop the current TTS response mid-playback. This allows users to:
- Stop lengthy responses when they've heard enough
- Interrupt incorrect or unwanted responses
- Provide a more natural conversation experience

### Configuration
Add interrupt words to your `config.json`:

```json
{
  "voice": {
    "interruptWords": ["停止", "暂停", "闭嘴", "stop", "pause"]
  }
}
```

### Parameters
- `interruptWords`: Array of words that will interrupt the current TTS playback

### How it Works
1. The system monitors voice instructions for interrupt words in real-time
2. When an interrupt word is detected during TTS playback:
   - The current TTS session is immediately stopped
   - Any ongoing audio playback is killed
   - The system returns to listening mode
3. Interrupt detection has a 2-second cooldown to prevent false triggers

## Technical Implementation

### State Management
- Uses a global `StateManager` to track TTS sessions
- Each TTS response gets a unique ID for precise interrupt control
- Thread-safe atomic operations ensure reliable state tracking

### Monitoring Services
- `KwsMonitor`: Enhanced to support both original and custom wake words
- `InterruptMonitor`: New service that monitors for interrupt words
- `InstructionMonitor`: Processes voice commands and handles wake word logic

### File Monitoring
The system monitors these key files:
- `/tmp/open-xiaoai/kws.log`: Original wake word detection
- `/tmp/mico_aivs_lab/instruction.log`: Voice instructions and custom wake words
- `/tmp/open-xiaoai/interrupt.log`: Interrupt word detection (created automatically)

## Example Configurations

### Scenario 1: Dual Agent Mode (Recommended)
Both your custom assistant and Mi's original agent work together:

```json
{
  "voice": {
    "customWakeWords": ["小智"],
    "interruptWords": ["停止", "暂停"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": true
  }
}
```

Usage:
- Say "小爱同学" to use Mi's built-in features
- Say "小智" to use your LLM assistant
- Say "停止" to interrupt any response

### Scenario 2: Custom Only Mode
Replace Mi's agent entirely with your custom assistant:

```json
{
  "voice": {
    "customWakeWords": ["小智", "助手"],
    "interruptWords": ["停止", "闭嘴"],
    "wakeWordEnabled": true,
    "originalAgentEnabled": false
  }
}
```

### Scenario 3: Always Listening Mode
Process all voice commands without wake words:

```json
{
  "voice": {
    "interruptWords": ["停止"],
    "wakeWordEnabled": false,
    "originalAgentEnabled": false
  }
}
```

## Usage Examples

### With Wake Words
1. Say: "小智" (custom wake word)
2. Wait for system response
3. Say: "今天天气怎么样？"
4. Listen to LLM response
5. Say: "停止" (if you want to interrupt)

### Always Listening Mode
1. Say: "今天天气怎么样？" (directly)
2. Listen to LLM response
3. Say: "暂停" (to interrupt if needed)

## Debugging

Enable debug mode to see detailed voice interaction logs:

```bash
./client config.json --debug
```

Debug output includes:
- Wake word detection events
- Custom wake word matches
- Interrupt word detection
- TTS session management
- Voice processing decisions

## Troubleshooting

### Wake Words Not Detected
- Check if `wakeWordEnabled` is set to `true`
- Verify custom wake words are in the `customWakeWords` array
- Enable debug mode to see detection events

### Interrupts Not Working
- Ensure interrupt words are in the `interruptWords` array
- Check that TTS is actually playing when you try to interrupt
- Verify the system has permission to kill audio processes

### Both Agents Responding
- If you want only your custom agent, set `originalAgentEnabled: false`
- Check that wake words don't overlap with Mi's built-in triggers

## Performance Notes

- The system uses multiple monitoring tasks running in parallel
- File monitoring is optimized to avoid excessive CPU usage
- State management uses atomic operations for thread safety
- Interrupt detection has minimal latency (< 100ms typically)

## Future Enhancements

Potential improvements for future versions:
- Audio-based wake word detection (instead of text-based)
- Confidence scoring for wake word matches
- User-customizable interrupt sensitivity
- Voice activity detection integration
- Multi-language wake word support
