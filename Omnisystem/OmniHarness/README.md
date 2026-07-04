# OmniHarness

**The next-generation, enterprise-grade AI harness for Omnisystem.**

OmniHarness is a fully polyglot AI harness that makes Omnisystem capable of using any and all AI models. It puts personal use and individuals first while delivering enterprise-grade reliability, cryptographic integrity, and formal verification.

---

## What it does

- **Chat with any AI model** — Anthropic, OpenAI, Google Gemini, Cohere, Mistral, Groq, OpenRouter, Together, Fireworks, and Ollama — all via a single API
- **Run autonomous agents** — ReAct (Reason+Act) loop with tool use, HTN task planning
- **Persistent memory** — Episodic conversation history, semantic vector search, knowledge graph
- **Immutable audit trail** — SHA-256 Merkle chain — every event is cryptographically linked
- **WASM sandboxing** — Safe execution of untrusted code, 100M fuel + 64MB memory limit
- **P2P mesh** — libp2p gossipsub event replication across nodes
- **Formal verification** — Axiom theorems prove policy correctness mathematically
- **GPU acceleration** — Helix compute pipelines for embedding, attention, cosine search

---

## Quick Start

### Prerequisites

- Rust 1.78+ (for kernel)
- Python 3.11+ (for orchestrator)
- Java 21+ + Clojure CLI (for clj-orchestrator)
- Node 20+ (for ClojureScript GUI)

### 1. Start the Rust kernel

```bash
cd kernel
cargo build --release
ANTHROPIC_API_KEY=sk-ant-... ./target/release/omniharness-kernel
# Listening on [::1]:50051
```

### 2. Start the Python orchestrator

```bash
cd orchestrator
pip install -e ".[all]"
ANTHROPIC_API_KEY=sk-ant-... uvicorn omniharness.server:app --port 8080
# Listening on http://0.0.0.0:8080
```

### 3. (Optional) Start the Clojure orchestrator

```bash
cd clj-orchestrator
OMNIHARNESS_GRPC_HOST=localhost OMNIHARNESS_GRPC_PORT=50051 \
  clojure -M:run
```

### 4. Start the GUI

```bash
cd gui
npm install
npm run dev
# Open http://localhost:3000
```

---

## API

### Chat

```bash
curl -X POST http://localhost:8080/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "model_id": "claude-sonnet-4-6",
    "messages": [{"role":"user","content":"Hello!"}],
    "session_id": "my-session"
  }'
```

### Run agent

```bash
curl -X POST http://localhost:8080/api/agent/run \
  -H "Content-Type: application/json" \
  -d '{
    "objective": "Read README.md and summarize it in 3 bullet points",
    "model_id": "claude-sonnet-4-6",
    "max_steps": 10
  }'
```

### Store memory

```bash
curl -X POST http://localhost:8080/api/memory/store \
  -H "Content-Type: application/json" \
  -d '{"collection":"personal","content":"My favorite color is purple"}'
```

### Search memory

```bash
curl -X POST http://localhost:8080/api/memory/search \
  -H "Content-Type: application/json" \
  -d '{"collection":"personal","query":"color preferences","top_k":5}'
```

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `ANTHROPIC_API_KEY` | — | Anthropic Claude models |
| `OPENAI_API_KEY` | — | OpenAI GPT/o1/o3 models |
| `GOOGLE_API_KEY` | — | Google Gemini models |
| `COHERE_API_KEY` | — | Cohere Command models |
| `MISTRAL_API_KEY` | — | Mistral models |
| `GROQ_API_KEY` | — | Groq-hosted Llama/Mixtral |
| `OPENROUTER_API_KEY` | — | OpenRouter (100+ models) |
| `TOGETHER_API_KEY` | — | Together AI |
| `FIREWORKS_API_KEY` | — | Fireworks AI |
| `OMNIHARNESS_ADMIN_KEY` | (random) | Admin API key |
| `OMNIHARNESS_HOST` | localhost | Kernel host |
| `OMNIHARNESS_GRPC_PORT` | 50051 | Kernel gRPC port |
| `OMNIHARNESS_PYTHON_PORT` | 8080 | Python REST port |
| `OMNIHARNESS_DEFAULT_MODEL` | claude-sonnet-4-6 | Default model |

Ollama is always registered (no key needed) at `http://localhost:11434`.

---

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full technical breakdown.

```
Rust kernel (gRPC :50051)
  ↕
Python orchestrator (REST :8080)
  ↕
Clojure orchestrator (functional policies + HTN)
  ↕
Omni-Languages integration (Titan/Aether/Sylva/Axiom/Vera/Nexus/Helix)
  ↕
ClojureScript GUI (shadow-cljs + re-frame, port 3000)
```

---

## Using from Titan (Omnisystem native)

```titan
use OmniHarness::HarnessCore

fn main() {
    let harness = OmniHarness::create()

    // Simple chat
    let resp = harness.chat("What is the meaning of life?".to_string())?
    println(resp.content)

    // Specific model
    let resp2 = harness.chat_with_model(
        "Explain quantum entanglement".to_string(),
        "gemini-2.0-flash".to_string()
    )?

    // Store memory
    harness.remember("facts".to_string(), "The user prefers dark mode".to_string())?

    // Recall memory
    let memories = harness.recall("facts".to_string(), "UI preferences".to_string(), 5)?

    // Run agent
    let answer = harness.run_agent("Research the latest Rust releases".to_string())?
}
```

---

## Integrity

Every event is recorded in a SHA-256 Merkle chain. The chain is verified on startup:

```
Event N:  hash = sha256("timestamp|module|type|payload|prev_hash")
Event N+1: prev_hash = hash(Event N)
```

If any event is tampered with, `verify_chain()` detects it and returns `false`.

---

## Security

- API keys are SHA-256 hashed at rest
- Capability scopes control what each key can do (`chat`, `memory`, `tools`, `admin`, `*`)  
- WASM execution is sandboxed (fuel + memory limits, WASI allow-list)
- Policy engine is default-deny — unknown actions are blocked
- 9 Axiom formal theorems prove safety invariants mathematically
- Shell tool execution is local-only, never exposed over gRPC

---

## License

Part of the Omnisystem project.
