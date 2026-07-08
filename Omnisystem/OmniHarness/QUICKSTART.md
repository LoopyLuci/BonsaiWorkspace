# OmniHarness — Quick Start

You can be talking to any AI model in under 5 minutes.

---

## Option A — Terminal CLI (fastest)

### 1. Install

```powershell
cd OmniHarness/orchestrator
pip install -e ".[all]"
```

### 2. Set your key(s)

Copy `.env.example` → `.env`, then fill in at least one key:

```
ANTHROPIC_API_KEY=sk-ant-...
```

Or for a fully **free, local** setup — no key needed:
```
OLLAMA_ENABLED=1
```
(Requires [Ollama](https://ollama.com) running with `ollama pull llama3.2`)

### 3. Chat

```powershell
omniharness chat "What is the capital of France?"
```

That's it. You're talking to Claude (or whatever key you set).

---

## Common CLI commands

```powershell
# Chat with default model
omniharness chat "Explain neural networks simply"

# Use a specific model
omniharness chat "Write a haiku" --model openai/gpt-4o
omniharness chat "Hello" --model gemini/gemini-2.0-flash
omniharness chat "Tell me a joke" --model groq/llama-3.3-70b-versatile
omniharness chat "Hello" --model ollama/llama3.2

# Stream token by token (default)
omniharness chat "Write me a short story" --stream

# Run an autonomous agent
omniharness agent "Search the web for the latest Rust releases and summarize"

# List all available models
omniharness models

# Check which providers are healthy
omniharness health

# Store and retrieve memory
omniharness remember "My favorite language is Titan"
omniharness recall "programming preferences"

# Start the REST server
omniharness serve
```

---

## Option B — REST API

### Start the server

```powershell
cd OmniHarness
.\start.ps1
```

Or manually:
```powershell
cd orchestrator
uvicorn omniharness.server:app --port 8080
```

### Call it

```powershell
# Chat
curl http://localhost:8080/api/chat `
  -Method POST `
  -ContentType "application/json" `
  -Body '{"model_id":"claude-sonnet-4-6","messages":[{"role":"user","content":"Hello!"}]}'

# Stream (SSE)
curl http://localhost:8080/api/chat/stream `
  -Method POST `
  -ContentType "application/json" `
  -Body '{"model_id":"gpt-4o","messages":[{"role":"user","content":"Count to 10"}],"stream":true}'

# Run agent
curl http://localhost:8080/api/agent/run `
  -Method POST `
  -ContentType "application/json" `
  -Body '{"objective":"What files are in the current directory?","model_id":"claude-sonnet-4-6"}'

# List all models
curl http://localhost:8080/api/models

# Health check
curl http://localhost:8080/api/health
```

Interactive Swagger UI: http://localhost:8080/docs

---

## Option C — GUI

```powershell
cd OmniHarness/gui
npm install
npm run dev
# Open http://localhost:3000
```

The GUI gives you a full chat interface with model switching, memory visualization, tool panel, and ReAct step inspection.

---

## Option D — One command (everything at once)

```powershell
cd OmniHarness
.\start.ps1
```

Starts the orchestrator + GUI automatically. Add `--no-gui` for API-only.

---

## Using from Titan (Omnisystem native code)

```titan
use OmniHarness::HarnessCore

fn main() {
    let harness = OmniHarness::create()
    let resp = harness.chat("What is the meaning of life?".to_string())?
    println(resp.content)
}
```

---

## Supported models (examples)

| What you type | Calls |
|---|---|
| `claude-sonnet-4-6` | Anthropic Claude Sonnet 4.6 |
| `anthropic/claude-opus-4-8` | Anthropic Claude Opus 4.8 |
| `gpt-4o` | OpenAI GPT-4o |
| `openai/o3-mini` | OpenAI o3-mini |
| `gemini/gemini-2.0-flash` | Google Gemini 2.0 Flash |
| `groq/llama-3.3-70b-versatile` | Llama 3.3 70B via Groq (fast) |
| `mistral/mistral-large-latest` | Mistral Large |
| `together/meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo` | Llama 3.1 405B via Together |
| `fireworks/accounts/fireworks/models/firefunction-v2` | FireFunction v2 |
| `openrouter/anthropic/claude-3-5-sonnet` | Any model via OpenRouter |
| `ollama/llama3.2` | Local Llama 3.2 (no key needed) |
| `ollama/mistral` | Local Mistral (no key needed) |
| `ollama/codellama` | Local CodeLlama (no key needed) |

Provider is inferred automatically — you rarely need the prefix.
