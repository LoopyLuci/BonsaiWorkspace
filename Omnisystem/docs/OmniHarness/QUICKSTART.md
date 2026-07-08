# OmniHarness Quick Start

Get from zero to your first AI response in under 5 minutes. This guide covers installation, configuration, and all four ways to interact with OmniHarness: CLI, REST API, GUI, and Titan native code.

---

## Prerequisites

- Python 3.11+
- Rust 1.75+ (for the kernel — `rustup install stable`)
- Node.js 20+ (for the ClojureScript GUI, optional)
- At least one API key (or Ollama for local models)

---

## Step 1 — Install Python Dependencies

```powershell
cd OmniHarness/orchestrator
pip install -r requirements.txt
```

The requirements include: `fastapi`, `uvicorn`, `anthropic`, `openai`, `google-generativeai`, `groq`, `cohere`, `httpx`, `websockets`, `click`, `rich`, `sqlite-utils`, `grpcio`, `protobuf`.

---

## Step 2 — Configure Environment

```powershell
# Copy the example config
cp OmniHarness/.env.example OmniHarness/.env

# Edit with your API keys
notepad OmniHarness/.env
```

Minimum configuration for the most common providers:

```env
# Required for Anthropic Claude
ANTHROPIC_API_KEY=sk-ant-...

# Required for OpenAI
OPENAI_API_KEY=sk-...

# For local models via Ollama (no key needed)
OLLAMA_BASE_URL=http://localhost:11434

# Default model if none specified
OMNIHARNESS_DEFAULT_MODEL=anthropic/claude-sonnet-4-5
```

See [CONFIGURATION.md](CONFIGURATION.md) for every variable.

---

## Step 3 — Build the Rust Kernel

```powershell
cd OmniHarness/kernel
cargo build --release
```

The kernel binary is output to `OmniHarness/kernel/target/release/omniharness-kernel`. It starts automatically when you use `start.ps1`.

---

## Step 4 — Start Everything

```powershell
# From repo root — starts kernel (gRPC :50051) + orchestrator (HTTP :8000)
OmniHarness/start.ps1
```

Or start components individually:

```powershell
# Terminal 1 — Rust kernel
OmniHarness/kernel/target/release/omniharness-kernel

# Terminal 2 — Python orchestrator
cd OmniHarness/orchestrator
uvicorn omniharness.server:app --host 0.0.0.0 --port 8000
```

---

## Your First AI Response

### CLI

```powershell
omniharness chat "hello"
# → Hello! How can I help you today?

# Use a specific model
omniharness chat --model gpt-4o "Explain monads in one sentence"

# Use local Ollama
omniharness chat --model ollama/llama3.2 "What is 2+2?"
```

### REST API

```bash
curl -X POST http://localhost:8000/api/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "hello", "model": "anthropic/claude-sonnet-4-5"}'
```

Response:

```json
{
  "response": "Hello! How can I help you today?",
  "model": "anthropic/claude-sonnet-4-5",
  "session_id": "sess_abc123",
  "event_id": "evt_xyz789",
  "tokens": {"input": 10, "output": 12}
}
```

### Streaming

```bash
curl -X POST http://localhost:8000/api/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "Write a haiku about compilers"}' \
  --no-buffer
```

Returns server-sent events:

```
data: {"delta": "Tokens"}
data: {"delta": " flow"}
data: {"delta": " through"}
data: [DONE]
```

### WebSocket

```javascript
const ws = new WebSocket("ws://localhost:8000/ws/chat/my-session");
ws.send(JSON.stringify({ message: "hello" }));
ws.onmessage = e => console.log(JSON.parse(e.data).delta);
```

### Titan Native Code

```titan
import omniharness.HarnessCore;

fn main() {
    let harness = HarnessCore.connect("http://localhost:8000");
    let response = harness.chat("hello", model: "anthropic/claude-sonnet-4-5");
    println!(response.text);
}
```

See [INTEGRATION.md](INTEGRATION.md) for the full Titan API.

---

## GUI (ClojureScript)

```powershell
cd OmniHarness/gui
npm install
npx shadow-cljs watch app
# Open http://localhost:3000
```

The GUI provides OmniBar (command palette), ThoughtPanel (agent reasoning), ModelHub (model browser), MemoryGraph (knowledge graph), and Settings.

---

## All Model Formats

OmniHarness uses prefix-based model routing. Pass the model string in any request:

| Format | Example | Provider |
|--------|---------|---------|
| `anthropic/...` | `anthropic/claude-opus-4-5` | Anthropic |
| `gpt-...` | `gpt-4o` | OpenAI |
| `o1`, `o3-mini` | `o3-mini` | OpenAI |
| `gemini/...` | `gemini/gemini-1.5-pro` | Google |
| `groq/...` | `groq/llama-3.3-70b-versatile` | Groq |
| `mistral/...` | `mistral/mistral-large-latest` | Mistral |
| `cohere/...` | `cohere/command-r-plus` | Cohere |
| `openrouter/...` | `openrouter/meta-llama/llama-3.1-405b` | OpenRouter |
| `together/...` | `together/meta-llama/Llama-3-70b-chat-hf` | Together AI |
| `fireworks/...` | `fireworks/accounts/fireworks/models/mixtral-8x7b` | Fireworks AI |
| `ollama/...` | `ollama/llama3.2` | Ollama (local) |

See [MODELS.md](MODELS.md) for the full table with context windows and capabilities.

---

## What to Do Next

| Goal | Doc |
|------|-----|
| Use AI from Titan/Aether/Sylva code | [INTEGRATION.md](INTEGRATION.md) |
| Run agents with tool use | [CLI.md](CLI.md) — `omniharness agent` |
| Store and search memories | [MEMORY.md](MEMORY.md) |
| Call the REST API | [API.md](API.md) |
| Configure all providers | [CONFIGURATION.md](CONFIGURATION.md) |
| Understand the internals | [ARCHITECTURE.md](ARCHITECTURE.md) |
