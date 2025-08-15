# secure-ws (example)

A minimal WebSocket server for open-xiaoai that keeps on-device STT/TTS by default and supports ws:// and wss:// with optional mutual TLS (mTLS).

## What it does
- Accepts a single device connection over WebSocket.
- Uses the same JSON/Binary protocol as `examples/migpt`.
- Keeps defaults: ASR and TTS run on the device; the server only routes messages.
- Supports TLS and optional client-auth (mTLS) configured via environment variables at startup.

## Build and run

ws (no TLS):
```bash
cargo run -p open_xiaoai_secure_server --bin secure-ws
```

wss (TLS):
```bash
export SECURE_WS_ENABLE_TLS=true
export SECURE_WS_CERT_CHAIN_PEM="$(cat /path/to/server.crt)"
export SECURE_WS_PRIV_KEY_PEM="$(cat /path/to/server.key)"
cargo run -p open_xiaoai_secure_server --bin secure-ws
```

wss + mTLS (require client certificate):
```bash
export SECURE_WS_ENABLE_TLS=true
export SECURE_WS_REQUIRE_CLIENT_AUTH=true
export SECURE_WS_CERT_CHAIN_PEM="$(cat /path/to/server.crt)"
export SECURE_WS_PRIV_KEY_PEM="$(cat /path/to/server.key)"
export SECURE_WS_CLIENT_CA_PEM="$(cat /path/to/client-ca.crt)"
cargo run -p open_xiaoai_secure_server --bin secure-ws
```

Notes
- All PEMs must be plaintext PEM (PKCS#8 or RSA private key). For multi-cert chains, concatenate full chain in `server.crt`.
- Server listens on `0.0.0.0:4399`.
- No HTTP endpoints are exposed.

## Point the client
Set the client to connect to your server URL (ws or wss):
- Using CLI arg (see `packages/client-rust/src/bin/client.rs`): `client wss://your-host:4399`
- Or via the device setup described in `packages/client-rust/README.md`.

## Default behavior
- ASR: Device produces instruction events; server does not run server-side ASR by default.
- TTS/Playback: Device handles TTS or local playback; server may optionally stream bytes with tag `play`.

## Troubleshooting
- TLS handshake fails: verify `server.crt` and `server.key` match and include full chain.
- mTLS failure: ensure the client presents a cert issued by `SECURE_WS_CLIENT_CA_PEM` and the device trusts your server.
- Port in use: change the port in `src/server.rs` and rebuild.
