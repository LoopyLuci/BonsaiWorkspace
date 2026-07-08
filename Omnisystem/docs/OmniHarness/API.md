# OmniHarness REST API Reference

Complete reference for the OmniHarness HTTP API — every endpoint with method, path, request schema, response schema, and curl examples. Base URL: `http://localhost:8000`.

---

## Authentication

All endpoints accept an optional `Authorization: Bearer <token>` header. If `OMNIHARNESS_SECRET` is not set in `.env`, auth is disabled and all requests are accepted.

---

## Health & Status

### GET /api/health

Check if the orchestrator and kernel are running.

**Response:**
```json
{
  "status": "ok",
  "kernel": "connected",
  "kernel_address": "localhost:50051",
  "version": "1.0.0",
  "uptime_seconds": 3600
}
```

**Curl:**
```bash
curl http://localhost:8000/api/health
```

---

## Models

### GET /api/models

List all available models across all configured providers.

**Query params:**
- `provider` (optional) — filter by provider name (e.g., `anthropic`, `ollama`)

**Response:**
```json
{
  "models": [
    {
      "id": "anthropic/claude-sonnet-4-5",
      "provider": "anthropic",
      "context_window": 200000,
      "supports_tools": true,
      "supports_vision": true
    },
    {
      "id": "ollama/llama3.2",
      "provider": "ollama",
      "context_window": 128000,
      "supports_tools": false,
      "supports_vision": false
    }
  ]
}
```

**Curl:**
```bash
curl "http://localhost:8000/api/models?provider=anthropic"
```

---

## Chat

### POST /api/chat

Send a message and receive a complete response.

**Request:**
```json
{
  "message": "string (required)",
  "model": "string (optional, default: OMNIHARNESS_DEFAULT_MODEL)",
  "session_id": "string (optional, creates new if omitted)",
  "system": "string (optional system prompt)",
  "tools": ["string"] ,
  "images": ["data:image/...;base64,..."],
  "temperature": 0.7,
  "max_tokens": 4096,
  "extras": {}
}
```

**Response:**
```json
{
  "response": "string",
  "model": "anthropic/claude-sonnet-4-5",
  "session_id": "sess_abc123",
  "event_id": "evt_xyz789",
  "tokens": {
    "input": 42,
    "output": 128
  },
  "tool_calls": [],
  "finish_reason": "end_turn"
}
```

**Curl:**
```bash
curl -X POST http://localhost:8000/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "What is the capital of France?",
    "model": "anthropic/claude-sonnet-4-5"
  }'
```

---

### POST /api/chat/stream

Send a message and receive a streaming response via server-sent events.

**Request:** Same as `POST /api/chat`.

**Response:** Server-sent event stream:
```
data: {"delta": "Paris", "event_id": "evt_abc"}
data: {"delta": " is", "event_id": "evt_abc"}
data: {"delta": " the capital.", "event_id": "evt_abc"}
data: [DONE]
```

**Curl:**
```bash
curl -X POST http://localhost:8000/api/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "Tell me a story"}' \
  --no-buffer
```

---

### WebSocket /ws/chat/{session_id}

Bidirectional streaming chat on a persistent session.

**Connect:**
```javascript
const ws = new WebSocket("ws://localhost:8000/ws/chat/sess_abc123");
```

**Send message:**
```json
{"message": "hello", "model": "anthropic/claude-sonnet-4-5"}
```

**Receive stream:**
```json
{"type": "delta", "content": "Hello"}
{"type": "delta", "content": "!"}
{"type": "done", "event_id": "evt_xyz"}
```

---

## Agent

### POST /api/agent/run

Run an autonomous agent with ReAct loop and tool use.

**Request:**
```json
{
  "goal": "string (required)",
  "model": "string (optional)",
  "session_id": "string (optional)",
  "tools": ["web_search", "file_read"],
  "max_iterations": 10,
  "system": "string (optional system prompt)"
}
```

**Response:**
```json
{
  "result": "string (final answer)",
  "steps": [
    {
      "iteration": 1,
      "thought": "I need to search for...",
      "action": {"tool": "web_search", "input": "..."},
      "observation": "Search results: ..."
    }
  ],
  "iterations_used": 3,
  "session_id": "sess_abc123",
  "event_ids": ["evt_1", "evt_2", "evt_3"]
}
```

**Curl:**
```bash
curl -X POST http://localhost:8000/api/agent/run \
  -H "Content-Type: application/json" \
  -d '{
    "goal": "Find the population of Tokyo and convert it to scientific notation",
    "tools": ["web_search"]
  }'
```

---

## Sessions

### POST /api/sessions

Create a new session.

**Request:**
```json
{
  "model": "anthropic/claude-sonnet-4-5",
  "system": "You are a helpful coding assistant.",
  "metadata": {}
}
```

**Response:**
```json
{
  "session_id": "sess_abc123",
  "created_at": "2026-07-02T10:00:00Z",
  "model": "anthropic/claude-sonnet-4-5"
}
```

---

### GET /api/sessions

List all sessions.

**Query params:**
- `limit` (default: 20)
- `offset` (default: 0)

**Response:**
```json
{
  "sessions": [
    {
      "session_id": "sess_abc123",
      "created_at": "2026-07-02T10:00:00Z",
      "model": "anthropic/claude-sonnet-4-5",
      "message_count": 15
    }
  ],
  "total": 42
}
```

---

### GET /api/sessions/{id}

Get a session with full message history.

**Response:**
```json
{
  "session_id": "sess_abc123",
  "model": "anthropic/claude-sonnet-4-5",
  "messages": [
    {"role": "user", "content": "hello", "timestamp": "..."},
    {"role": "assistant", "content": "Hi there!", "timestamp": "..."}
  ],
  "created_at": "2026-07-02T10:00:00Z"
}
```

---

### DELETE /api/sessions/{id}

Delete a session and its message history.

**Response:**
```json
{"deleted": true, "session_id": "sess_abc123"}
```

---

## Memory

### POST /api/memory/store

Store content in the vector memory.

**Request:**
```json
{
  "content": "string (required)",
  "collection": "episodic",
  "metadata": {
    "source": "chat",
    "session_id": "sess_abc123",
    "importance": 0.8
  }
}
```

**Response:**
```json
{
  "id": "mem_abc123",
  "collection": "episodic",
  "stored_at": "2026-07-02T10:00:00Z"
}
```

**Curl:**
```bash
curl -X POST http://localhost:8000/api/memory/store \
  -H "Content-Type: application/json" \
  -d '{
    "content": "The user prefers concise answers.",
    "collection": "user_prefs"
  }'
```

---

### POST /api/memory/search

Search the vector memory by semantic similarity.

**Request:**
```json
{
  "query": "string (required)",
  "collection": "episodic",
  "k": 5,
  "min_score": 0.7
}
```

**Response:**
```json
{
  "results": [
    {
      "id": "mem_abc123",
      "content": "The user prefers concise answers.",
      "score": 0.94,
      "metadata": {"source": "chat"},
      "stored_at": "2026-07-02T10:00:00Z"
    }
  ]
}
```

**Curl:**
```bash
curl -X POST http://localhost:8000/api/memory/search \
  -H "Content-Type: application/json" \
  -d '{"query": "user preferences", "collection": "user_prefs", "k": 3}'
```

---

### GET /api/memory/collections

List all memory collections.

**Response:**
```json
{
  "collections": [
    {"name": "episodic", "count": 1234},
    {"name": "documents", "count": 56},
    {"name": "user_prefs", "count": 7}
  ]
}
```

---

## Tools

### GET /api/tools

List all registered tools.

**Response:**
```json
{
  "tools": [
    {
      "name": "web_search",
      "description": "Search the web for information",
      "parameters": {
        "query": {"type": "string", "required": true},
        "num_results": {"type": "integer", "default": 5}
      },
      "sandboxed": true
    }
  ]
}
```

---

### POST /api/tools/execute

Execute a tool directly (without agent loop).

**Request:**
```json
{
  "tool": "web_search",
  "input": {
    "query": "Omnisystem documentation",
    "num_results": 3
  },
  "session_id": "sess_abc123"
}
```

**Response:**
```json
{
  "result": { ... },
  "execution_time_ms": 234,
  "fuel_used": 1200000,
  "event_id": "evt_abc123"
}
```

**Curl:**
```bash
curl -X POST http://localhost:8000/api/tools/execute \
  -H "Content-Type: application/json" \
  -d '{"tool": "web_search", "input": {"query": "hello world"}}'
```

---

## Knowledge Graph

### POST /api/graph/extract

Extract entity-relationship triples from text and store in the knowledge graph.

**Request:**
```json
{
  "text": "string (required)",
  "model": "string (optional — uses default for extraction)",
  "session_id": "string (optional)"
}
```

**Response:**
```json
{
  "triples": [
    {"subject": "OmniHarness", "predicate": "uses", "object": "Wasmtime"},
    {"subject": "Wasmtime", "predicate": "implements", "object": "WebAssembly"}
  ],
  "count": 2
}
```

---

### GET /api/graph

Query the knowledge graph starting from an entity.

**Query params:**
- `entity` (required) — starting entity name
- `depth` (default: 2) — BFS hop count
- `limit` (default: 50) — max triples returned

**Response:**
```json
{
  "root": "OmniHarness",
  "depth": 2,
  "triples": [
    {"subject": "OmniHarness", "predicate": "uses", "object": "Wasmtime"},
    {"subject": "OmniHarness", "predicate": "written_in", "object": "Rust"},
    {"subject": "Rust", "predicate": "compiles_to", "object": "native_code"}
  ]
}
```

**Curl:**
```bash
curl "http://localhost:8000/api/graph?entity=OmniHarness&depth=2"
```

---

## Error Responses

All errors follow the same format:

```json
{
  "error": {
    "code": "POLICY_VIOLATION",
    "message": "Request violates ContentPolicy theorem",
    "details": {}
  }
}
```

| HTTP Status | Code | Meaning |
|---|---|---|
| 400 | `INVALID_REQUEST` | Malformed request body |
| 401 | `UNAUTHORIZED` | Missing or invalid token |
| 403 | `POLICY_VIOLATION` | Axiom policy theorem failed |
| 404 | `NOT_FOUND` | Session, memory, or tool not found |
| 429 | `RATE_LIMITED` | Too many requests |
| 500 | `KERNEL_ERROR` | gRPC error from Rust kernel |
| 502 | `PROVIDER_ERROR` | LLM provider returned an error |
| 504 | `TOOL_TIMEOUT` | WASM sandbox exceeded fuel budget |

---

**See also:** [CLI.md](CLI.md) | [MODELS.md](MODELS.md) | [MEMORY.md](MEMORY.md)
