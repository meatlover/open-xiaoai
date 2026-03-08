import asyncio
import json
import logging
import os
import re
import tempfile
import uuid
from pathlib import Path
from typing import Any, AsyncGenerator, Dict, List

import edge_tts
import httpx
import websockets
from aiohttp import web


LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO").upper()
logging.basicConfig(level=LOG_LEVEL, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("ai_brain")

WS_HOST = os.getenv("WS_HOST", "0.0.0.0")
WS_PORT = int(os.getenv("WS_PORT", "9000"))
HTTP_PORT = int(os.getenv("HTTP_PORT", "9001"))
# Auto-detect server IP for device to fetch audio files
SERVER_IP = os.getenv("SERVER_IP", "192.168.31.142")

LLM_BASE_URL = os.getenv("LLM_BASE_URL", "http://127.0.0.1:1234")
LLM_MODEL = os.getenv("LLM_MODEL", "local-model")
LLM_API_KEY = os.getenv("LLM_API_KEY", "")

TTS_VOICE = os.getenv("TTS_VOICE", "zh-CN-XiaoxiaoNeural")

SYSTEM_PROMPT = os.getenv("SYSTEM_PROMPT", (
    "You are a helpful voice assistant running on a Xiaomi smart speaker. "
    "Keep responses concise and natural for spoken conversation. "
    "Avoid markdown formatting, bullet points, and code blocks. "
    "Speak in plain sentences. Be brief. "
    "Reply in the same language the user speaks."
))

# Sentence-ending characters for TTS chunking
SENTENCE_ENDINGS = re.compile(r'[.。!！?？\n;；]')
MIN_SENTENCE_CHARS = 8

# Temp dir for TTS audio files
TTS_DIR = Path(tempfile.mkdtemp(prefix="xiaoai-tts-"))
logger.info("TTS audio dir: %s", TTS_DIR)


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
    async with httpx.AsyncClient(timeout=timeout, proxy=None) as client:
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
                    chunk = json.loads(data)
                except json.JSONDecodeError:
                    logger.warning("Failed to parse SSE line: %s", line)
                    continue
                if "error" in chunk:
                    raise RuntimeError(chunk["error"])
                choices = chunk.get("choices", [])
                if not choices:
                    continue
                delta = choices[0].get("delta", {})
                token = delta.get("content")
                if token:
                    yield token


async def send_json(websocket, payload: Dict[str, Any]) -> None:
    await websocket.send(json.dumps(payload, ensure_ascii=True))


async def run_shell_on_device(websocket, script: str) -> None:
    """Send an RPC request to the Rust client to run a shell command on the device."""
    request = {
        "Request": {
            "id": str(uuid.uuid4()),
            "command": "run_shell",
            "payload": script,
        }
    }
    await websocket.send(json.dumps(request))
    logger.info("RPC run_shell: %s", script[:120])


async def tts_play(websocket, text: str) -> None:
    """Generate TTS audio via edge-tts, serve via HTTP, play on device via miplayer."""
    filename = f"{uuid.uuid4().hex}.mp3"
    filepath = TTS_DIR / filename

    try:
        communicate = edge_tts.Communicate(text, TTS_VOICE)
        await communicate.save(str(filepath))
    except Exception as exc:
        logger.error("edge-tts failed: %s", exc)
        return

    audio_url = f"http://{SERVER_IP}:{HTTP_PORT}/audio/{filename}"
    script = f"miplayer -f '{audio_url}'"
    await run_shell_on_device(websocket, script)

    # Clean up old files (keep last 20)
    try:
        files = sorted(TTS_DIR.glob("*.mp3"), key=lambda f: f.stat().st_mtime)
        for old_file in files[:-20]:
            old_file.unlink(missing_ok=True)
    except Exception:
        pass


# --- HTTP file server for TTS audio ---

async def handle_audio_request(request: web.Request) -> web.Response:
    filename = request.match_info["filename"]
    filepath = TTS_DIR / filename
    if not filepath.exists():
        return web.Response(status=404, text="Not found")
    return web.FileResponse(filepath, headers={"Content-Type": "audio/mpeg"})


async def start_http_server() -> None:
    app = web.Application()
    app.router.add_get("/audio/{filename}", handle_audio_request)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, "0.0.0.0", HTTP_PORT)
    await site.start()
    logger.info("HTTP audio server on 0.0.0.0:%s", HTTP_PORT)


# --- WebSocket handler ---

async def handle_connection(websocket) -> None:
    session_messages: List[Dict[str, str]] = [
        {"role": "system", "content": SYSTEM_PROMPT},
    ]
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
                # Ignore non-user_input messages (e.g., RPC responses from device)
                continue

            text = (message.get("text") or "").strip()
            if not text:
                await send_json(websocket, {"type": "error", "error": "empty_text"})
                continue

            logger.info("User said: %s", text)
            session_messages.append({"role": "user", "content": text})
            assistant_text = []
            sentence_buffer = ""

            try:
                async for token in stream_chat_completion(session_messages):
                    assistant_text.append(token)
                    sentence_buffer += token

                    # Check if we have a complete sentence to speak
                    if (SENTENCE_ENDINGS.search(sentence_buffer)
                            and len(sentence_buffer) >= MIN_SENTENCE_CHARS):
                        sentence = sentence_buffer.strip()
                        if sentence:
                            logger.info("TTS sentence: %s", sentence)
                            await tts_play(websocket, sentence)
                        sentence_buffer = ""

            except Exception as exc:
                logger.exception("LLM streaming error")
                await send_json(websocket, {"type": "error", "error": str(exc)})
                continue

            # Flush remaining text
            remaining = sentence_buffer.strip()
            if remaining:
                logger.info("TTS final: %s", remaining)
                await tts_play(websocket, remaining)

            full_response = "".join(assistant_text)
            if full_response:
                session_messages.append({"role": "assistant", "content": full_response})
                logger.info("Full response: %s", full_response[:200])

            await send_json(websocket, {"type": "llm_end"})

    except websockets.ConnectionClosed:
        logger.info("Client disconnected: %s", websocket.remote_address)


async def main() -> None:
    await start_http_server()
    logger.info("Starting AI-Brain WS on %s:%s", WS_HOST, WS_PORT)
    async with websockets.serve(handle_connection, WS_HOST, WS_PORT):
        await asyncio.Future()


if __name__ == "__main__":
    asyncio.run(main())
