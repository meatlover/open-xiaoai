#!/usr/bin/env python3
"""
Simple test client for AI-Brain WebSocket server.
"""
import asyncio
import json
import sys

import websockets


async def test_conversation():
    uri = "ws://127.0.0.1:9000"
    print(f"Connecting to {uri}...")
    
    async with websockets.connect(uri) as websocket:
        print("✓ Connected to AI-Brain server\n")
        
        # Test 1: Simple greeting
        test_messages = [
            "Hello, who are you?",
            "What is 2+2?",
            "Tell me a short joke about programming."
        ]
        
        for i, msg in enumerate(test_messages, 1):
            print(f"[Test {i}] User: {msg}")
            
            # Send user input
            await websocket.send(json.dumps({
                "type": "user_input",
                "session_id": "test-session",
                "text": msg
            }))
            
            print(f"[Test {i}] Assistant: ", end="", flush=True)
            
            # Receive streaming tokens
            full_response = []
            while True:
                try:
                    response = await websocket.recv()
                    data = json.loads(response)
                    
                    if data["type"] == "llm_token":
                        token = data["token"]
                        full_response.append(token)
                        print(token, end="", flush=True)
                    elif data["type"] == "llm_end":
                        print("\n")
                        break
                    elif data["type"] == "error":
                        print(f"\n✗ Error: {data.get('error')}")
                        break
                except websockets.ConnectionClosed:
                    print("\n✗ Connection closed unexpectedly")
                    return
            
            print()  # Extra newline between tests


async def main():
    try:
        await test_conversation()
        print("✓ All tests completed successfully")
    except ConnectionRefusedError:
        print("✗ Could not connect to server. Is it running?")
        print("  Start with: python server.py")
        sys.exit(1)
    except Exception as e:
        print(f"✗ Test failed: {e}")
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
