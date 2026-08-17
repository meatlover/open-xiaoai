#!/usr/bin/env python3
"""
Unit tests for server.py's CLI-based AI-gateway provider path.
"""
import asyncio
import json
from unittest.mock import AsyncMock, MagicMock, patch

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


@pytest.mark.asyncio
async def test_stream_chat_completion_cli_kills_subprocess_on_internal_timeout():
    """Our own asyncio.wait_for(..., timeout=...) firing must still kill and
    reap the subprocess (regression guard for the try/finally refactor)."""
    provider = {"cli_bin": "openclaw", "agent_id": "xiaoai", "timeout_s": 0.01}
    mock_proc = AsyncMock()
    mock_proc.returncode = None

    async def hang_forever(*_a, **_kw):
        await asyncio.sleep(10)

    mock_proc.communicate.side_effect = hang_forever
    mock_proc.kill = MagicMock()
    with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
        with pytest.raises(RuntimeError, match="timed out"):
            async for _ in stream_chat_completion_cli(provider, [{"role": "user", "content": "hi"}]):
                pass

    mock_proc.kill.assert_called_once()
    mock_proc.wait.assert_awaited_once()


@pytest.mark.asyncio
async def test_stream_chat_completion_cli_kills_subprocess_on_external_cancellation():
    """If the *caller* cancels this generator mid-await (e.g. an interrupt
    cancels token_iter.__anext__() from the outside, or closes the generator
    via aclose()), the subprocess must still be killed/reaped rather than
    left orphaned — not just when our own internal timeout fires."""
    provider = {"cli_bin": "openclaw", "agent_id": "xiaoai", "timeout_s": 30.0}
    mock_proc = AsyncMock()
    mock_proc.returncode = None

    async def hang_forever(*_a, **_kw):
        await asyncio.sleep(10)

    mock_proc.communicate.side_effect = hang_forever
    mock_proc.kill = MagicMock()

    with patch("asyncio.create_subprocess_exec", return_value=mock_proc):
        gen = stream_chat_completion_cli(provider, [{"role": "user", "content": "hi"}])
        task = asyncio.ensure_future(gen.__anext__())
        # Let the generator run until it's parked inside proc.communicate().
        await asyncio.sleep(0)
        await asyncio.sleep(0)
        task.cancel()
        with pytest.raises(asyncio.CancelledError):
            await task

    mock_proc.kill.assert_called_once()
    mock_proc.wait.assert_awaited_once()
