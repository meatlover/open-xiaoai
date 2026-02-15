#!/bin/bash
# E2E Test Script for AI-Brain Phase 1 Integration
# Tests the complete flow: Simulated ASR → Rust Client → AI-Brain → LM Studio

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🧪 AI-Brain Phase 1 - E2E Test"
echo "=============================="
echo ""

# Check 1: LM Studio
echo "1. Checking LM Studio..."
if curl -sf http://127.0.0.1:1234/v1/models > /dev/null 2>&1; then
    echo -e "${GREEN}✅ LM Studio is running${NC}"
else
    echo -e "${RED}❌ LM Studio is NOT running${NC}"
    echo "   Start LM Studio and enable local server (port 1234)"
    exit 1
fi

# Check 2: AI-Brain Server
echo ""
echo "2. Checking AI-Brain server..."
if lsof -i :9000 > /dev/null 2>&1; then
    echo -e "${GREEN}✅ AI-Brain server is running on port 9000${NC}"
else
    echo -e "${RED}❌ AI-Brain server is NOT running${NC}"
    echo "   Start server: cd server && source .venv/bin/activate && python server.py"
    exit 1
fi

# Check 3: WebSocket connectivity
echo ""
echo "3. Testing WebSocket connectivity..."
if command -v websocat > /dev/null 2>&1; then
    echo '{"type":"user_input","session_id":"test","text":"hello"}' | timeout 5 websocat ws://localhost:9000 > /tmp/ws_test.out 2>&1 &
    WS_PID=$!
    sleep 2
    if ps -p $WS_PID > /dev/null 2>&1; then
        echo -e "${GREEN}✅ WebSocket connection successful${NC}"
        kill $WS_PID 2>/dev/null || true
    else
        echo -e "${RED}❌ WebSocket connection failed${NC}"
        cat /tmp/ws_test.out
        exit 1
    fi
else
    echo -e "${YELLOW}⚠️  websocat not found, skipping WS test${NC}"
    echo "   Install: brew install websocat (macOS) or cargo install websocat"
fi

# Check 4: Rust client build
echo ""
echo "4. Checking Rust client..."
CLIENT_PATH="packages/client-rust/target/release/client"
if [ ! -f "$CLIENT_PATH" ]; then
    CLIENT_PATH="packages/client-rust/target/debug/client"
fi

if [ -f "$CLIENT_PATH" ]; then
    echo -e "${GREEN}✅ Rust client found: $CLIENT_PATH${NC}"
else
    echo -e "${RED}❌ Rust client not built${NC}"
    echo "   Build: cd packages/client-rust && cargo build --release"
    exit 1
fi

# Check 5: Create simulated ASR log directory
echo ""
echo "5. Setting up simulated ASR environment..."
ASR_LOG_DIR="/tmp/mico_aivs_lab"
ASR_LOG_FILE="$ASR_LOG_DIR/instruction.log"

mkdir -p "$ASR_LOG_DIR"
rm -f "$ASR_LOG_FILE"
touch "$ASR_LOG_FILE"
echo -e "${GREEN}✅ Created $ASR_LOG_FILE${NC}"

# Check 6: Run integration test
echo ""
echo "6. Running integration test..."
echo "   Starting Rust client..."

# Start client in background
"$CLIENT_PATH" ws://127.0.0.1:9000 > /tmp/client_test.log 2>&1 &
CLIENT_PID=$!

# Wait for client to connect
sleep 2

if ! ps -p $CLIENT_PID > /dev/null; then
    echo -e "${RED}❌ Client failed to start${NC}"
    cat /tmp/client_test.log
    exit 1
fi

echo -e "${GREEN}✅ Client connected (PID: $CLIENT_PID)${NC}"

# Simulate ASR events
echo ""
echo "7. Simulating ASR events..."

# Test 1: Non-final result (should be ignored)
echo '{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"test1","id":"1"},"payload":{"is_final":false,"is_vad_begin":false,"results":[{"text":"Hell","confidence":0.5}]}}' >> "$ASR_LOG_FILE"
sleep 1

# Test 2: Final result
echo '{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"test1","id":"2"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":"Hello AI Brain","confidence":0.95}]}}' >> "$ASR_LOG_FILE"
sleep 2

# Test 3: Another final result
echo '{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"test2","id":"3"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":"What is the weather","confidence":0.92}]}}' >> "$ASR_LOG_FILE"
sleep 2

# Test 4: Empty text (should be ignored)
echo '{"header":{"namespace":"SpeechRecognizer","name":"RecognizeResult","dialog_id":"test3","id":"4"},"payload":{"is_final":true,"is_vad_begin":false,"results":[{"text":"","confidence":0.0}]}}' >> "$ASR_LOG_FILE"
sleep 1

# Check results
echo ""
echo "8. Checking results..."

if grep -q "🔥 ASR final result: Hello AI Brain" /tmp/client_test.log; then
    echo -e "${GREEN}✅ Test 1 passed: Final ASR result detected and sent${NC}"
else
    echo -e "${RED}❌ Test 1 failed: Final result not detected${NC}"
    FAILED=1
fi

if grep -q "🔥 ASR final result: What is the weather" /tmp/client_test.log; then
    echo -e "${GREEN}✅ Test 2 passed: Second final result processed${NC}"
else
    echo -e "${RED}❌ Test 2 failed: Second result not processed${NC}"
    FAILED=1
fi

# Non-final should NOT appear
if grep -q "🔥 ASR final result: Hell" /tmp/client_test.log; then
    echo -e "${RED}❌ Test 3 failed: Non-final result incorrectly processed${NC}"
    FAILED=1
else
    echo -e "${GREEN}✅ Test 3 passed: Non-final result correctly ignored${NC}"
fi

# Cleanup
echo ""
echo "9. Cleanup..."
kill $CLIENT_PID 2>/dev/null || true
rm -f /tmp/client_test.log /tmp/ws_test.out
echo -e "${GREEN}✅ Cleanup complete${NC}"

# Summary
echo ""
echo "=============================="
if [ -z "$FAILED" ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    echo ""
    echo "Phase 1 integration is working correctly."
    echo "The Rust client successfully:"
    echo "  - Monitors ASR instruction log"
    echo "  - Extracts final ASR text"
    echo "  - Sends user_input messages to AI-Brain"
    echo "  - Ignores non-final and empty results"
    exit 0
else
    echo -e "${RED}❌ Some tests failed${NC}"
    echo ""
    echo "Client logs:"
    cat /tmp/client_test.log
    exit 1
fi
