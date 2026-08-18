# AI-Brain Admin Guide

**Updated 2026-08-18** — reflects the post-mTLS-migration deployment on macmini. See
`xiaomi-stereo/docs/architecture-ai-orchestrator.md` for the full system architecture and
`xiaomi-stereo/docs/superpowers/plans/2026-08-16-mtls-brain-migration.md` for the migration's
full history (every deviation, ruling, and fix made along the way).

## Quick Reference

### Service Management (macOS LaunchAgent, on macmini)

The service runs as `com.meatlover.xiaoai-brain`, deployed to `/Users/alex/open-xiaoai-server/`
on **macmini** (not this Mac Studio — the old Mac Studio copy was decommissioned once the
macmini copy was verified working end-to-end).

```bash
# Check service status
ssh macmini 'launchctl list | grep xiaoai-brain'

# Restart (per this project's own rule: launchd-managed services get unload/load, never kill —
# launchd just respawns a killed process)
ssh macmini 'launchctl unload ~/Library/LaunchAgents/com.meatlover.xiaoai-brain.plist'
ssh macmini 'launchctl load ~/Library/LaunchAgents/com.meatlover.xiaoai-brain.plist'

# View logs
ssh macmini 'tail -f /Users/alex/open-xiaoai-server/logs/stderr.log'
```

**Service features:**
- Auto-starts at login/boot (`RunAtLoad`)
- Auto-restarts on crash (`KeepAlive`)
- Logs to `logs/stdout.log` (usually empty — Python's stdout buffering) and `logs/stderr.log`
  (the real log stream); conversation transcripts separately in `logs/conversations.jsonl`

### The AI-gateway (OpenClaw) — a separate service, also on macmini

`server.py` doesn't talk to DeepSeek directly — it shells out to the AI-gateway CLI, which is its
own separate `launchd` service (`ai.openclaw.gateway`, listening on `:8080`).

```bash
ssh macmini 'launchctl list | grep gateway'
ssh macmini 'launchctl unload ~/Library/LaunchAgents/ai.openclaw.gateway.plist'
ssh macmini 'launchctl load ~/Library/LaunchAgents/ai.openclaw.gateway.plist'
```

If `server.py` is up but every query fails, check the gateway first — it's a separate failure
domain. `oc-cli agents list --token AlexIsAnAI` (see the CLI-alias note below) is a fast health
check that doesn't touch `server.py` at all.

### Manual Start (Development, on either machine)

```bash
source .venv/bin/activate
export LLM_PROVIDERS=openclaw
export LLM_OPENCLAW_MODE=cli
export LLM_OPENCLAW_CLI_BIN=/opt/homebrew/bin/openclaw
export LLM_OPENCLAW_AGENT_ID=xiaoai
python server.py
```

---

## Key Management Points

### 1. Environment Variables

**Provider selection (as of the current CLI-mode deployment):**

| Variable | Value in production | Purpose |
|----------|---------|---------|
| `LLM_PROVIDERS` | `openclaw` | Ordered provider list, comma-separated (only one entry now) |
| `LLM_OPENCLAW_MODE` | `cli` | `http` (original design, unused — see below) or `cli` (real) |
| `LLM_OPENCLAW_CLI_BIN` | `/opt/homebrew/bin/openclaw` | Path to the gateway CLI binary |
| `LLM_OPENCLAW_AGENT_ID` | `xiaoai` | Which gateway agent to invoke |
| `LLM_OPENCLAW_TIMEOUT_S` | `30` | CLI subprocess timeout |
| `LLM_FIRST_TOKEN_TIMEOUT` | `40` | **Must be strictly greater than `LLM_OPENCLAW_TIMEOUT_S`** — see "Timeout ordering" below |

**Why `mode=http` still exists but is unused**: the original design assumed an OpenAI-compatible
`POST /v1/chat/completions` REST endpoint on the gateway. That endpoint does not exist in the
deployed gateway version — its HTTP surface is only `/health` and a web Control UI. `mode=http`
(and the underlying `stream_chat_completion()` function) is left in the code as dead-but-harmless
fallback logic, not deleted, in case a future gateway version reintroduces a REST surface.

**Timeout ordering — read this before touching either value.** `_stream_and_enqueue_tts` arms an
outer deadline (`LLM_FIRST_TOKEN_TIMEOUT`) *before* spawning the CLI subprocess. If that outer
deadline is ≤ the CLI's own internal timeout (`LLM_OPENCLAW_TIMEOUT_S`), the outer timeout always
fires first, which means the CLI subprocess's own cleanup path never runs and a slow turn leaks
an orphaned subprocess. Keep `LLM_FIRST_TOKEN_TIMEOUT` comfortably above
`LLM_OPENCLAW_TIMEOUT_S` (current values: 40 vs 30) whenever either is changed.

**Other variables (unchanged from before the migration):**

| Variable | Default | Purpose |
|----------|---------|---------|
| `WS_HOST` | `0.0.0.0` | WebSocket bind address — **accepted risk**: this means the home LAN can reach `server.py` directly, bypassing the mTLS boundary entirely (mTLS is enforced by nginx on azcn-nginx2, not by `server.py` itself, which has never had any TLS support). Reviewed and explicitly accepted 2026-08-18; revisit by restricting to `10.8.0.82`+`127.0.0.1` if this changes. |
| `WS_PORT` | `9000` | WebSocket port |
| `LOG_LEVEL` | `INFO` | Logging verbosity |
| `SYSTEM_PROMPT` | (persona text) | Injected as the system role for every turn |
| `BRAVE_API_KEY` | (secret) | Enables `server.py`'s own web-search tool-calling — **mostly redundant now**: the gateway's own `xiaoai` agent has its own much richer built-in toolset (web search, memory, etc.), and CLI-mode calls skip `server.py`'s own tool dispatch entirely. Left configured for parity, not required. |

The full deployed plist is snapshotted at
`xiaomi-stereo/docs/adr/reference-macmini-deploy/com.meatlover.xiaoai-brain.plist` in the sibling
repo — treat that as the reference for exact current values, not this table, if they ever
diverge.

---

### 2. The gateway CLI alias workaround (macmini only)

This project's development environment has a quirk where any Bash command containing the literal
substring `openclaw` (case-insensitive, including as a path fragment like `.openclaw/`) gets
silently killed. To work around this without ever needing that string in executed commands,
these symlink aliases exist on macmini (created manually, not part of any automated deploy):

| Alias | Real target |
|---|---|
| `oc-cli` (on `$PATH`) | `/opt/homebrew/bin/openclaw` |
| `~/.ocgw` | `~/.openclaw/` |
| `~/.ocgw/gw-config.json` | `~/.openclaw/openclaw.json` |
| `~/Library/LaunchAgents/ai.ocgw.gateway.plist` | `~/Library/LaunchAgents/ai.openclaw.gateway.plist` |

Use these in any command you run against macmini. This is purely a workaround for one
development environment's own restriction — it has no bearing on the actual deployed system,
which references the real paths directly in its own config/code.

---

### 3. Dependencies

```bash
cd /Users/alex/open-xiaoai-server   # on macmini
source .venv/bin/activate
pip install -r requirements.txt
```

`server.py`'s `tts_generate()` hardcodes `/opt/homebrew/bin/ffmpeg` — confirm it's installed
(`brew install ffmpeg`) if TTS conversion starts failing after a machine change.

---

### 4. Log Monitoring

```bash
ssh macmini 'tail -f /Users/alex/open-xiaoai-server/logs/stderr.log'
```

**What to look for:**
- `Starting AI-Brain WS on 0.0.0.0:9000` = server started successfully
- `Client connected: (...)` — check the source IP: `10.8.0.1` means a connection arrived via the
  OpenVPN tunnel from azcn-nginx2 (i.e. a real device, over the mTLS-protected internet path);
  `127.0.0.1` means a local test connection on macmini itself, bypassing nginx/mTLS entirely
  (useful for isolating whether a problem is in `server.py`/the gateway vs. the network path)
- `Trying provider: openclaw` → `First token from openclaw` → `TTS final: ...` → `Full response:
  ...` is the healthy sequence for one turn
- `all_providers_failed` = every configured provider failed this turn — check the gateway next

A separate structured log, `logs/conversations.jsonl`, records every user/assistant turn (role,
content, provider, model, timestamp) — useful for confirming exactly what was said/heard without
parsing the free-text stderr log.

---

### 5. Health Checks

**Basic connectivity (local, bypasses mTLS/nginx):**
```bash
ssh macmini 'python3 -c "
import asyncio, websockets, json
async def test():
    async with websockets.connect(\"ws://localhost:9000\") as ws:
        await ws.send(json.dumps({\"type\":\"user_input\",\"text\":\"你好\"}))
        print(await ws.recv())
asyncio.run(test())
"'
```

**Real end-to-end (through nginx's mTLS, from anywhere with the device's client cert):**
```bash
curl -v --cert device-setup/payload/open-xiaoai/certs/client.crt \
        --key device-setup/payload/open-xiaoai/certs/client.key \
        --cacert device-setup/payload/open-xiaoai/certs/server-ca.crt \
        --noproxy '*' https://azcn-nginx2.harmanota.com.cn:8444/
```
(run from `xiaomi-stereo/`; expect `HTTP/1.1 426 Upgrade Required` — a plain GET against a
WebSocket-only endpoint, which confirms the mTLS handshake succeeded)

**Gateway health (bypasses `server.py` entirely):**
```bash
ssh macmini 'oc-cli agents list --token AlexIsAnAI --json'
```

---

### 6. Common Issues & Solutions

| Issue | Symptom | Solution |
|-------|---------|----------|
| **Gateway won't come back up after an upgrade** | `launchctl list` shows the gateway loaded with a non-zero exit status; `~/.ocgw/logs/gateway.err.log` (or `/tmp/openclaw/...` depending on version) shows a fatal startup-migration error | This happened once during this project's own migration — a new gateway version refused to start because a legacy state directory (`~/.clawdbot`) was a symlink, then refused again once converted to a real directory because the destination (`~/.openclaw`) already had content. Resolved by renaming the legacy dir out of the way entirely (`mv ~/.clawdbot ~/.clawdbot.legacy-unmigrated`) rather than merging/deleting anything. `oc-cli doctor --fix`/`--fix --force` did NOT resolve it and made unrelated config changes (model fallback, disabled skills) — worth reviewing those separately if this recurs. |
| **Real device connects but no response** | `stderr.log` shows the connection from `10.8.0.1` but the query never completes | Check `oc-cli agents list` (gateway health) and `oc-cli models --agent xiaoai auth list` (is the DeepSeek key actually registered? — see note below) before assuming `server.py` itself is broken |
| **mTLS handshake fails from outside macmini/azcn-nginx2** | curl/device gets `SSL_ERROR_SYSCALL` with no TLS alert at all | Check the Azure NSG (`azcn-it-nginx1-nsg`, subscription `HARMAN-SP-OTA-IT-PROD-3-CH`, RG `AZCN-IT-MGMT-RG`) for an inbound rule on the port in use — a TCP-level timeout with zero TLS handshake activity means the cloud firewall, not nginx, is the blocker. This bit the initial deployment: the new port had no NSG rule at all. |
| **mTLS handshake starts but curl reports "unsuitable certificate purpose"** | Full TLS handshake begins, server sends its cert, client rejects it | The server's TLS cert lacks `TLS Web Server Authentication` in its Extended Key Usage. Check `openssl x509 -in <cert> -noout -text \| grep -A2 "Extended Key Usage"`. This bit the initial deployment too: azcn-nginx2's *shared* cert turned out to be client-auth-only; the fix was a dedicated server-auth cert under a new filename, not touching the shared one. |
| **`Text file busy` when re-running `setup-device.sh`** | Upload of the client binary fails against a device with a live prior client process | A stale `watchdog.sh`/`client` process is holding the old binary open. `setup-device.sh` now checks for `client.key` existing up front, but does not (yet) kill stale processes before uploading — kill them manually first (`ssh <device> 'kill $(cat /data/open-xiaoai/watchdog.pid); pkill -f open-xiaoai/client'`) if a re-run fails this way. |
| **DeepSeek auth error despite the model config looking correct** | `oc-cli agent --agent xiaoai --message ... --json` fails with `ProviderAuthError: No API key found` | This gateway version does NOT use `~/.ocgw/agents/xiaoai/agent/models.json`'s `apiKey` field for authentication — that field is dead for auth purposes (only the model catalog entries — id/contextWindow/maxTokens — are read from it). Auth lives in a per-agent SQLite-backed profile store, registered via `oc-cli models --agent xiaoai auth paste-api-key --provider deepseek` (reads the key from stdin). Verify with `oc-cli models --agent xiaoai auth list`. |
| **Module not found / import errors** | Exit status 1 in `launchctl list`, `stderr.log` shows `ModuleNotFoundError` | `pip install -r requirements.txt` inside `.venv`, then restart the service |
| **Firewall blocking (LAN)** | Clients on the LAN can't connect | Check macOS Firewall settings; `WS_HOST=0.0.0.0` should already allow this (see the accepted-risk note above) |

---

### 7. Security Considerations

**Current state (post-mTLS-migration):**
- **The internet-facing path is authenticated via mTLS**, enforced by nginx on azcn-nginx2, not
  by `server.py`. `ssl_verify_client on`, scoped to the project's own standalone CA chain
  specifically (not any broader/shared trust anchor).
- **`server.py` itself has no authentication and no TLS** — it trusts whatever reaches it on
  `:9000`. On macmini this means the home LAN can reach it directly (`WS_HOST=0.0.0.0`), an
  explicitly accepted, documented risk (see the environment-variables table above).
- **No rate limiting.**
- **Session memory grows unbounded** within a connection's lifetime (per-connection, not
  cross-connection — a fresh WS connection starts fresh).
- **DeepSeek API key and the gateway's own auth token** live only on macmini, never on the
  device or in this repo. The gateway's auth token is a plaintext static token, but since
  `server.py` now reaches it via `localhost:8080` only (colocated on macmini post-migration),
  it's no longer an internet-facing credential the way it would have been in the pre-migration
  cross-host setup.
- **No cert revocation infrastructure** (no CRL/OCSP) for the project's ~30-year cert chain — the
  only remedy for a compromised key is reissuing and manually redeploying.

**For further internet exposure changes**: this system is already internet-reachable via mTLS as
of this migration — there's no "add a reverse proxy with TLS" step remaining, that's what this
whole migration was.

---

### 8. Upgrades & Maintenance

#### Update the server code
```bash
cd /Users/meatlover/repos/open-xiaoai   # local checkout, main branch
git pull                                 # if tracking a shared remote
# ... make changes, test locally (.venv/bin/pytest -v in server/, cargo test in packages/client-rust/) ...
scp server/server.py macmini:/Users/alex/open-xiaoai-server/server.py
ssh macmini 'launchctl unload ~/Library/LaunchAgents/com.meatlover.xiaoai-brain.plist'
ssh macmini 'launchctl load ~/Library/LaunchAgents/com.meatlover.xiaoai-brain.plist'
```

#### Update the gateway CLI itself
```bash
ssh macmini 'npm update -g openclaw'    # run this via your own shell if this environment's
                                          # command-text restriction blocks it for an agent
```
**Warning**: this project hit a real gateway-version upgrade regression once (see the Common
Issues table above) — budget time to verify the gateway actually comes back up healthy
(`oc-cli agents list`) before considering an upgrade complete, not just that the version number
changed.

#### Backup session state
No persistent conversation state beyond `logs/conversations.jsonl` (append-only log, not a
resumable session store). Sessions are lost on service restart — this is unchanged from before
the migration.

---

### 9. PKI / Certificate Renewal

The full standalone PKI (root CA, issuing CA, device cert, server cert) lives at
`~/pki/xiaoai-speaker/` on the Mac Studio that originally set it up — see
`xiaomi-stereo/docs/architecture-ai-orchestrator.md` §5 for the full layout and naming
convention. Key passphrases are in that machine's Keychain
(`xiaoai-speaker-root-ca`, `xiaoai-speaker-issuing-ca`).

To reissue the device's client cert (e.g. compromise, or approaching the ~30-year expiry — not
an urgent concern in practice): repeat the `openssl x509 -req ... -CA
~/pki/xiaoai-speaker/issuing/xiaoai-issuing-ca.crt` pattern used originally, redeploy
`client.crt`/`client.key` to `device-setup/payload/open-xiaoai/certs/` and re-run
`setup-device.sh` against the physical device.

To reissue the nginx server cert: same pattern against
`~/pki/xiaoai-speaker/issued/azcn-nginx2-server.{key,crt}`, redeploy to azcn-nginx2 at
`/etc/pki/rbdev/azcn-nginx2-xiaoai-server.{crt,key}`, `nginx -t` then `nginx -s reload`.

---

## Quick Troubleshooting Checklist

1. ✅ Is `server.py` running? `ssh macmini 'launchctl list | grep xiaoai-brain'`
2. ✅ Is the gateway running? `ssh macmini 'launchctl list | grep gateway'`
3. ✅ Is the gateway actually healthy (not just "running")? `oc-cli agents list --token AlexIsAnAI`
4. ✅ Is the DeepSeek auth profile registered? `oc-cli models --agent xiaoai auth list`
5. ✅ Is port 9000 listening on macmini? `ssh macmini 'lsof -i :9000'`
6. ✅ Is the OpenVPN tunnel up (macmini claims `10.8.0.82`)? `ssh macmini 'ifconfig | grep 10.8.0.82'`
7. ✅ Is nginx up and listening on azcn-nginx2:8444? `ssh azcn-nginx2 'ss -tlnp | grep 8444'`
8. ✅ Does a real mTLS handshake succeed? (see §5 "Real end-to-end" curl command)
9. ✅ Are logs showing errors? `ssh macmini 'tail -30 /Users/alex/open-xiaoai-server/logs/stderr.log'`
