#!/usr/bin/env python3
"""
Unit tests for server.py's CLI-based AI-gateway provider path.
"""
import json
from unittest.mock import AsyncMock, patch

import pytest

from server import stream_chat_completion_cli


@pytest.mark.asyncio
async def test_stream_chat_completion_cli_yields_reply_text():
    provider = {"cli_bin": "openclaw", "agent_id": "xiaoai", "timeout_s": 30.0}
    fake_stdout = json.dumps({
        "status": "ok",
        "result": {"payloads": [{"text": "你好，有事直接说。", "mediaUrl": None}]},
    }).encode()
    mock_proc = AsyncMock()
    mock_proc.communicate.return_value = (fake_stdout, b"")
    mock_proc.returncode = 0
    with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
        tokens = [t async for t in stream_chat_completion_cli(provider, [{"role": "user", "content": "你好"}])]
    assert tokens == ["你好，有事直接说。"]


@pytest.mark.asyncio
async def test_stream_chat_completion_cli_raises_on_status_not_ok():
    provider = {"cli_bin": "openclaw", "agent_id": "xiaoai", "timeout_s": 30.0}
    fake_stdout = json.dumps({"status": "error", "result": {}}).encode()
    mock_proc = AsyncMock()
    mock_proc.communicate.return_value = (fake_stdout, b"")
    mock_proc.returncode = 0
    with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
        with pytest.raises(RuntimeError, match="reported failure"):
            async for _ in stream_chat_completion_cli(provider, [{"role": "user", "content": "hi"}]):
                pass


@pytest.mark.asyncio
async def test_stream_chat_completion_cli_raises_on_nonzero_exit():
    provider = {"cli_bin": "openclaw", "agent_id": "xiaoai", "timeout_s": 30.0}
    mock_proc = AsyncMock()
    mock_proc.communicate.return_value = (b"", b"some CLI error text")
    mock_proc.returncode = 1
    with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
        with pytest.raises(RuntimeError, match="exited 1"):
            async for _ in stream_chat_completion_cli(provider, [{"role": "user", "content": "hi"}]):
                pass
