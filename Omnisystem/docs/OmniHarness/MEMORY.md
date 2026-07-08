# OmniHarness Memory Systems

OmniHarness provides three complementary memory systems — episodic (SQLite + LLM summarization), vector (FNV-1a embeddings + cosine search), and a knowledge graph (entity-relationship triples + BFS traversal) — that work together to give AI agents persistent, searchable context across sessions.

---

## Overview

```
User Message / Document
        │
        ▼
┌───────────────────────────────┐
│  MemoryLayer.sylva            │  ← Sylva ML semantic indexing
│  (automatic during chat)      │
└───────┬───────────────────────┘
        │
        ├──► Vector Store ──► cosine search ──► RAG context
        │    (fast semantic retrieval)
        │
        ├──► Episodic Memory ──► LLM summarization ──► bounded context window
        │    (SQLite, conversation history)
        │
        └──► Knowledge Graph ──► BFS traversal ──► entity relationships
             (entity-relationship triples)
```

All three systems are accessible via:
- REST API (see [API.md](API.md))
- CLI (`omniharness remember` / `omniharness recall`)
- Titan code via `HarnessCore.titan`
- Sylva code via `MemoryLayer.sylva`

---

## 1. Vector Store

**Source:** `OmniHarness/kernel/src/vector_store.rs` (kernel) + `OmniHarness/orchestrator/omniharness/memory/vector.py` (orchestrator)

The vector store converts text to 128-dimensional embeddings and retrieves similar content by cosine similarity.

### How It Works

1. Text is tokenized (whitespace + punctuation split)
2. Each token is hashed with FNV-1a-64
3. The 64-bit hashes are projected across 128 float dimensions
4. The resulting vector is L2-normalized to unit length
5. Vectors are stored in named collections

On search, the query is embedded the same way and cosine similarity is computed against all stored vectors. The top-k results are returned sorted by score.

### Collections

Vectors are organized into named collections. Common collections:

| Collection | Purpose |
|-----------|---------|
| `episodic` | Auto-populated from chat history |
| `documents` | Files and web pages stored with `omniharness remember --file` |
| `user_prefs` | Persistent user preference facts |
| `code` | Code snippets and documentation |

### REST API

**Store:**
```bash
curl -X POST http://localhost:8000/api/memory/store \
  -H "Content-Type: application/json" \
  -d '{
    "content": "OmniHarness uses Wasmtime for WASM sandboxing",
    "collection": "facts",
    "metadata": {"source": "documentation", "importance": 0.8}
  }'
```

**Search:**
```bash
curl -X POST http://localhost:8000/api/memory/search \
  -H "Content-Type: application/json" \
  -d '{"query": "sandbox security", "collection": "facts", "k": 5}'
```

Response:
```json
{
  "results": [
    {
      "id": "mem_abc123",
      "content": "OmniHarness uses Wasmtime for WASM sandboxing",
      "score": 0.94,
      "metadata": {"source": "documentation", "importance": 0.8},
      "stored_at": "2026-07-02T10:00:00Z"
    }
  ]
}
```

**List collections:**
```bash
curl http://localhost:8000/api/memory/collections
```

### Titan Code

```titan
import omniharness.HarnessCore;

let harness = HarnessCore.connect();

// Store
harness.memory_store("OmniHarness uses Wasmtime", collection: "facts");

// Search
let results = harness.memory_search("sandbox security", collection: "facts", k: 5);
for r in results {
    println!("[{:.2}] {}", r.score, r.content);
}
```

### CLI

```bash
omniharness remember "OmniHarness uses Wasmtime" --collection facts
omniharness recall "sandbox security" --collection facts --k 5 --show-scores
```

### High-Accuracy Embeddings

The default FNV-1a local embeddings are fast but approximate. For higher-accuracy semantic search, override with an API-based embedding model:

```env
OMNIHARNESS_EMBEDDING_MODEL=openai/text-embedding-3-small
```

When set, the Python orchestrator calls the embedding API instead of the local FNV-1a algorithm.

---

## 2. Episodic Memory

**Source:** `OmniHarness/orchestrator/omniharness/memory/episodic.py`

Episodic memory stores every message in a conversation and automatically summarizes older messages to keep the context window bounded.

### SQLite Schema

```sql
CREATE TABLE episodes (
  id          TEXT PRIMARY KEY,
  session_id  TEXT NOT NULL,
  timestamp   TEXT NOT NULL,
  role        TEXT NOT NULL,   -- 'user' | 'assistant' | 'tool' | 'system'
  content     TEXT NOT NULL,
  summary     TEXT,            -- LLM summary (NULL = raw, not yet summarized)
  importance  REAL DEFAULT 0.5 -- 0.0–1.0, used for pruning order
);

CREATE INDEX idx_session ON episodes(session_id, timestamp);
```

**Database location:** `OMNIHARNESS_SQLITE_PATH` (default: `~/.omniharness/episodes.db`)

### Automatic Summarization

When a session accumulates more than `OMNIHARNESS_EPISODE_WINDOW` (default: 20) messages, the orchestrator triggers summarization of the oldest batch:

```python
# Simplified summarization flow
old_episodes = get_episodes(session_id, limit=EPISODE_WINDOW, order="asc")
summary_prompt = f"Summarize these conversation turns in 2-3 sentences:\n{format_episodes(old_episodes)}"
summary = call_model(summary_prompt, model=DEFAULT_MODEL, max_tokens=200)
mark_summarized(old_episodes, summary=summary)
```

The summary replaces the raw content of the batch in subsequent context windows, so conversation history stays bounded regardless of session length.

### Context Assembly

When building the context for a new chat message, episodic memory is assembled as:

```
[system prompt]
[oldest batch summary (if summarized)]
[next batch summary (if summarized)]
...
[recent raw messages (up to EPISODE_WINDOW)]
[new user message]
```

### Manual Storage

```bash
# Store a fact in episodic memory manually
omniharness remember "The project deadline is July 15, 2026" --collection episodic

# Retrieve episodic context
omniharness recall "project deadline" --collection episodic
```

---

## 3. Knowledge Graph

**Source:** `OmniHarness/orchestrator/omniharness/memory/graph.py`

The knowledge graph stores entity-relationship triples extracted from text. It enables structured reasoning over facts by traversing entity relationships.

### Triple Format

Each triple is `(subject, predicate, object)`:

```
("OmniHarness", "uses", "Wasmtime")
("Wasmtime", "implements", "WebAssembly")
("WebAssembly", "standard_body", "W3C")
("OmniHarness", "written_in", "Rust")
("Rust", "safety_property", "memory-safe")
```

### Extraction

Triples are extracted from text using the LLM:

```python
extraction_prompt = f"""Extract all (subject, predicate, object) triples from this text.
Return as JSON array: [{{"subject": "...", "predicate": "...", "object": "..."}}]

Text: {text}"""

triples = call_model(extraction_prompt, response_format="json")
```

**REST API:**
```bash
curl -X POST http://localhost:8000/api/graph/extract \
  -H "Content-Type: application/json" \
  -d '{"text": "OmniHarness uses Wasmtime for sandboxing tool calls."}'
```

Response:
```json
{
  "triples": [
    {"subject": "OmniHarness", "predicate": "uses", "object": "Wasmtime"},
    {"subject": "OmniHarness", "predicate": "purpose_of_wasmtime", "object": "sandboxing tool calls"}
  ],
  "count": 2
}
```

### BFS Traversal

The graph is stored as an adjacency list. Queries traverse from a starting entity using BFS:

```bash
curl "http://localhost:8000/api/graph?entity=OmniHarness&depth=2"
```

Response:
```json
{
  "root": "OmniHarness",
  "depth": 2,
  "triples": [
    {"subject": "OmniHarness", "predicate": "uses", "object": "Wasmtime"},
    {"subject": "OmniHarness", "predicate": "written_in", "object": "Rust"},
    {"subject": "Wasmtime", "predicate": "implements", "object": "WebAssembly"},
    {"subject": "Rust", "predicate": "safety_property", "object": "memory-safe"}
  ]
}
```

Depth 1 = direct connections; depth 2 = connections of connections; and so on.

---

## How the Three Systems Work Together

A typical RAG (Retrieval-Augmented Generation) flow uses all three:

```
User: "What do we know about OmniHarness's security?"
         │
         ▼
1. Vector search "OmniHarness security" → top-5 relevant stored facts
2. Knowledge graph BFS from "OmniHarness" → entity relationships
3. Episodic memory → recent conversation context
         │
         ▼
Context = [system] + [episodic history] + [vector results] + [graph triples]
         │
         ▼
LLM generates answer grounded in all three memory sources
```

### Automatic Memory During Chat

When memory auto-population is enabled (`OMNIHARNESS_AUTO_MEMORY=true`), the orchestrator automatically:
- Stores every user message in the vector store (collection: `episodic`)
- Stores every assistant response in episodic SQLite
- Runs triple extraction on significant responses and stores to graph

---

## Memory Quotas and Pruning

| Limit | Config Variable | Default |
|-------|----------------|---------|
| Max vectors per session | `OMNIHARNESS_MEMORY_QUOTA` | 10,000 |
| Messages before summarization | `OMNIHARNESS_EPISODE_WINDOW` | 20 |
| Graph triples per session | (no hard limit) | — |

When the vector quota is reached, the lowest-importance entries (by `metadata.importance`) are pruned first.

---

## Sylva ML Integration

`OmniHarness/omni-integration/MemoryLayer.sylva` adds 5 ML layers on top of the raw vector store:

| Layer | What It Does |
|-------|-------------|
| L1 — Embedding | Text → 128-dim vector (local or API) |
| L2 — Clustering | Groups related memories for faster search |
| L3 — Scoring | Combines similarity + recency + importance |
| L4 — Ranking | Reorders for optimal RAG context placement |
| L5 — Feedback | Updates importance from retrieval outcomes |

Use `MemoryLayer.sylva` when writing Sylva code that needs intelligent memory retrieval — it automatically applies all 5 layers. Use `HarnessCore.titan` for simple store/search from Titan code.

---

**See also:** [INTEGRATION.md](INTEGRATION.md) | [API.md](API.md) | [KERNEL.md](KERNEL.md)
