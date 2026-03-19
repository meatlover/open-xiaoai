import asyncio
import json
import logging
import os
import re
import tempfile
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any, AsyncGenerator, Dict, List, Optional

import edge_tts
import httpx
import websockets


LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO").upper()
logging.basicConfig(level=LOG_LEVEL, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("ai_brain")

# Dedicated conversation log (JSONL) — full user/assistant turns, no truncation
CONVERSATION_LOG_DIR = Path(os.getenv(
    "CONVERSATION_LOG_DIR",
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "logs"),
))
CONVERSATION_LOG_DIR.mkdir(parents=True, exist_ok=True)
CONVERSATION_LOG = CONVERSATION_LOG_DIR / "conversations.jsonl"


def log_conversation_turn(
    role: str, content: str, *, model: str = "", client: str = "",
    provider: str = "",
) -> None:
    """Append a conversation turn to the JSONL log."""
    entry = {
        "ts": datetime.now().isoformat(),
        "provider": provider,
        "model": model,
        "client": client,
        "role": role,
        "content": content,
    }
    try:
        with open(CONVERSATION_LOG, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")
    except Exception:
        logger.exception("Failed to write conversation log")

WS_HOST = os.getenv("WS_HOST", "0.0.0.0")
WS_PORT = int(os.getenv("WS_PORT", "9000"))

LLM_FIRST_TOKEN_TIMEOUT = float(os.getenv("LLM_FIRST_TOKEN_TIMEOUT", "10"))
LLM_FULL_RESPONSE_TIMEOUT = float(os.getenv("LLM_FULL_RESPONSE_TIMEOUT", "30"))

TTS_VOICE = os.getenv("TTS_VOICE", "zh-CN-XiaoxiaoNeural")

CONTINUOUS_MODE = os.getenv("CONTINUOUS_MODE", "false").lower() == "true"

# --- LLM Provider configuration ---
# Each provider: name, base_url, api_key, model, system_prompt (optional override)
# Providers are tried in order. First success wins; on failure, next is tried.

def _parse_providers() -> List[Dict[str, str]]:
    """Build ordered provider list from env vars.

    Format:  LLM_PROVIDERS=name1,name2,...
    Per-provider:
        LLM_{NAME}_BASE_URL, LLM_{NAME}_API_KEY, LLM_{NAME}_MODEL
        LLM_{NAME}_SYSTEM_PROMPT  (optional, overrides default)
    """
    providers: List[Dict[str, str]] = []
    names_str = os.getenv("LLM_PROVIDERS", "")

    if names_str:
        for name in names_str.split(","):
            name = name.strip()
            if not name:
                continue
            upper = name.upper().replace("-", "_")
            base_url = os.getenv(f"LLM_{upper}_BASE_URL", "")
            if not base_url:
                logger.warning("Provider '%s' missing LLM_%s_BASE_URL, skipping", name, upper)
                continue
            providers.append({
                "name": name,
                "base_url": base_url,
                "api_key": os.getenv(f"LLM_{upper}_API_KEY", ""),
                "model": os.getenv(f"LLM_{upper}_MODEL", ""),
                "system_prompt": os.getenv(f"LLM_{upper}_SYSTEM_PROMPT", ""),
                "no_system_prompt": os.getenv(f"LLM_{upper}_NO_SYSTEM_PROMPT", "") == "1",
                "user": os.getenv(f"LLM_{upper}_USER", ""),
                "no_proxy": os.getenv(f"LLM_{upper}_NO_PROXY", "") == "1",
            })

    # Fallback: legacy single-provider env vars
    if not providers:
        providers.append({
            "name": "default",
            "base_url": os.getenv("LLM_BASE_URL", "http://127.0.0.1:1234"),
            "api_key": os.getenv("LLM_API_KEY", ""),
            "model": os.getenv("LLM_MODEL", "local-model"),
            "system_prompt": "",
        })

    return providers


PROVIDERS = _parse_providers()
logger.info("LLM providers (in precedence order): %s", [p["name"] for p in PROVIDERS])

# --- Smart home command filter (keyword-based, no LLM) ---
# Device keywords and action keywords. A command is detected when BOTH match.
# Configurable via env: comma-separated lists.
SMARTHOME_DEVICES = os.getenv(
    "SMARTHOME_DEVICES", "灯,空调"
).split(",")
SMARTHOME_ACTIONS = os.getenv(
    "SMARTHOME_ACTIONS", "开,关,打开,关闭,关掉,关上,打开,开启,调高,调低,调到,调亮,调暗,变亮,变暗,设到,设为,设置,度,模式,制冷,制热,除湿"
).split(",")


def is_smart_home_command(text: str) -> bool:
    """Fast keyword check: text must contain a device word AND an action word.

    This avoids false positives like "空调怎么选" (question about AC, no action)
    while catching "开空调", "把灯关掉", "空调调到26度" etc.
    """
    has_device = any(d in text for d in SMARTHOME_DEVICES)
    if not has_device:
        return False
    has_action = any(a in text for a in SMARTHOME_ACTIONS)
    return has_action


# --- Session reset phrases (firmware built-in commands) ---
# "开启新的对话" activates firmware continuous-dialogue mode, which causes
# both the firmware AI and the brain to respond (double replies).
# We intercept these, reset the brain session, and cancel continuous mode.
SESSION_RESET_PHRASES = os.getenv(
    "SESSION_RESET_PHRASES", "开启新的对话,新的对话,开始新对话,重新开始"
).split(",")


def is_session_reset_command(text: str) -> bool:
    """Check if the text is a session-reset / new-conversation command."""
    return any(p in text for p in SESSION_RESET_PHRASES)


# Default system prompt (used when provider has no override)
DEFAULT_SYSTEM_PROMPT = os.getenv("SYSTEM_PROMPT", (
    "You are a helpful voice assistant running on a Xiaomi smart speaker. "
    "Keep responses concise and natural for spoken conversation. "
    "Avoid markdown formatting, bullet points, and code blocks. "
    "Speak in plain sentences. Be brief. "
    "Reply in the same language the user speaks."
))

# Web search tool for function calling
BRAVE_API_KEY = os.getenv("BRAVE_API_KEY", "")
TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "web_search",
            "description": "Search the web for current information including today's date, news, weather, real-time data, or anything you are unsure about.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query string",
                    }
                },
                "required": ["query"],
            },
        },
    }
]
MAX_TOOL_ROUNDS = 3

# Sentence-ending characters for TTS chunking
SENTENCE_ENDINGS = re.compile(r'[.。!！?？\n;；]')
MIN_SENTENCE_CHARS = 8


def strip_markdown(text: str) -> str:
    """Remove markdown formatting so TTS reads pure natural text."""
    # Bold/italic: ***text***, **text**, *text*, ___text___, __text__, _text_
    s = re.sub(r'\*{1,3}(.+?)\*{1,3}', r'\1', text)
    s = re.sub(r'_{1,3}(.+?)_{1,3}', r'\1', s)
    # Strikethrough: ~~text~~
    s = re.sub(r'~~(.+?)~~', r'\1', s)
    # Inline code: `text`
    s = re.sub(r'`(.+?)`', r'\1', s)
    # Headers: # text
    s = re.sub(r'^#{1,6}\s+', '', s, flags=re.MULTILINE)
    # Unordered list markers: - item, * item, + item
    s = re.sub(r'^[\s]*[-*+]\s+', '', s, flags=re.MULTILINE)
    # Ordered list markers: 1. item
    s = re.sub(r'^[\s]*\d+\.\s+', '', s, flags=re.MULTILINE)
    # Links: [text](url) -> text
    s = re.sub(r'\[([^\]]+)\]\([^)]+\)', r'\1', s)
    # Images: ![alt](url) -> alt
    s = re.sub(r'!\[([^\]]*)\]\([^)]+\)', r'\1', s)
    # Blockquotes: > text
    s = re.sub(r'^>\s+', '', s, flags=re.MULTILINE)
    # Horizontal rules: ---, ***, ___
    s = re.sub(r'^[-*_]{3,}\s*$', '', s, flags=re.MULTILINE)
    return s

# Temp dir for TTS audio files (used temporarily during MP3→PCM conversion)
TTS_DIR = Path(tempfile.mkdtemp(prefix="xiaoai-tts-"))
logger.info("TTS temp dir: %s", TTS_DIR)

# Audio streaming chunk size: ~128ms at 16kHz 16-bit mono
AUDIO_CHUNK_SIZE = 4096


# --- LLM helpers ---

def _provider_headers(provider: Dict[str, str]) -> Dict[str, str]:
    headers = {"Content-Type": "application/json"}
    if provider["api_key"]:
        headers["Authorization"] = f"Bearer {provider['api_key']}"
    return headers


def _provider_url(provider: Dict[str, str]) -> str:
    return f"{provider['base_url'].rstrip('/')}/v1/chat/completions"


def _build_system_prompt(provider: Dict[str, str]) -> str:
    base = provider.get("system_prompt") or DEFAULT_SYSTEM_PROMPT
    now = datetime.now()
    weekday = ['星期一','星期二','星期三','星期四','星期五','星期六','星期日'][now.weekday()]
    date_info = (
        f"【当前时间】现在是 {now.strftime('%Y年%m月%d日 %H:%M')}，{weekday}。"
        f"你必须使用这个日期回答任何关于日期、星期、时间的问题。\n\n"
    )
    return date_info + base


# --- Web search ---

async def execute_web_search(query: str) -> str:
    """Execute web search via Brave Search API."""
    if not BRAVE_API_KEY:
        return "Web search unavailable: no API key configured."
    headers = {
        "Accept": "application/json",
        "X-Subscription-Token": BRAVE_API_KEY,
    }
    params = {"q": query, "count": 5}
    try:
        async with httpx.AsyncClient(timeout=httpx.Timeout(20)) as client:
            resp = await client.get(
                "https://api.search.brave.com/res/v1/web/search",
                params=params, headers=headers,
            )
            resp.raise_for_status()
            data = resp.json()
        results = data.get("web", {}).get("results", [])
        summaries = []
        for r in results[:5]:
            title = r.get("title", "")
            snippet = r.get("description", "")
            summaries.append(f"{title}: {snippet}")
        return "\n".join(summaries) if summaries else "No results found."
    except Exception as exc:
        logger.exception("Web search failed")
        return f"Web search error: {exc}"


async def execute_tool(name: str, arguments: str) -> str:
    """Dispatch a tool call by name."""
    try:
        args = json.loads(arguments)
    except json.JSONDecodeError:
        args = {}
    if name == "web_search":
        query = args.get("query", "")
        logger.info("Tool call: web_search(%s)", query)
        result = await execute_web_search(query)
        logger.info("Search result: %s", result[:200])
        return result
    return f"Unknown tool: {name}"


# --- Streaming chat completion ---

async def stream_chat_completion(
    provider: Dict[str, str],
    messages: List[Dict],
    tools: Optional[list] = None,
    tool_calls_out: Optional[list] = None,
) -> AsyncGenerator[str, None]:
    """Stream chat completion from a specific provider. Yields content tokens.

    If tools is provided, also detects tool calls and appends them to
    tool_calls_out (list of {id, name, arguments} dicts).
    """
    payload: Dict[str, Any] = {
        "model": provider["model"],
        "messages": messages,
        "stream": True,
    }
    if provider.get("user"):
        payload["user"] = provider["user"]
    if tools:
        payload["tools"] = tools
        payload["tool_choice"] = "auto"
    timeout = httpx.Timeout(None)
    accumulated_tool_calls: Dict[int, Dict[str, str]] = {}
    client_kwargs: Dict[str, Any] = {"timeout": timeout}
    if provider.get("no_proxy"):
        client_kwargs["proxy"] = None
    async with httpx.AsyncClient(**client_kwargs) as client:
        async with client.stream(
            "POST", _provider_url(provider),
            headers=_provider_headers(provider), json=payload,
        ) as resp:
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
                # Content tokens
                token = delta.get("content")
                if token:
                    yield token
                # Tool call chunks
                if delta.get("tool_calls") and tool_calls_out is not None:
                    for tc in delta["tool_calls"]:
                        idx = tc.get("index", 0)
                        if idx not in accumulated_tool_calls:
                            accumulated_tool_calls[idx] = {
                                "id": "", "name": "", "arguments": "",
                            }
                        if tc.get("id"):
                            accumulated_tool_calls[idx]["id"] = tc["id"]
                        fn = tc.get("function", {})
                        if fn.get("name"):
                            accumulated_tool_calls[idx]["name"] = fn["name"]
                        if fn.get("arguments"):
                            accumulated_tool_calls[idx]["arguments"] += fn["arguments"]
    # Copy accumulated tool calls to output list
    if tool_calls_out is not None:
        for idx in sorted(accumulated_tool_calls):
            tool_calls_out.append(accumulated_tool_calls[idx])


# --- WebSocket / RPC / Audio streaming ---

async def send_json(websocket, payload: Dict[str, Any]) -> None:
    await websocket.send(json.dumps(payload, ensure_ascii=True))


async def rpc_call(
    websocket, command: str, payload, pending_rpcs: Dict[str, asyncio.Future],
    timeout_secs: float = 10,
) -> Dict[str, Any]:
    """Send a generic RPC request and await the device response."""
    request_id = str(uuid.uuid4())
    future: asyncio.Future[Dict[str, Any]] = asyncio.get_running_loop().create_future()
    pending_rpcs[request_id] = future

    request = {
        "Request": {
            "id": request_id,
            "command": command,
            "payload": payload,
        }
    }
    await websocket.send(json.dumps(request))
    logger.info("RPC %s: %s", command, str(payload)[:120] if payload else "(none)")

    try:
        return await asyncio.wait_for(future, timeout=timeout_secs)
    except asyncio.TimeoutError:
        logger.warning("RPC timeout after %ss: %s", timeout_secs, command)
        return {"error": "timeout"}
    finally:
        pending_rpcs.pop(request_id, None)


async def run_shell_on_device(
    websocket, script: str, pending_rpcs: Dict[str, asyncio.Future],
    timeout_secs: float = 120,
) -> Dict[str, Any]:
    """Send a shell RPC request and await the device response."""
    return await rpc_call(websocket, "run_shell", script, pending_rpcs, timeout_secs)


async def send_audio_stream(websocket, pcm_bytes: bytes) -> None:
    """Send raw PCM audio to device via WebSocket as Stream messages."""
    for i in range(0, len(pcm_bytes), AUDIO_CHUNK_SIZE):
        chunk = pcm_bytes[i:i + AUDIO_CHUNK_SIZE]
        stream = json.dumps({
            "id": str(uuid.uuid4()),
            "tag": "play",
            "bytes": list(chunk),
        }).encode()
        await websocket.send(stream)


async def tts_generate(text: str) -> Optional[bytes]:
    """Generate TTS audio, return raw PCM bytes (S16LE, 16kHz, mono)."""
    mp3_path = TTS_DIR / f"{uuid.uuid4().hex}.mp3"
    try:
        communicate = edge_tts.Communicate(text, TTS_VOICE)
        await communicate.save(str(mp3_path))
        # Convert MP3 → raw PCM
        proc = await asyncio.create_subprocess_exec(
            "/opt/homebrew/bin/ffmpeg", "-i", str(mp3_path),
            "-f", "s16le", "-ar", "16000", "-ac", "1", "-",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.DEVNULL,
        )
        stdout, _ = await proc.communicate()
        return stdout if proc.returncode == 0 else None
    except Exception as exc:
        logger.error("TTS failed: %s", exc)
        return None
    finally:
        mp3_path.unlink(missing_ok=True)


async def playback_worker(
    websocket, queue: asyncio.Queue, pending_rpcs: Dict[str, asyncio.Future],
) -> None:
    """Stream PCM audio to device via WebSocket.

    Lazily calls start_play on first audio chunk, so no device RPC is made
    if no audio is ever enqueued (e.g. claim denied before any TTS).
    """
    started = False
    try:
        while True:
            pcm_bytes = await queue.get()
            if pcm_bytes is None:  # sentinel: no more items
                queue.task_done()
                break
            if not started:
                await rpc_call(websocket, "start_play", None, pending_rpcs)
                started = True
            await send_audio_stream(websocket, pcm_bytes)
            queue.task_done()
    finally:
        if started:
            await rpc_call(websocket, "stop_play", None, pending_rpcs)


# --- Answer deduplication: ensures only one provider speaks per query ---

_answered_lock = asyncio.Lock()
_answered: Dict[str, bool] = {}


async def claim_answer(query_id: str) -> bool:
    """Atomically claim the right to speak for a query. Returns True if granted."""
    async with _answered_lock:
        if _answered.get(query_id):
            return False
        _answered[query_id] = True
        return True


def release_answer(query_id: str) -> None:
    """Remove a query_id from the answered set (cleanup)."""
    _answered.pop(query_id, None)


# --- Core conversation handler ---

async def stream_response_with_tts(
    websocket, provider: Dict[str, str], messages: List[Dict],
    client_addr: str, pending_rpcs: Dict[str, asyncio.Future],
    query_id: str = "", cancel_event: Optional[asyncio.Event] = None,
) -> Optional[str]:
    """Stream LLM response with tool calling, think-block stripping, and TTS.

    Returns the full assistant response text, or None on error.
    Modifies messages in-place (appends assistant/tool messages).
    TTS audio is generated during streaming and queued for sequential playback.
    """
    use_tools = TOOLS if BRAVE_API_KEY else None

    # Playback queue: TTS generation pushes PCM bytes, worker streams to device
    pb_queue: asyncio.Queue[Optional[bytes]] = asyncio.Queue()
    worker = asyncio.create_task(playback_worker(websocket, pb_queue, pending_rpcs))

    try:
        result = await _stream_and_enqueue_tts(
            websocket, provider, messages, client_addr, use_tools, pb_queue,
            query_id, cancel_event,
        )
    except BaseException:
        # On error/timeout/interrupt: discard queued audio so playback stops immediately
        while not pb_queue.empty():
            try:
                pb_queue.get_nowait()
            except asyncio.QueueEmpty:
                break
        raise
    finally:
        # Signal worker to stop and wait for remaining audio (if any) to finish
        await pb_queue.put(None)
        await worker

    return result


async def _stream_and_enqueue_tts(
    websocket, provider: Dict[str, str], messages: List[Dict],
    client_addr: str, use_tools: Optional[list],
    pb_queue: asyncio.Queue, query_id: str = "",
    cancel_event: Optional[asyncio.Event] = None,
) -> Optional[str]:
    """Inner streaming loop: generates TTS and enqueues PCM bytes for playback.

    Applies two-tier timeout:
    - LLM_FIRST_TOKEN_TIMEOUT: max wait for the first token from the provider.
    - LLM_FULL_RESPONSE_TIMEOUT: max time from first token to completion.
    Raises asyncio.TimeoutError if either is exceeded.
    """
    for _round in range(MAX_TOOL_ROUNDS):
        assistant_text: List[str] = []
        sentence_buffer = ""
        in_think_block = False
        raw_buffer = ""
        tool_calls_out: List[Dict] = []
        first_token_received = False
        deadline = asyncio.get_event_loop().time() + LLM_FIRST_TOKEN_TIMEOUT

        token_iter = stream_chat_completion(
            provider, messages,
            tools=use_tools, tool_calls_out=tool_calls_out,
        ).__aiter__()

        try:
            while True:
                # Check for interrupt from wake-up word
                if cancel_event and cancel_event.is_set():
                    logger.info("Generation interrupted by wake word")
                    return "".join(assistant_text) if assistant_text else None

                remaining_time = deadline - asyncio.get_event_loop().time()
                if remaining_time <= 0:
                    kind = "full response" if first_token_received else "first token"
                    raise asyncio.TimeoutError(f"{kind} timeout")
                try:
                    token = await asyncio.wait_for(
                        token_iter.__anext__(), timeout=remaining_time,
                    )
                except StopAsyncIteration:
                    break

                if not first_token_received:
                    first_token_received = True
                    # Claim answer on first token — block other providers
                    if query_id:
                        if not await claim_answer(query_id):
                            logger.info("Query %s already answered, aborting", query_id)
                            return None
                    # Switch to full-response deadline
                    deadline = asyncio.get_event_loop().time() + LLM_FULL_RESPONSE_TIMEOUT
                    logger.info("First token from %s", provider["name"])

                assistant_text.append(token)
                raw_buffer += token

                # Strip <think>...</think> blocks (may span many tokens)
                while True:
                    if in_think_block:
                        close_idx = raw_buffer.find("</think>")
                        if close_idx != -1:
                            raw_buffer = raw_buffer[close_idx + len("</think>"):]
                            in_think_block = False
                        else:
                            raw_buffer = ""
                            break
                    else:
                        open_idx = raw_buffer.find("<think>")
                        if open_idx != -1:
                            sentence_buffer += raw_buffer[:open_idx]
                            raw_buffer = raw_buffer[open_idx + len("<think>"):]
                            in_think_block = True
                        else:
                            safe = max(0, len(raw_buffer) - 6)
                            sentence_buffer += raw_buffer[:safe]
                            raw_buffer = raw_buffer[safe:]
                            break

                # TTS sentence chunking — split at sentence-ending punctuation
                while True:
                    m = SENTENCE_ENDINGS.search(sentence_buffer)
                    if not m:
                        break
                    # Split at the end of the punctuation mark
                    end_pos = m.end()
                    sentence = sentence_buffer[:end_pos].strip()
                    sentence_buffer = sentence_buffer[end_pos:]
                    if sentence and len(sentence) >= MIN_SENTENCE_CHARS:
                        sentence = strip_markdown(sentence)
                        logger.info("TTS sentence: %s", sentence)
                        pcm = await tts_generate(sentence)
                        if pcm:
                            await pb_queue.put(pcm)
                    elif sentence:
                        # Too short — restore to buffer and wait for more content
                        # IMPORTANT: Break to avoid infinite loop on short sentences like "1."
                        sentence_buffer = sentence + sentence_buffer
                        break
        finally:
            await token_iter.aclose()

        if tool_calls_out:
            logger.info("Tool calls requested: %s", [tc["name"] for tc in tool_calls_out])
            assistant_msg: Dict[str, Any] = {"role": "assistant", "content": None}
            assistant_msg["tool_calls"] = [
                {
                    "id": tc["id"],
                    "type": "function",
                    "function": {"name": tc["name"], "arguments": tc["arguments"]},
                }
                for tc in tool_calls_out
            ]
            messages.append(assistant_msg)
            for tc in tool_calls_out:
                result = await execute_tool(tc["name"], tc["arguments"])
                log_conversation_turn(
                    "tool", f"[{tc['name']}] {result}",
                    client=client_addr, provider=provider["name"],
                )
                messages.append({
                    "role": "tool",
                    "tool_call_id": tc["id"],
                    "content": result,
                })
            use_tools = None
            continue

        # No tool calls — flush remaining text
        if not in_think_block and raw_buffer:
            sentence_buffer += raw_buffer
        remaining = sentence_buffer.strip()
        if remaining:
            remaining = strip_markdown(remaining)
            logger.info("TTS final: %s", remaining)
            pcm = await tts_generate(remaining)
            if pcm:
                await pb_queue.put(pcm)

        full_response = "".join(assistant_text)
        return full_response

    return "".join(assistant_text) if assistant_text else None


async def handle_connection(websocket) -> None:
    session_messages: List[Dict] = []
    pending_rpcs: Dict[str, asyncio.Future] = {}
    user_queue: asyncio.Queue[Dict] = asyncio.Queue()
    # Cancel event: set by ws_reader on interrupt, checked by streaming loop
    cancel_event = asyncio.Event()
    logger.info("Client connected: %s", websocket.remote_address)

    async def ws_reader():
        """Read WebSocket messages, dispatch RPC responses, queue user messages."""
        try:
            async for raw_message in websocket:
                try:
                    message = json.loads(raw_message)
                except json.JSONDecodeError:
                    await send_json(websocket, {"type": "error", "error": "invalid_json"})
                    continue

                logger.debug("WS recv: %s", str(raw_message)[:200])

                # RPC response from device — resolve the pending future
                if "Response" in message:
                    resp = message["Response"]
                    rid = resp.get("id", "")
                    fut = pending_rpcs.pop(rid, None)
                    if fut and not fut.done():
                        fut.set_result(resp)
                    else:
                        logger.debug("Unmatched RPC response id=%s", rid)
                    continue

                # Interrupt: cancel current LLM generation immediately
                if message.get("type") == "interrupt":
                    logger.info("Interrupt received — cancelling current generation")
                    cancel_event.set()
                    continue

                # Everything else goes to the user message queue
                await user_queue.put(message)
        except websockets.ConnectionClosed:
            pass
        finally:
            # Unblock the processor if it's waiting
            await user_queue.put({"type": "_disconnect"})

    reader_task = asyncio.create_task(ws_reader())

    try:
        while True:
            message = await user_queue.get()
            msg_type = message.get("type")

            if msg_type == "_disconnect":
                break

            if msg_type != "user_input":
                logger.debug("Ignoring msg type: %s", msg_type)
                continue

            text = (message.get("text") or "").strip()
            if not text:
                await send_json(websocket, {"type": "error", "error": "empty_text"})
                continue

            logger.info("User said: %s", text)
            client_addr = str(websocket.remote_address)
            log_conversation_turn("user", text, client=client_addr)

            # Skip smart home device commands — factory firmware handles these
            if is_smart_home_command(text):
                logger.info("Smart home command detected, skipping LLM: %s", text)
                log_conversation_turn(
                    "assistant", "[silent: smart home command]",
                    client=client_addr, provider="filter",
                )
                await send_json(websocket, {"type": "llm_end"})
                continue

            # Session reset: "开启新的对话" etc.
            if is_session_reset_command(text):
                logger.info("Session reset command: %s", text)
                session_messages.clear()
                log_conversation_turn(
                    "assistant", "[session reset]",
                    client=client_addr, provider="filter",
                )
                if not CONTINUOUS_MODE:
                    # Cancel firmware continuous-dialogue mode (mic-on then mic-off)
                    try:
                        cancel_script = (
                            "ubus call pnshelper event_notify "
                            "'{\"src\":3, \"event\":7}' && "
                            "sleep 0.3 && "
                            "ubus call pnshelper event_notify "
                            "'{\"src\":3, \"event\":8}'"
                        )
                        await run_shell_on_device(
                            websocket, cancel_script, pending_rpcs, timeout_secs=5,
                        )
                        logger.info("Cancelled firmware continuous-dialogue mode")
                    except Exception as exc:
                        logger.warning("Failed to cancel continuous mode: %s", exc)
                await send_json(websocket, {"type": "llm_end"})
                continue

            session_messages.append({"role": "user", "content": text})

            # Clear any previous interrupt before starting new generation
            cancel_event.clear()

            # Unique ID for this query — used to prevent duplicate answers
            query_id = uuid.uuid4().hex[:12]

            # Try each provider in precedence order
            response = None
            used_provider = None
            for provider in PROVIDERS:
                if provider.get("no_system_prompt"):
                    messages_for_provider = list(session_messages)
                else:
                    sys_msg = {"role": "system", "content": _build_system_prompt(provider)}
                    # Inject current date/time as a concrete context message
                    # so models that ignore system prompt still get the date
                    now = datetime.now()
                    weekday = ['星期一','星期二','星期三','星期四','星期五','星期六','星期日'][now.weekday()]
                    date_ctx = [
                        {"role": "user", "content": "现在几点了？今天几号？"},
                        {"role": "assistant", "content":
                            f"现在是{now.strftime('%Y年%m月%d日 %H:%M')}，{weekday}。"},
                    ]
                    messages_for_provider = [sys_msg] + date_ctx + session_messages

                try:
                    logger.info("Trying provider: %s (%s)", provider["name"], provider["model"])
                    response = await stream_response_with_tts(
                        websocket, provider, messages_for_provider,
                        client_addr, pending_rpcs, query_id,
                        cancel_event,
                    )
                    if response is not None and response.strip():
                        used_provider = provider
                        session_messages = messages_for_provider[1:]  # strip system msg
                        break
                    else:
                        log_conversation_turn(
                            "assistant", "(empty)",
                            client=client_addr,
                            provider=provider["name"],
                            model=provider["model"],
                        )
                        logger.warning(
                            "Provider '%s' returned empty response — trying next",
                            provider["name"],
                        )
                        continue
                except asyncio.TimeoutError as te:
                    log_conversation_turn(
                        "assistant", f"(timeout: {te})",
                        client=client_addr,
                        provider=provider["name"],
                        model=provider["model"],
                    )
                    logger.warning(
                        "Provider '%s' timed out: %s",
                        provider["name"], te,
                    )
                    # If this provider already spoke, don't try others
                    if _answered.get(query_id):
                        logger.info("Provider '%s' already spoke, not trying fallback", provider["name"])
                        break
                    continue
                except Exception as exc:
                    log_conversation_turn(
                        "assistant", f"(error: {exc})",
                        client=client_addr,
                        provider=provider["name"],
                        model=provider["model"],
                    )
                    logger.warning(
                        "Provider '%s' failed: %s — trying next",
                        provider["name"], exc,
                    )
                    # If this provider already spoke, don't try others
                    if _answered.get(query_id):
                        logger.info("Provider '%s' already spoke, not trying fallback", provider["name"])
                        break
                    continue

            if response is not None and used_provider is not None:
                if response.strip():
                    session_messages.append({"role": "assistant", "content": response})
                    log_conversation_turn(
                        "assistant", response,
                        client=client_addr,
                        provider=used_provider["name"],
                        model=used_provider["model"],
                    )
                    logger.info(
                        "[%s] Full response: %s",
                        used_provider["name"], response[:200],
                    )
                    # Play "response complete" chime on device
                    await run_shell_on_device(
                        websocket,
                        "aplay -D notify /data/open-xiaoai/sounds/multirounds_tone.wav 2>/dev/null",
                        pending_rpcs,
                        timeout_secs=5,
                    )
            elif _answered.get(query_id):
                # A provider already spoke (partial TTS) but timed out before
                # returning a full response. Don't play error or pop history.
                logger.warning("Provider spoke partially before timeout for: %s", text)
            else:
                logger.error("All providers failed for: %s", text)
                # Play fallback message via audio stream
                fallback_pcm = await tts_generate("大脑宕机了")
                if fallback_pcm:
                    await rpc_call(websocket, "start_play", None, pending_rpcs)
                    await send_audio_stream(websocket, fallback_pcm)
                    await rpc_call(websocket, "stop_play", None, pending_rpcs)
                session_messages.pop()  # remove the unanswered user message
                await send_json(websocket, {"type": "error", "error": "all_providers_failed"})

            # Cleanup answer lock for this query
            release_answer(query_id)

            await send_json(websocket, {"type": "llm_end"})

    except websockets.ConnectionClosed:
        logger.info("Client disconnected: %s", websocket.remote_address)
    finally:
        reader_task.cancel()
        # Cancel any pending RPCs
        for fut in pending_rpcs.values():
            if not fut.done():
                fut.cancel()
        pending_rpcs.clear()


async def main() -> None:
    logger.info("Starting AI-Brain WS on %s:%s", WS_HOST, WS_PORT)
    async with websockets.serve(handle_connection, WS_HOST, WS_PORT):
        await asyncio.Future()


if __name__ == "__main__":
    asyncio.run(main())
