# OmniHarness Configuration

Every environment variable used by OmniHarness — what it does, its default value, and which layer uses it. The `.env.example` template is reproduced at the bottom.

---

## Configuration File

OmniHarness reads configuration from `OmniHarness/.env`. Copy the example:

```powershell
cp OmniHarness/.env.example OmniHarness/.env
```

Variables can also be set in the shell environment; shell variables take precedence over `.env`.

---

## Provider API Keys

| Variable | Default | Used By | Description |
|----------|---------|---------|-------------|
| `ANTHROPIC_API_KEY` | — | Python orchestrator | Anthropic Claude API key (`sk-ant-...`) |
| `OPENAI_API_KEY` | — | Python orchestrator | OpenAI API key (`sk-...`) |
| `GOOGLE_API_KEY` | — | Python orchestrator | Google Gemini API key (`AIza...`) |
| `GROQ_API_KEY` | — | Python orchestrator | Groq API key (`gsk_...`) |
| `MISTRAL_API_KEY` | — | Python orchestrator | Mistral AI API key |
| `COHERE_API_KEY` | — | Python orchestrator | Cohere API key |
| `OPENROUTER_API_KEY` | — | Python orchestrator | OpenRouter API key (`sk-or-...`) |
| `TOGETHER_API_KEY` | — | Python orchestrator | Together AI API key |
| `FIREWORKS_API_KEY` | — | Python orchestrator | Fireworks AI API key |

At least one provider key must be set, or `OLLAMA_BASE_URL` must point to a running Ollama instance.

---

## Ollama (Local Models)

| Variable | Default | Description |
|----------|---------|-------------|
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Base URL of Ollama REST API |
| `OLLAMA_TIMEOUT` | `120` | Request timeout in seconds (local models can be slow) |

---

## Model Defaults

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_DEFAULT_MODEL` | `anthropic/claude-sonnet-4-5` | Model used when no `model` is specified in a request |
| `OMNIHARNESS_FALLBACK_MODEL` | — | Secondary model if primary provider returns an error |
| `OMNIHARNESS_EMBEDDING_MODEL` | (local FNV-1a) | Override local embeddings with an API-based model (e.g., `openai/text-embedding-3-small`) |

---

## Orchestrator (HTTP Server)

| Variable | Default | Used By | Description |
|----------|---------|---------|-------------|
| `OMNIHARNESS_HOST` | `0.0.0.0` | Python orchestrator | Bind host for REST API |
| `OMNIHARNESS_PORT` | `8000` | Python orchestrator | Bind port for REST API |
| `OMNIHARNESS_WORKERS` | `1` | Python orchestrator | Number of Uvicorn worker processes |
| `OMNIHARNESS_LOG_LEVEL` | `info` | Python orchestrator | Log level: debug / info / warning / error |
| `OMNIHARNESS_CORS_ORIGINS` | `*` | Python orchestrator | Comma-separated allowed CORS origins |

---

## Rust Kernel (gRPC)

| Variable | Default | Used By | Description |
|----------|---------|---------|-------------|
| `OMNIHARNESS_KERNEL_HOST` | `0.0.0.0` | Rust kernel | gRPC bind host |
| `OMNIHARNESS_KERNEL_PORT` | `50051` | Rust kernel + Python | gRPC port |
| `OMNIHARNESS_KERNEL_ADDR` | `localhost:50051` | Python orchestrator | Address Python uses to reach the kernel |
| `OMNIHARNESS_DATA_DIR` | `~/.omniharness` | Rust kernel | Directory for event JSONL, peer key, and vector data |

---

## Authentication

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_SECRET` | — | Master secret for JWT signing. If unset, auth is disabled. |
| `OMNIHARNESS_TOKEN_EXPIRY` | `86400` | JWT expiry in seconds (default: 24 hours) |

---

## Rate Limiting

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_RATE_LIMIT` | `60` | Max requests per minute per client IP |
| `OMNIHARNESS_RATE_WINDOW` | `60` | Rate limit window in seconds |

---

## Memory

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_MEMORY_QUOTA` | `10000` | Max vector entries per session |
| `OMNIHARNESS_EPISODE_WINDOW` | `20` | Number of messages before LLM summarization triggers |
| `OMNIHARNESS_SQLITE_PATH` | `~/.omniharness/episodes.db` | Path to episodic memory SQLite database |
| `OMNIHARNESS_VECTOR_BACKEND` | `local` | Vector backend: `local` (in-memory) or `grpc` (external) |
| `OMNIHARNESS_VECTOR_GRPC_ADDR` | — | gRPC address of external vector store (when backend=grpc) |

---

## WASM Sandbox

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_WASM_FUEL` | `100000000` | Max Wasmtime fuel per tool execution |
| `OMNIHARNESS_WASM_MEMORY_MB` | `64` | Max linear memory per WASM module (MB) |
| `OMNIHARNESS_TOOL_TIMEOUT` | `30` | Tool execution wall-clock timeout (seconds) |
| `OMNIHARNESS_ALLOWED_PATH_PREFIX` | (empty — deny all) | Path prefix for file_read/file_write tools |

---

## libp2p Mesh (Multi-node)

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_MESH_ENABLED` | `false` | Enable libp2p gossipsub mesh |
| `OMNIHARNESS_MESH_PORT` | `4001` | libp2p listen port |
| `OMNIHARNESS_BOOTSTRAP_PEERS` | — | Comma-separated multiaddrs for bootstrap peers |
| `OMNIHARNESS_MESH_TOPICS` | `chat,events,memory,tools` | Gossipsub topics to subscribe |

---

## ClojureScript GUI

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_GUI_PORT` | `3000` | Port for the shadow-cljs dev server |
| `OMNIHARNESS_GUI_API_URL` | `http://localhost:8000` | API URL the GUI connects to |

---

## ReAct Agent

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_REACT_MAX_ITER` | `10` | Default max iterations for ReAct loop |
| `OMNIHARNESS_REACT_TIMEOUT` | `120` | Total agent run timeout (seconds) |

---

## Content Policy

| Variable | Default | Description |
|----------|---------|-------------|
| `OMNIHARNESS_BLOCKED_PATTERNS` | — | Comma-separated regex patterns blocked by ContentPolicy theorem |

---

## .env.example Template

```env
# =============================================================================
# OmniHarness Configuration
# Copy to .env and fill in your values
# =============================================================================

# --- Provider API Keys (set at least one) ---
ANTHROPIC_API_KEY=
OPENAI_API_KEY=
GOOGLE_API_KEY=
GROQ_API_KEY=
MISTRAL_API_KEY=
COHERE_API_KEY=
OPENROUTER_API_KEY=
TOGETHER_API_KEY=
FIREWORKS_API_KEY=

# --- Local Models (Ollama) ---
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_TIMEOUT=120

# --- Model Defaults ---
OMNIHARNESS_DEFAULT_MODEL=anthropic/claude-sonnet-4-5
# OMNIHARNESS_FALLBACK_MODEL=ollama/llama3.2
# OMNIHARNESS_EMBEDDING_MODEL=openai/text-embedding-3-small

# --- Orchestrator ---
OMNIHARNESS_HOST=0.0.0.0
OMNIHARNESS_PORT=8000
OMNIHARNESS_WORKERS=1
OMNIHARNESS_LOG_LEVEL=info
OMNIHARNESS_CORS_ORIGINS=*

# --- Rust Kernel ---
OMNIHARNESS_KERNEL_HOST=0.0.0.0
OMNIHARNESS_KERNEL_PORT=50051
OMNIHARNESS_KERNEL_ADDR=localhost:50051
OMNIHARNESS_DATA_DIR=~/.omniharness

# --- Auth (leave blank to disable) ---
# OMNIHARNESS_SECRET=change-me-in-production
OMNIHARNESS_TOKEN_EXPIRY=86400

# --- Rate Limiting ---
OMNIHARNESS_RATE_LIMIT=60
OMNIHARNESS_RATE_WINDOW=60

# --- Memory ---
OMNIHARNESS_MEMORY_QUOTA=10000
OMNIHARNESS_EPISODE_WINDOW=20
OMNIHARNESS_SQLITE_PATH=~/.omniharness/episodes.db
OMNIHARNESS_VECTOR_BACKEND=local

# --- WASM Sandbox ---
OMNIHARNESS_WASM_FUEL=100000000
OMNIHARNESS_WASM_MEMORY_MB=64
OMNIHARNESS_TOOL_TIMEOUT=30
# OMNIHARNESS_ALLOWED_PATH_PREFIX=/home/user/projects

# --- Mesh (multi-node, disabled by default) ---
OMNIHARNESS_MESH_ENABLED=false
# OMNIHARNESS_MESH_PORT=4001
# OMNIHARNESS_BOOTSTRAP_PEERS=/ip4/1.2.3.4/tcp/4001/p2p/Qm...

# --- GUI ---
OMNIHARNESS_GUI_PORT=3000
OMNIHARNESS_GUI_API_URL=http://localhost:8000

# --- ReAct Agent ---
OMNIHARNESS_REACT_MAX_ITER=10
OMNIHARNESS_REACT_TIMEOUT=120
```

---

**See also:** [QUICKSTART.md](QUICKSTART.md) | [MODELS.md](MODELS.md) | [KERNEL.md](KERNEL.md)
