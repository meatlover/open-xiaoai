import asyncio
import json
import logging
import os
from typing import Any, AsyncGenerator, Dict, List, Optional

import httpx
import websockets


LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO").upper()
logging.basicConfig(level=LOG_LEVEL, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("ai_brain")

WS_HOST = os.getenv("WS_HOST", "0.0.0.0")
WS_PORT = int(os.getenv("WS_PORT", "9000"))

LLM_BASE_URL = os.getenv("LLM_BASE_URL", "http://127.0.0.1:1234")
LLM_MODEL = os.getenv("LLM_MODEL", "local-model")
LLM_API_KEY = os.getenv("LLM_API_KEY", "")


def _llm_headers() -> Dict[str, str]:
    headers = {"Content-Type": "application/json"}
    if LLM_API_KEY:
        headers["Authorization"] = f"Bearer {LLM_API_KEY}"
    return headers


def _llm_url() -> str:
    return f"{LLM_BASE_URL.rstrip('/')}/v1/chat/completions"


async def stream_chat_completion(
    messages: List[Dict[str, str]],
) -> AsyncGenerator[str, None]:
    payload = {
        "model": LLM_MODEL,
        "messages": messages,
        "stream": True,
    }
    timeout = httpx.Timeout(None)
    async with httpx.AsyncClient(timeout=timeout) as client:
        async with client.stream("POST", _llm_url(), headers=_llm_headers(), json=payload) as resp:
            resp.raise_for_status()
            async for line in resp.aiter_lines():
                if not line:
                    continue
                if not line.startswith("data: "):
                    continue
                data = line[6:]
                if data == "[DONE]":
                    break
                try:
                    payload = json.loads(data)
                except json.JSONDecodeError:
                    logger.warning("Failed to parse SSE line: %s", line)
                    continue
                if "error" in payload:
                    raise RuntimeError(payload["error"])
                choices = payload.get("choices", [])
                if not choices:
                    continue
                delta = choices[0].get("delta", {})
                token = delta.get("content")
                if token:
                    yield token


async def send_json(websocket: websockets.WebSocketServerProtocol, payload: Dict[str, Any]) -> None:
    await websocket.send(json.dumps(payload, ensure_ascii=True))


async def handle_connection(websocket: websockets.WebSocketServerProtocol) -> None:
    session_messages: List[Dict[str, str]] = []
    logger.info("Client connected: %s", websocket.remote_address)
    try:
        async for raw_message in websocket:
            try:
                message = json.loads(raw_message)
            except json.JSONDecodeError:
                await send_json(websocket, {"type": "error", "error": "invalid_json"})
                continue

            msg_type = message.get("type")
            if msg_type != "user_input":
                await send_json(websocket, {"type": "error", "error": "unsupported_type"})
                continue

            text = (message.get("text") or "").strip()
            if not text:
                await send_json(websocket, {"type": "error", "error": "empty_text"})
                continue

            session_messages.append({"role": "user", "content": text})
            assistant_text = []

            try:
                async for token in stream_chat_completion(session_messages):
                    assistant_text.append(token)
                    await send_json(websocket, {"type": "llm_token", "token": token})
            except Exception as exc:
                logger.exception("LLM streaming error")
                await send_json(websocket, {"type": "error", "error": str(exc)})
                continue

            if assistant_text:
                session_messages.append({"role": "assistant", "content": "".join(assistant_text)})
            await send_json(websocket, {"type": "llm_end"})
    except websockets.ConnectionClosed:
        logger.info("Client disconnected: %s", websocket.remote_address)


async def main() -> None:
    logger.info("Starting AI-Brain WS on %s:%s", WS_HOST, WS_PORT)
    async with websockets.serve(handle_connection, WS_HOST, WS_PORT):
        await asyncio.Future()


if __name__ == "__main__":
    asyncio.run(main())
