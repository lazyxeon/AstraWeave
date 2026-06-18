# Environment Variables Reference

**AstraWeave Game Engine Configuration**

This document lists all environment variables used across the AstraWeave codebase.

---

## LLM Integration

### `LOCAL_LLM_API_KEY`
- **Purpose**: API key for OpenAI or other LLM providers
- **Default**: None (LLM features disabled)
- **Example**: `set LOCAL_LLM_API_KEY=sk-proj-...`
- **Used by**: examples/llm_integration, astraweave-llm
- **Security**: ⚠️ **NEVER commit to Git**

### `OLLAMA_URL`
- **Purpose**: Ollama server endpoint
- **Default**: `http://localhost:11434`
- **Example**: `set OLLAMA_URL=http://192.168.1.100:11434`
- **Used by**: astraweave-llm, examples/ollama_probe

### `OLLAMA_MODEL`
- **Purpose**: Default Ollama model name
- **Default**: `phi3:medium`
- **Example**: `set OLLAMA_MODEL=llama3:latest`

---

## Asset Pipeline

### `POLYHAVEN_BASE_URL`
- **Purpose**: Override PolyHaven CDN URL
- **Default**: `https://dl.polyhaven.org`
- **Used by**: tools/astraweave-assets

---

## Debugging

### `RUST_LOG`
- **Purpose**: Logging level (standard Rust)
- **Default**: `info`
- **Examples**:
  - `set RUST_LOG=debug`
  - `set RUST_LOG=astraweave_render=trace`

### `ASTRAWEAVE_USE_LLM`
- **Purpose**: Force enable/disable LLM
- **Default**: Auto-detect
- **Example**: `set ASTRAWEAVE_USE_LLM=false`

---

## Network

### `AW_WS_URL`
- **Purpose**: WebSocket server URL
- **Default**: `ws://127.0.0.1:8788` (dev), `wss://server:8788` (prod)
- **Used by**: net/aw-net-client

---

## Security Best Practices

1. Never commit secrets to Git
2. Use `.env` files for local development (add to .gitignore)
3. Use OS keyring or secret manager for production
4. Rotate API keys quarterly
5. Separate keys for dev/staging/prod

---

**Last Updated:** 2025-11-13
