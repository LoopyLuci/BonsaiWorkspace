# OmniHarness Rust Kernel

Deep-dive into the Rust kernel — the performance and trust anchor of OmniHarness. Covers every source file in `OmniHarness/kernel/src/`, the build process, and the 7 gRPC services.

---

## Overview

The Rust kernel is a gRPC server (port 50051) that provides the foundational services of OmniHarness: event storage, model routing, vector search, tool sandboxing, authentication, and multi-node mesh. It is intentionally stateless in the network layer (any state lives in the event JSONL files or the vector index) so it can be restarted without data loss.

**Build:**
```powershell
cd OmniHarness/kernel
cargo build --release
# Binary: target/release/omniharness-kernel
```

**Start:**
```powershell
OmniHarness/kernel/target/release/omniharness-kernel
# Listening on 0.0.0.0:50051
```

---

## Source Files

```
OmniHarness/kernel/src/
├── main.rs           — Entry point, config loading, server startup
├── auth.rs           — JWT capability scopes
├── event_store.rs    — SHA-256 Merkle event chain (JSONL)
├── grpc_server.rs    — 7 gRPC service implementations
├── mesh.rs           — libp2p gossipsub multi-node mesh
├── model_router.rs   — Prefix-based provider inference
├── sandbox.rs        — Wasmtime WASM execution environment
├── session_store.rs  — In-memory session registry
├── tool_registry.rs  — Built-in tool definitions and dispatch
└── vector_store.rs   — FNV-1a embeddings + cosine similarity
```

---

## main.rs — Entry Point

Loads configuration from environment variables (see [CONFIGURATION.md](CONFIGURATION.md)), initializes all subsystems in dependency order, and starts the gRPC server:

```
1. Load config (env vars → Config struct)
2. Initialize EventStore (open/create JSONL files)
3. Initialize VectorStore (load existing index from OMNIHARNESS_DATA_DIR)
4. Initialize SessionStore (empty in-memory map)
5. Initialize ToolRegistry (register 6 built-in tools)
6. Initialize Sandbox (Wasmtime Engine with configured limits)
7. Initialize Auth (load OMNIHARNESS_SECRET)
8. Initialize Mesh (if OMNIHARNESS_MESH_ENABLED=true)
9. Start gRPC server on OMNIHARNESS_KERNEL_HOST:OMNIHARNESS_KERNEL_PORT
```

Shutdown is graceful: the server stops accepting new connections, waits for in-flight RPCs to complete (up to 30 seconds), then flushes the event store.

---

## auth.rs — JWT Capability Scopes

Issues and validates JWT tokens with granular capability scopes.

### Scopes

```rust
pub enum Scope {
    Read,     // GET endpoints, chat, memory search
    Write,    // POST endpoints, memory store, tool registration
    Execute,  // Tool execution, shell_command
    Admin,    // Session deletion, chain verification, peer management
}
```

### Token Structure

```json
{
  "sub": "client_id",
  "iat": 1751000000,
  "exp": 1751086400,
  "scopes": ["read", "write"]
}
```

Tokens are HMAC-SHA256 signed with `OMNIHARNESS_SECRET`. Every gRPC call checks the `Authorization` metadata field for a valid token with the required scope. If `OMNIHARNESS_SECRET` is unset, all calls are accepted without a token.

---

## event_store.rs — Merkle Event Chain

Appends events to a SHA-256 linked JSONL log, maintaining tamper-evidence.

### Event Lifecycle

```
1. gRPC call AppendEvent(EventRequest)
2. kernel reads current head_hash (last written event's hash)
3. Compute new_hash = SHA-256(head_hash + timestamp + kind + session_id + payload_json)
4. Write JSONL line: {id, parent_hash: head_hash, timestamp, kind, session_id, payload, hash: new_hash}
5. Update head_hash = new_hash
6. Return EventResponse{id, hash}
```

### File Layout

```
~/.omniharness/
├── events/
│   ├── 2026-07-01.jsonl
│   ├── 2026-07-02.jsonl
│   └── ...
└── head_hash.txt   — current chain head (one SHA-256 hex string)
```

### Verification

`VerifyChain(from_id, to_id)` replays the JSONL files between the two event IDs and recomputes every hash, returning `{valid: true/false, broken_at: event_id?}`.

---

## model_router.rs — Provider Inference

Maps model strings to provider names via prefix matching.

### Prefix Table (in order of specificity)

```rust
const PREFIXES: &[(&str, &str)] = &[
    ("anthropic/",  "anthropic"),
    ("gpt-",        "openai"),
    ("o1",          "openai"),
    ("o3",          "openai"),
    ("gemini/",     "gemini"),
    ("groq/",       "groq"),
    ("mistral/",    "mistral"),
    ("cohere/",     "cohere"),
    ("openrouter/", "openrouter"),
    ("together/",   "together"),
    ("fireworks/",  "fireworks"),
    ("ollama/",     "ollama"),
];
```

The router returns the provider name; the Python orchestrator's adapter layer makes the actual API call. This separation keeps the kernel free of provider-specific HTTP logic.

---

## sandbox.rs — Wasmtime Execution

Runs tool code in an isolated WebAssembly environment.

### Engine Configuration

```rust
let mut config = Config::new();
config.consume_fuel(true);           // enable fuel metering
config.max_wasm_stack(512 * 1024);   // 512 KB stack
config.wasm_memory64(false);         // 32-bit addressing only

let engine = Engine::new(&config)?;
```

### Per-Instance Limits

```rust
// Applied to each Store before execution
store.set_fuel(OMNIHARNESS_WASM_FUEL)?;   // default: 100_000_000

let memory_type = MemoryType::new(
    0,                        // min pages
    Some(WASM_MEMORY_PAGES),  // max pages = OMNIHARNESS_WASM_MEMORY_MB * 16
);
```

### WASI Imports

Only safe WASI imports are provided: `wasi_snapshot_preview1` stdio (stdout/stderr), clock, and random. Network and filesystem syscalls are intercepted and proxied through the kernel's tool handlers (so file_read and http_request work, but only within authorized prefixes/URLs).

---

## tool_registry.rs — Built-in Tools

Defines and dispatches the 6 built-in tools.

### Tool Definitions

```rust
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: serde_json::Value,  // JSON Schema
    pub required_scope: Scope,
}
```

The 6 built-in tools and their required scopes:

| Tool | Required Scope | Description |
|------|---------------|-------------|
| `web_search` | Read | Search the web via configured search API |
| `file_read` | Read | Read a file from allowed path prefix |
| `file_write` | Write | Write a file to allowed path prefix |
| `code_execute` | Execute | Execute code in isolated interpreter |
| `http_request` | Read | Make outbound HTTP request |
| `shell_command` | Execute | Run shell command (most restricted) |

### Custom Tool Registration

Custom tools are registered via gRPC `ToolService.Register`. They are stored in a `DashMap<String, Tool>` (thread-safe concurrent hash map). Custom tools must provide a WASM binary; the kernel runs it in the sandbox.

---

## vector_store.rs — Embeddings and Search

In-memory vector store with FNV-1a embeddings and cosine similarity.

### Embedding Algorithm

```rust
pub fn embed(text: &str) -> [f32; 128] {
    let tokens = tokenize(text);  // whitespace + punctuation split
    let mut vec = [0.0f32; 128];
    for token in &tokens {
        let hash = fnv1a_64(token.as_bytes());
        for dim in 0..128 {
            vec[dim] += ((hash >> dim) & 1) as f32;
        }
    }
    normalize(&mut vec);  // L2 normalization to unit length
    vec
}
```

### Search

```rust
pub fn search(query: &str, collection: &str, k: usize) -> Vec<SearchResult> {
    let q_vec = embed(query);
    let entries = self.collections.get(collection)?;
    let mut scores: Vec<(f32, &Entry)> = entries
        .iter()
        .map(|e| (cosine_similarity(&q_vec, &e.vector), e))
        .collect();
    scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    scores.into_iter().take(k).map(|(score, e)| SearchResult { score, ..e.clone() }).collect()
}
```

### Persistence

The vector index is serialized to `~/.omniharness/vectors/{collection}.bin` (bincode format) on shutdown and reloaded on startup.

---

## session_store.rs — Session Registry

Thread-safe in-memory map of `session_id` to `SessionMeta`:

```rust
pub struct SessionMeta {
    pub id: String,
    pub model: String,
    pub system: Option<String>,
    pub created_at: DateTime<Utc>,
    pub message_count: u64,
    pub metadata: serde_json::Value,
}
```

Sessions survive as long as the kernel process runs. For persistent sessions across restarts, use the Python orchestrator's SQLite episodic memory layer.

---

## mesh.rs — libp2p Gossipsub

Multi-node agent coordination via libp2p.

### Startup

```rust
// Generate or load Ed25519 keypair
let keypair = load_or_generate_keypair(data_dir)?;
let peer_id = PeerId::from(&keypair.public());

// Build swarm with gossipsub behaviour
let behaviour = Gossipsub::new(MessageAuthenticity::Signed(keypair), config)?;
let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, peer_id).build();

// Listen
swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{}", mesh_port).parse()?)?;

// Subscribe to topics: chat, events, memory, tools
for topic_name in &config.mesh_topics {
    swarm.behaviour_mut().subscribe(&Topic::new(topic_name))?;
}
```

The `MeshService` gRPC methods `Publish` and `Subscribe` forward messages to/from the gossipsub swarm.

---

## grpc_server.rs — 7 Services

Implements all 7 gRPC service traits defined in `OmniHarness/proto/omniharness.proto`.

| gRPC Service | Delegates To |
|---|---|
| `ModelService` | model_router.rs → Python orchestrator (HTTP callback) |
| `EventService` | event_store.rs |
| `VectorService` | vector_store.rs |
| `SessionService` | session_store.rs |
| `ToolService` | tool_registry.rs + sandbox.rs |
| `MemoryService` | vector_store.rs + event_store.rs |
| `MeshService` | mesh.rs |

Every gRPC handler:
1. Extracts and validates the `Authorization` metadata (auth.rs)
2. Checks the required capability scope
3. Delegates to the appropriate subsystem
4. Wraps the result in the protobuf response type

---

## Building from Source

```powershell
# Prerequisites
rustup install stable

# Development build
cd OmniHarness/kernel
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check for issues
cargo clippy
```

The kernel has no external C dependencies — all dependencies are pure Rust crates: `tokio`, `tonic`, `wasmtime`, `libp2p`, `fnv`, `jsonwebtoken`, `sha2`, `bincode`, `serde_json`, `dashmap`, `chrono`.

---

**See also:** [ARCHITECTURE.md](ARCHITECTURE.md) | [CONFIGURATION.md](CONFIGURATION.md) | [MEMORY.md](MEMORY.md)
