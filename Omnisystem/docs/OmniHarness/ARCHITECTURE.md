# OmniHarness Architecture

Full technical architecture of OmniHarness — covering the Merkle event chain, gRPC service definitions, model router, ReAct loop, HTN planner, vector store, WASM sandbox, libp2p mesh, episodic memory, knowledge graph, Axiom policy layer, Sylva ML integration, Helix GPU pipelines, and ClojureScript re-frame GUI.

---

## System Overview

```
[Titan/Aether/Sylva/Vera/Nexus/Helix Code]
         │  HarnessCore.titan REST bridge
         ▼
[Python Orchestrator — port 8000]
  FastAPI · Model adapters · ReAct engine
  Memory (episodic + vector + graph)
         │  gRPC channel
         ▼
[Rust Kernel — port 50051]
  Merkle chain · Model router · Vector store
  WASM sandbox · libp2p mesh · Auth
         │
         ├─► Anthropic API
         ├─► OpenAI API
         ├─► Google Gemini API
         ├─► Groq API
         ├─► Mistral API
         ├─► Cohere API
         ├─► OpenRouter API
         ├─► Together AI API
         ├─► Fireworks AI API
         └─► Ollama (local)
```

---

## Merkle Event Chain

**File:** `OmniHarness/kernel/src/event_store.rs`

Every action in OmniHarness — chat message, tool call, memory write, model response — is recorded as an event in a SHA-256 Merkle chain.

### Event Structure

```json
{
  "id": "evt_a3f9b2c1",
  "parent_hash": "sha256:9f3a...",
  "timestamp": "2026-07-02T10:00:00.000Z",
  "kind": "chat_request",
  "session_id": "sess_xyz",
  "payload": { ... },
  "hash": "sha256:7c2d..."
}
```

The `hash` field is `SHA-256(parent_hash + timestamp + kind + session_id + payload)`. Verifying the chain requires checking every `parent_hash` link from genesis to head.

### Storage Format

Events are stored as newline-delimited JSON (JSONL) in `~/.omniharness/events/{date}.jsonl`. The kernel holds the head hash in memory and flushes to disk after every event.

### Querying

The gRPC `EventService` exposes:
- `GetEvent(id)` — single event lookup
- `StreamEvents(session_id, since)` — streaming replay
- `VerifyChain(from, to)` — cryptographic chain verification

---

## gRPC Services

**File:** `OmniHarness/proto/omniharness.proto`

The Rust kernel exposes 7 gRPC services on port 50051:

| Service | Methods | Purpose |
|---------|---------|---------|
| `ModelService` | `Chat`, `ChatStream`, `ListModels` | LLM inference |
| `EventService` | `AppendEvent`, `GetEvent`, `StreamEvents`, `VerifyChain` | Merkle audit chain |
| `VectorService` | `Store`, `Search`, `Delete`, `ListCollections` | Vector embeddings |
| `SessionService` | `Create`, `Get`, `List`, `Delete`, `Update` | Session management |
| `ToolService` | `Register`, `Execute`, `List` | Tool registry and sandboxed execution |
| `MemoryService` | `Store`, `Recall`, `Extract`, `Graph` | Episodic and graph memory |
| `MeshService` | `Publish`, `Subscribe`, `Peers` | libp2p gossipsub |

The Python orchestrator connects to all 7 services via `grpc_client.py`. The Clojure orchestrator connects via `client.clj`.

---

## Model Router

**File:** `OmniHarness/kernel/src/model_router.rs`

The model router infers the provider from the model string prefix and dispatches to the correct adapter. Routing is synchronous in the kernel; actual HTTP calls to provider APIs are made by the Python orchestrator's adapter layer.

### Prefix Table

```
anthropic/   → AnthropicAdapter
gpt-         → OpenAIAdapter
o1           → OpenAIAdapter
o3           → OpenAIAdapter
gemini/      → GeminiAdapter
groq/        → GroqAdapter
mistral/     → MistralAdapter
cohere/      → CohereAdapter
openrouter/  → OpenRouterAdapter
together/    → TogetherAdapter
fireworks/   → FireworksAdapter
ollama/      → OllamaAdapter
(default)    → uses OMNIHARNESS_DEFAULT_MODEL
```

### Fallback Chain

If the primary provider returns an error (rate limit, timeout, model unavailable), the router can optionally fall back to a secondary provider. Configure via `OMNIHARNESS_FALLBACK_MODEL` in `.env`.

---

## ReAct Loop

**File:** `OmniHarness/orchestrator/omniharness/react/engine.py`

The ReAct (Reason + Act) loop enables agents to use tools iteratively to answer complex questions.

### Loop Algorithm

```
1. Receive user message + tool list
2. Send to LLM: "You have these tools: {tools}. Think step by step."
3. Parse LLM response:
   a. If <thought>...</thought> → record reasoning step
   b. If <action tool="X">...</action> → execute tool X in WASM sandbox
   c. If <final_answer>...</final_answer> → return to user
4. Append tool result as observation to context
5. Repeat from step 2 (max 10 iterations)
```

### Tool Execution

Tool calls extracted from `<action>` blocks are routed to the kernel's `ToolService.Execute` gRPC method. The kernel runs the tool in a Wasmtime sandbox (see below). The result is returned as an `<observation>` and appended to the conversation context.

---

## HTN Planner

**File:** `OmniHarness/orchestrator/omniharness/react/planner.py`
**Clojure:** `OmniHarness/clj-orchestrator/src/omniharness/planner.clj`

The Hierarchical Task Network planner decomposes complex goals into sequences of primitive tasks before the ReAct loop begins.

### Example Decomposition

```
Goal: "Research and summarize the top 3 papers on attention mechanisms"
Plan:
  1. web_search("attention mechanism papers 2024")
  2. foreach result[0..2]:
     a. http_request(url)
     b. memory.store(content, collection="papers")
  3. memory.search("attention mechanism", k=10)
  4. chat("Summarize these papers: {results}")
```

The planner outputs a `Plan` protobuf message consumed by the Python orchestrator's ReAct engine.

---

## Vector Store

**File:** `OmniHarness/kernel/src/vector_store.rs`

### Embedding Algorithm

OmniHarness uses a fast local embedding based on FNV-1a hashing projected to 128 dimensions. The algorithm:

1. Tokenize text (whitespace + punctuation)
2. For each token, compute `FNV-1a-64(token)`
3. Distribute the 64-bit hash across 128 float dimensions using modular indexing
4. Normalize the resulting vector to unit length

This avoids a dependency on a heavy embedding model for basic semantic search. For high-accuracy retrieval, configure `OMNIHARNESS_EMBEDDING_MODEL` to use an API-based embedding (OpenAI `text-embedding-3-small`, etc.).

### Cosine Similarity Search

Search returns the top-k vectors by cosine similarity. The kernel stores vectors in a flat in-memory index (no ANN approximation for collections under 100k vectors). For larger collections, configure gRPC vector offloading to an external store.

### Collections

Vectors are organized into named collections (e.g., `episodic`, `documents`, `code`). Each collection is independent; searches are scoped to a single collection.

---

## WASM Sandbox

**File:** `OmniHarness/kernel/src/sandbox.rs`

Tool code submitted by agents runs inside a Wasmtime WebAssembly sandbox with strict resource limits:

| Limit | Value |
|-------|-------|
| Fuel (CPU units) | 100,000,000 |
| Linear memory | 64 MB |
| Table size | 10,000 entries |
| Stack depth | 512 frames |
| Imports allowed | WASI stdio, clock, random |
| Network access | Denied (proxied through kernel) |
| Filesystem access | Denied (proxied through kernel) |

When a tool exhausts its fuel budget, Wasmtime traps and the kernel returns a `TOOL_TIMEOUT` error to the orchestrator.

---

## Tool Registry

**File:** `OmniHarness/kernel/src/tool_registry.rs`

6 built-in tools are registered at startup:

| Tool | Description | Sandboxed |
|------|-------------|-----------|
| `web_search` | Search the web via configured search API | Yes |
| `file_read` | Read a file from the allowed path prefix | Yes |
| `file_write` | Write a file to the allowed path prefix | Yes |
| `code_execute` | Execute code in an isolated interpreter | Yes |
| `http_request` | Make an outbound HTTP request | Yes |
| `shell_command` | Run a shell command (requires `execute` capability) | Yes |

Custom tools can be registered via `POST /api/tools` (see [API.md](API.md)).

---

## Auth

**File:** `OmniHarness/kernel/src/auth.rs`

OmniHarness uses JWT tokens with capability scopes:

| Scope | Grants |
|-------|--------|
| `read` | GET endpoints, chat, memory search |
| `write` | POST endpoints, memory store, tool registration |
| `execute` | Tool execution, shell_command tool |
| `admin` | Session deletion, chain verification, peer management |

Tokens are issued by `POST /api/auth/token` with a configured master secret (`OMNIHARNESS_SECRET`). The kernel validates scopes on every gRPC call.

---

## libp2p Mesh

**File:** `OmniHarness/kernel/src/mesh.rs`

For multi-node deployments, OmniHarness nodes form a gossipsub mesh using libp2p. Each node:

1. Generates an Ed25519 keypair on first start (stored in `~/.omniharness/peer.key`)
2. Connects to bootstrap peers configured in `OMNIHARNESS_BOOTSTRAP_PEERS`
3. Subscribes to topics: `chat`, `events`, `memory`, `tools`
4. Publishes local events to subscribed topics

The `MeshService` gRPC methods expose publish/subscribe to the Python orchestrator, enabling distributed agent coordination across nodes.

---

## Episodic Memory

**File:** `OmniHarness/orchestrator/omniharness/memory/episodic.py`

Episodic memory stores conversation history in SQLite with automatic LLM-based summarization.

### Schema

```sql
CREATE TABLE episodes (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  timestamp TEXT,
  role TEXT,        -- 'user' | 'assistant' | 'tool'
  content TEXT,
  summary TEXT,     -- LLM-generated summary (NULL for recent)
  importance REAL   -- 0.0–1.0, used for pruning
);
```

### Summarization

When a session exceeds `OMNIHARNESS_EPISODE_WINDOW` messages (default: 20), older episodes are summarized: the LLM is called with the raw episode batch and asked to produce a 2–3 sentence summary. The summary replaces the raw content in the context window, keeping history bounded.

---

## Knowledge Graph

**File:** `OmniHarness/orchestrator/omniharness/memory/graph.py`

The knowledge graph stores entity-relationship triples extracted from conversations and documents.

### Triple Format

```
(entity_a, relation, entity_b)
e.g., ("OmniHarness", "uses", "Wasmtime")
     ("Claude", "made_by", "Anthropic")
```

Extraction uses the LLM: `"Extract all (subject, predicate, object) triples from: {text}"`.

### BFS Traversal

Graph queries traverse the adjacency list using breadth-first search. The `GET /api/graph` endpoint accepts a starting entity and depth, returning all reachable triples within that hop count.

---

## Axiom Policy Verification

**File:** `OmniHarness/omni-integration/PolicyVerifier.axiom`

9 formal theorems gate every request before it reaches the model router:

| Theorem | Guards Against |
|---------|---------------|
| `RequestSafety` | Null/empty requests |
| `ModelExists` | Unknown model identifiers |
| `ContextBound` | Token count exceeding model max |
| `ToolAuthorized` | Unauthorized tool names |
| `SandboxCompliant` | Code violating sandbox constraints |
| `MemoryQuota` | Per-session memory limit exceeded |
| `RateLimit` | Per-minute request throttle |
| `ContentPolicy` | Blocked content categories |
| `AuditLogged` | Events that bypass the Merkle chain |

Each theorem is a first-order logic predicate. If any theorem evaluates to `false`, the request is rejected with a `POLICY_VIOLATION` error before consuming any provider tokens.

---

## Sylva ML Integration

**File:** `OmniHarness/omni-integration/MemoryLayer.sylva`

Sylva provides ML-native semantic indexing on top of OmniHarness's vector store:

- **Layer 1** — embedding generation (FNV-1a local or API-based)
- **Layer 2** — semantic clustering for collection organization
- **Layer 3** — relevance scoring combining vector similarity + recency + importance
- **Layer 4** — fine-tuned retrieval ordering for RAG pipelines
- **Layer 5** — online learning: relevance feedback updates importance scores

---

## Helix GPU Pipelines

**File:** `OmniHarness/omni-integration/GPUAcceleration.helix`

5 GPU compute pipelines for accelerating OmniHarness workloads:

| Pipeline | What It Accelerates |
|----------|---------------------|
| `EmbeddingPipeline` | Batch vector generation (128-dim, 10k docs/sec on RTX 4090) |
| `SimilarityPipeline` | Cosine similarity search over large collections |
| `TokenizerPipeline` | GPU-accelerated BPE tokenization |
| `InferencePipeline` | Local GGUF model inference via GPU layers |
| `CompressionPipeline` | Episodic memory compression and summarization |

Pipelines activate automatically when a CUDA/ROCm/Metal device is detected. Falls back to CPU if no GPU is available.

---

## ClojureScript GUI

**Directory:** `OmniHarness/gui/`

The GUI uses the re-frame state management pattern (event → handler → subscription → view):

```
Events (events.cljs)
  :chat/send, :model/select, :memory/search, :tool/run, ...
       ↓
Handlers → update app-db (re-frame effects)
       ↓
Subscriptions (subs.cljs)
  :chat/messages, :model/current, :memory/results, ...
       ↓
Views (views.cljs → components/*.cljs)
  OmniBar, ThoughtPanel, ModelHub, MemoryGraph, ToolPanel, Settings
```

The GUI communicates with the Python orchestrator via REST (chat, memory) and WebSocket (streaming, real-time thought panel).

Build configuration is in `shadow-cljs.edn`. The `:app` build target outputs to `gui/public/js/main.js`.

---

**Next:** [MODELS.md](MODELS.md) | [KERNEL.md](KERNEL.md) | [MEMORY.md](MEMORY.md)
