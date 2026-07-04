# OmniHarness Architecture

## Overview

OmniHarness is a fully polyglot, enterprise-grade AI harness built as the AI backbone of Omnisystem. It integrates every major AI provider, runs locally-first, and exposes a unified interface to all Omnisystem subsystems.

```
┌─────────────────────────────────────────────────────────────────┐
│                         OmniHarness                             │
│                                                                 │
│  ┌──────────────┐  ┌──────────────────┐  ┌───────────────────┐ │
│  │  Rust Kernel │  │Python Orchestrator│  │Clojure Orchestrator│ │
│  │  (gRPC :50051)│  │  (FastAPI :8080)  │  │    (Clojure/EDN) │ │
│  │              │  │                  │  │                   │ │
│  │ EventStore   │  │ ModelRouter      │  │ ReactEngine       │ │
│  │ VectorStore  │  │ ReActEngine      │  │ HTNPlanner        │ │
│  │ SessionStore │  │ HTNPlanner       │  │ PolicyEngine      │ │
│  │ ToolRegistry │  │ EpisodicMemory   │  │ PatchManager      │ │
│  │ AuthStore    │  │ KnowledgeGraph   │  │ EventBus          │ │
│  │ Sandbox      │  │ ToolRegistry     │  │                   │ │
│  │ MeshNode     │  │ FastAPI server   │  │                   │ │
│  └──────┬───────┘  └────────┬─────────┘  └────────┬──────────┘ │
│         │ gRPC              │ REST                 │ gRPC       │
│         └───────────────────┴──────────────────────┘           │
│                             │                                   │
│  ┌──────────────────────────┴──────────────────────────────┐   │
│  │           Omni-Languages Integration Layer               │   │
│  │  HarnessCore.titan   — Titan bridge, REST client         │   │
│  │  ModelBridge.aether  — Actor-based model routing         │   │
│  │  MemoryLayer.sylva   — ML semantic memory                │   │
│  │  PolicyVerifier.axiom — Formal policy theorems           │   │
│  │  HarnessUI.vera      — Chat UI components                │   │
│  │  HarnessLayout.nexus — Responsive layout tokens          │   │
│  │  GPUAcceleration.helix — GPU compute pipelines           │   │
│  └──────────────────────────────────────────────────────────┘   │
│                             │                                   │
│  ┌──────────────────────────┴──────────────────────────────┐   │
│  │        ClojureScript GUI (shadow-cljs + re-frame)        │   │
│  │  omnibar, thought-panel, model-hub, memory-graph,        │   │
│  │  tool-panel, settings                                    │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Components

### 1. Rust Kernel (`kernel/`)

The hardened, performance-critical core. Manages all persistent state and exposes a gRPC API.

| Module | Responsibility |
|---|---|
| `event_store.rs` | Immutable SHA-256 Merkle chain event log, JSONL-persisted |
| `model_router.rs` | Provider inference + HTTP calls to 10 AI providers |
| `vector_store.rs` | Cosine similarity search, FNV-1a hash embeddings |
| `sandbox.rs` | Wasmtime WASM execution, 100M fuel + 64MB memory limits |
| `tool_registry.rs` | Built-in tools: file I/O, HTTP, calculator, list_dir |
| `auth.rs` | Capability-scoped API key auth, SHA-256 hashed storage |
| `mesh.rs` | libp2p gossipsub P2P event replication, mDNS discovery |
| `grpc_server.rs` | tonic gRPC server, 7 services from proto |
| `main.rs` | Boot orchestration, graceful shutdown |

**Ports:** gRPC on `[::1]:50051`

### 2. Python Orchestrator (`orchestrator/`)

The AI ecosystem integration layer. Richest SDK support for all providers.

| Module | Responsibility |
|---|---|
| `models/router.py` | ModelRouter with adapters for 9 providers |
| `react/engine.py` | ReAct loop: text-format + native function calling |
| `react/planner.py` | HTN depth-first forward-chaining planner |
| `react/tools.py` | 12 built-in async tools, OpenAI/Anthropic format converters |
| `memory/vector.py` | LocalVectorStore + gRPC VectorClient with fallback |
| `memory/episodic.py` | SQLite conversation history with LLM summarization |
| `memory/graph.py` | Knowledge graph: BFS, entity extraction, JSON import/export |
| `grpc_client.py` | Lazy gRPC client with retry logic |
| `server.py` | FastAPI REST + WebSocket, all endpoints |

**Ports:** REST/WebSocket on `:8080`

### 3. Clojure Orchestrator (`clj-orchestrator/`)

Functional, immutable orchestration layer. Handles policy enforcement, HTN planning, and patch management.

| Namespace | Responsibility |
|---|---|
| `client.clj` | mount-managed gRPC channel + 6 service stubs |
| `events.clj` | Event append/verify/query with Merkle chain |
| `policy.clj` | Compiled policy rules, capability-based, default-deny |
| `react_engine.clj` | Async ReAct loop via core.async |
| `planner.clj` | HTN planner with sequential execution |
| `patch_manager.clj` | AI-generated code patch proposals + human approval |

### 4. Omni-Languages Integration (`omni-integration/`)

Bridges OmniHarness to the Omnisystem native runtime.

| File | Language | Responsibility |
|---|---|---|
| `HarnessCore.titan` | Titan | Main integration struct, REST HTTP client |
| `ModelBridge.aether` | Aether | Actor-based async model routing |
| `MemoryLayer.sylva` | Sylva | ML semantic memory with EmbeddingEncoder/SemanticCompressor |
| `PolicyVerifier.axiom` | Axiom | 9 formal safety theorems |
| `HarnessUI.vera` | Vera | OmniBar, ThoughtPanel, StepCard, ModelSelector, HarnessShell |
| `HarnessLayout.nexus` | Nexus | Design tokens, breakpoints, all layout rules |
| `GPUAcceleration.helix` | Helix | Compute pipelines: dot product, softmax, cosine search, hash embed |

### 5. ClojureScript GUI (`gui/`)

Full browser-based chat UI built with shadow-cljs + Reagent + re-frame.

| File | Responsibility |
|---|---|
| `core.cljs` | Entry point, boot sequence |
| `events.cljs` | All state mutations via re-frame events |
| `subs.cljs` | Derived subscriptions |
| `views.cljs` | Root shell layout |
| `components/omnibar.cljs` | Primary chat input |
| `components/thought_panel.cljs` | Message list + ReAct step visualization |
| `components/model_hub.cljs` | Provider-grouped model selector |
| `components/memory_graph.cljs` | Interactive vector store visualization |
| `components/tool_panel.cljs` | Tool browser + manual execution |
| `components/settings.cljs` | Temperature, tokens, feature toggles |

---

## Data Flow

### Chat request path:
```
User types → OmniBar → re-frame :send-message
  → Python /api/chat → ModelRouter.route()
    → provider adapter (Anthropic/OpenAI/Ollama/…)
      → AI response → chat-response event
        → ThoughtPanel re-renders
          → Rust kernel logs ChatComplete event to Merkle chain
```

### Agent (ReAct) path:
```
User types → :send-message (agent mode)
  → Python /api/agent/run → ReActEngine.run()
    → Reason: LLM generates Thought/Action/ActionInput
    → Act: ToolRegistry.execute(action, input)
    → Observe: result → next iteration
      → Final answer → agent-response event
        → ThoughtPanel shows steps with StepCard expansion
```

### Memory path:
```
Conversation → EpisodicMemory.add_turn()
  → VectorStore.store() (Python local or Rust gRPC)
    → FNV-1a hash embedding (128-dim) OR GPU HashEmbedding pipeline
      → Cosine search on recall
        → Sylva SemanticMemory importance decay + pruning
```

---

## Security Model

- All API keys SHA-256 hashed at rest in `auth.json`
- Capability scopes: `*`, `chat`, `memory`, `tools`, `admin`
- WASM sandbox: 100M fuel limit, 64MB memory cap, WASI allow-list
- Policy engine: default-deny, compiled rules, Axiom-verified invariants
- Chain integrity: every event cryptographically linked; tampering detected on startup
- Shell tools: local-only, not exposed over network gRPC

---

## Merkle Event Chain

Every action in OmniHarness is immutably logged:

```
Genesis: sha256("") = e3b0c44...

Event 1: sha256("ts|module|type|payload|genesis_hash")
Event 2: sha256("ts|module|type|payload|event1_hash")
...

Tamper detection: on startup, replay JSONL file and verify each hash
```

---

## Supported AI Providers

| Provider | Models | Auth |
|---|---|---|
| Anthropic | claude-* | x-api-key |
| OpenAI | gpt-*, o1, o3 | Bearer |
| Google | gemini-* | ?key= |
| Cohere | command-* | Bearer |
| Mistral | mistral-* | Bearer |
| Groq | llama-*, mixtral-* | Bearer |
| OpenRouter | user/model | Bearer |
| Together | * | Bearer |
| Fireworks | * | Bearer |
| Ollama | * | None (local) |

Provider is inferred from `model_id` prefix — no provider field needed.
