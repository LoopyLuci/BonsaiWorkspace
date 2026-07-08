# OmniHarness

OmniHarness is the AI inference and agent orchestration subsystem of Omnisystem — a 5-layer polyglot architecture that connects 10+ LLM providers to the Omni-Languages compiler ecosystem with persistent memory, formal policy verification, and GPU-accelerated inference.

---

## What Is OmniHarness?

OmniHarness gives every Omnisystem application access to large language models, autonomous agents, persistent episodic memory, vector search, and a knowledge graph — all through a single unified interface. It is designed so that Titan, Aether, Sylva, Axiom, Vera, Nexus, and Helix code can call AI capabilities without knowing which provider or model is running underneath.

Key design goals:
- **Provider agnosticism** — swap Claude for GPT-4o for Llama3 with one config change
- **Auditability** — every token, every tool call, every memory write is recorded in an append-only Merkle event chain
- **Safety** — Axiom formal theorems gate every request; WASM sandbox isolates tool execution
- **Performance** — Helix GPU pipelines accelerate embeddings; Rust kernel handles I/O at native speed
- **Integration** — native Titan bridge + Aether actor bus means zero FFI friction for Omni-Languages code

---

## 5-Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Layer 5 — ClojureScript GUI                            │
│  OmniBar · ThoughtPanel · ModelHub · MemoryGraph        │
│  OmniHarness/gui/src/omniharness/                       │
├─────────────────────────────────────────────────────────┤
│  Layer 4 — Omni-Languages Integration                   │
│  HarnessCore.titan · ModelBridge.aether                 │
│  MemoryLayer.sylva · PolicyVerifier.axiom               │
│  HarnessUI.vera · HarnessLayout.nexus                   │
│  GPUAcceleration.helix                                  │
│  OmniHarness/omni-integration/                          │
├─────────────────────────────────────────────────────────┤
│  Layer 3 — Clojure Orchestrator                         │
│  Patch manager · HTN planner · Policy enforcer          │
│  OmniHarness/clj-orchestrator/src/omniharness/          │
├─────────────────────────────────────────────────────────┤
│  Layer 2 — Python Orchestrator                          │
│  REST API · CLI · Model adapters · ReAct engine         │
│  Memory (vector + episodic + graph)                     │
│  OmniHarness/orchestrator/omniharness/                  │
├─────────────────────────────────────────────────────────┤
│  Layer 1 — Rust Kernel                                  │
│  Merkle event chain · gRPC server · Model router        │
│  Vector store · WASM sandbox · libp2p mesh              │
│  OmniHarness/kernel/src/                                │
└─────────────────────────────────────────────────────────┘
```

### Layer 1 — Rust Kernel

The kernel is the performance and trust anchor of OmniHarness. It runs as a gRPC server on port 50051 and provides:

- **Merkle event chain** (`event_store.rs`) — SHA-256 linked JSONL log; every event has a cryptographic parent hash making the audit trail tamper-evident
- **Model router** (`model_router.rs`) — prefix-based provider inference (`anthropic/`, `gpt-`, `gemini/`, `ollama/`, etc.) routing to 10 provider backends
- **Vector store** (`vector_store.rs`) — FNV-1a 128-dimensional embeddings with cosine similarity search, no external database required
- **WASM sandbox** (`sandbox.rs`) — Wasmtime with 100 million fuel limit and 64 MB memory cap for safe tool execution
- **Tool registry** (`tool_registry.rs`) — 6 built-in tools: web_search, file_read, file_write, code_execute, http_request, shell_command
- **Auth** (`auth.rs`) — capability-scoped JWT tokens (read / write / execute / admin)
- **Mesh** (`mesh.rs`) — libp2p gossipsub for multi-node agent communication
- **gRPC server** (`grpc_server.rs`) — 7 services defined in `proto/omniharness.proto`

### Layer 2 — Python Orchestrator

FastAPI-based REST server and CLI:

- **REST API** (`server.py`) — 16 HTTP endpoints + WebSocket streaming
- **CLI** (`cli.py`) — 7 commands with rich terminal output
- **Model adapters** — one file per provider in `models/`; each adapter translates the unified request schema to the provider's native format
- **ReAct engine** (`react/engine.py`) — Reason-Act loop: up to 10 iterations, tool calls interleaved with chain-of-thought
- **HTN planner** (`react/planner.py`) — Hierarchical Task Network for multi-step decomposition
- **Memory** — three stores: `memory/vector.py` (embeddings), `memory/episodic.py` (SQLite + LLM summarization), `memory/graph.py` (knowledge graph with BFS traversal)
- **gRPC client** (`grpc_client.py`) — connects Python layer to Rust kernel

### Layer 3 — Clojure Orchestrator

Functional orchestration layer for patch management and planning:

- **core.clj** — entry point, lifecycle management
- **client.clj** — gRPC client to Rust kernel
- **events.clj** — event bus and subscription model
- **patch_manager.clj** — atomic patch application and rollback
- **planner.clj** — HTN plan synthesis
- **policy.clj** — Axiom policy bridge
- **react_engine.clj** — Clojure ReAct implementation

### Layer 4 — Omni-Languages Integration

Seven files in `OmniHarness/omni-integration/`, one per Omni-Language:

| File | Language | Role |
|------|----------|------|
| `HarnessCore.titan` | Titan | REST bridge, session management, sync/async call wrappers |
| `ModelBridge.aether` | Aether | Actor-based async model routing, message streams |
| `MemoryLayer.sylva` | Sylva | ML semantic indexing, embedding generation, retrieval |
| `PolicyVerifier.axiom` | Axiom | 9 formal theorems gating all requests |
| `HarnessUI.vera` | Vera | Chat bubble, thought panel, model badge UI components |
| `HarnessLayout.nexus` | Nexus | Design tokens, breakpoints, responsive chat layout |
| `GPUAcceleration.helix` | Helix | 5 GPU compute pipelines for embeddings and inference |

### Layer 5 — ClojureScript GUI

Re-frame single-page application in `OmniHarness/gui/`:

- **OmniBar** — unified command palette for model selection and chat
- **ThoughtPanel** — live display of ReAct reasoning steps
- **ModelHub** — visual browser for all available providers and models
- **MemoryGraph** — interactive knowledge graph visualization
- **ToolPanel** — real-time tool execution trace
- **Settings** — API key management, model defaults, theme

---

## Key Capabilities

| Capability | Details |
|------------|---------|
| LLM providers | 10 providers, 50+ models |
| Context windows | Up to 2M tokens (Gemini 1.5 Pro) |
| Streaming | Server-sent events + WebSocket |
| Agent mode | ReAct loop, HTN planning, tool use |
| Memory | Episodic (SQLite) + vector (FNV-1a) + graph (BFS) |
| Audit trail | SHA-256 Merkle chain, append-only JSONL |
| Tool sandboxing | Wasmtime, 100M fuel, 64 MB |
| Multi-node | libp2p gossipsub mesh |
| GPU acceleration | Helix compute pipelines |
| Formal verification | Axiom policy theorems |

---

## Quick Start

```powershell
# One command — starts all layers
OmniHarness/start.ps1

# Send your first message
omniharness chat "hello"
```

See [QUICKSTART.md](QUICKSTART.md) for the full setup guide.

---

## Documentation Map

| Doc | What It Covers |
|-----|---------------|
| [QUICKSTART.md](QUICKSTART.md) | Zero to first response |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Full technical design |
| [MODELS.md](MODELS.md) | All providers and models |
| [API.md](API.md) | REST API reference |
| [CLI.md](CLI.md) | CLI command reference |
| [INTEGRATION.md](INTEGRATION.md) | Omni-Languages integration |
| [CONFIGURATION.md](CONFIGURATION.md) | Environment variables |
| [KERNEL.md](KERNEL.md) | Rust kernel internals |
| [MEMORY.md](MEMORY.md) | Memory systems |

---

**Location:** `OmniHarness/` | **Start script:** `OmniHarness/start.ps1` | **gRPC port:** 50051 | **HTTP port:** 8000
