# Configuration Management

## 🔒 Secure Configuration Setup

This project uses a template-based configuration system to keep sensitive information out of git while maintaining a working configuration during compilation.

### Quick Setup

1. **Copy environment template:**
   ```bash
   cp .env.example .env
   ```

2. **Edit `.env` with your actual values:**
   ```bash
   # Edit these with your real API keys and URLs
   OPENAI_API_KEY=your_real_api_key_here
   OPENAI_BASE_URL=https://your-llm-provider.com/v1
   # ... etc
   ```

3. **Generate config.json:**
   ```bash
   ./generate-config.sh
   ```

4. **Build and run:**
   ```bash
   cargo build --release --bin client
   ./target/release/client
   ```

### Files Structure

| File | Tracked in Git | Purpose |
|------|---------------|---------|
| `config.template.json` | ✅ **Yes** | Template with `${VAR}` placeholders |
| `.env.example` | ✅ **Yes** | Example environment variables |
| `.env` | ❌ **No** | Your actual secrets (auto-ignored) |
| `config.json` | ❌ **No** | Generated config with real values |
| `generate-config.sh` | ✅ **Yes** | Script to generate config.json |

### Environment Variables

| Variable | Example | Description |
|----------|---------|-------------|
| `OPENAI_API_KEY` | `sk-abc123...` | Your LLM API key |
| `OPENAI_BASE_URL` | `https://api.302.ai/v1` | LLM service base URL |
| `OPENAI_MODEL` | `gpt-4o-mini` | Model name to use |
| `SERVER_PROXY_URL` | `http://localhost:4399` | Server proxy URL for proxy mode |

### Benefits

- ✅ **Security**: No secrets in git history
- ✅ **Flexibility**: Easy to switch between environments
- ✅ **Team-friendly**: Everyone uses same template
- ✅ **CI/CD ready**: Environment variables work in pipelines
- ✅ **Local development**: Quick setup with `.env` file
