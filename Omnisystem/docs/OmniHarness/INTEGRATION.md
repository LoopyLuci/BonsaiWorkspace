# OmniHarness — Omnisystem Integration

How OmniHarness connects to the Omnisystem compiler ecosystem — covering the Titan REST bridge, Aether actor routing, Sylva semantic memory, Axiom policy verification, Vera chat UI, Nexus layout tokens, and Helix GPU pipelines.

---

## Integration Architecture

```
Omnisystem Application Code
      │
      ├── Titan → HarnessCore.titan (sync REST calls)
      ├── Aether → ModelBridge.aether (async actor routing)
      ├── Sylva → MemoryLayer.sylva (ML semantic memory)
      ├── Axiom → PolicyVerifier.axiom (formal safety proofs)
      ├── Vera → HarnessUI.vera (chat UI components)
      ├── Nexus → HarnessLayout.nexus (responsive layout tokens)
      └── Helix → GPUAcceleration.helix (GPU compute pipelines)
                          │
                    OmniHarness REST API
                    http://localhost:8000
                          │
                    Rust Kernel gRPC
                    localhost:50051
```

All seven integration files live in `OmniHarness/omni-integration/`.

---

## HarnessCore.titan — REST Bridge

**File:** `OmniHarness/omni-integration/HarnessCore.titan`

HarnessCore is the primary Titan interface to OmniHarness. It wraps the REST API in idiomatic Titan functions, handles session management, and provides both synchronous and asynchronous call patterns.

### Connecting

```titan
import omniharness.HarnessCore;

// Create a connection (reads OMNIHARNESS_API_URL from env or uses default)
let harness = HarnessCore.connect();

// Explicit URL
let harness = HarnessCore.connect("http://localhost:8000");

// With auth token
let harness = HarnessCore.connect(token: "Bearer abc123");
```

### Synchronous Chat

```titan
let response: HarnessResponse = harness.chat("What is 2+2?");
println!(response.text);         // "4"
println!(response.model);        // "anthropic/claude-sonnet-4-5"
println!(response.session_id);   // "sess_abc123"
```

### Asynchronous Chat

```titan
async fn ask_model(harness: HarnessCore, question: String) -> String {
    let response = await harness.chat_async(question);
    response.text
}
```

### Streaming

```titan
harness.chat_stream("Tell me a story", |token: String| {
    print!(token);  // tokens arrive incrementally
});
println!();
```

### Sessions

```titan
// Start a new session
let session = harness.new_session(model: "anthropic/claude-sonnet-4-5");

// Continue in session
let r1 = harness.chat("My name is Alice", session_id: session.id);
let r2 = harness.chat("What is my name?", session_id: session.id);
println!(r2.text);  // "Your name is Alice"

// List sessions
let sessions = harness.list_sessions();

// Delete
harness.delete_session(session.id);
```

### Agent

```titan
let result = harness.agent(
    goal: "Research the latest GPU benchmarks",
    tools: ["web_search", "http_request"],
    max_iterations: 10
);
println!(result.answer);
for step in result.steps {
    println!("[{}] {}", step.iteration, step.thought);
}
```

### Health Check

```titan
let health = HarnessCore.health();
if health.status != "ok" {
    panic!("OmniHarness not running: {}", health.status);
}
```

---

## ModelBridge.aether — Actor Routing

**File:** `OmniHarness/omni-integration/ModelBridge.aether`

ModelBridge exposes OmniHarness through Aether's actor-based concurrency model. Each model call becomes an actor message, enabling non-blocking fan-out to multiple models simultaneously.

### Basic Actor Usage

```aether
import omniharness.ModelBridge;

actor ModelClient {
    let bridge = ModelBridge.spawn();

    async fn ask(message: String) -> String {
        let response = await bridge.send(ChatMessage {
            content: message,
            model: "anthropic/claude-sonnet-4-5"
        });
        response.text
    }
}
```

### Parallel Multi-Model

```aether
async fn compare_models(question: String) {
    let bridge = ModelBridge.spawn();

    // Send to 3 models simultaneously
    let futures = [
        bridge.send(ChatMessage { content: question, model: "anthropic/claude-sonnet-4-5" }),
        bridge.send(ChatMessage { content: question, model: "gpt-4o" }),
        bridge.send(ChatMessage { content: question, model: "ollama/llama3.2" })
    ];

    let results = await join_all(futures);
    for (i, r) in results.enumerate() {
        println!("Model {}: {}", i, r.text);
    }
}
```

### Streaming Actor

```aether
actor StreamClient {
    async fn stream(message: String, sink: Channel<String>) {
        let bridge = ModelBridge.spawn();
        await bridge.stream(ChatMessage { content: message }, sink);
    }
}
```

### Message Types

```aether
// Chat
ChatMessage { content, model, session_id?, system?, temperature?, max_tokens? }

// Agent
AgentMessage { goal, tools, max_iterations?, session_id? }

// Memory store
MemoryStore { content, collection, metadata? }

// Memory search
MemorySearch { query, collection, k? }
```

---

## MemoryLayer.sylva — ML Semantic Memory

**File:** `OmniHarness/omni-integration/MemoryLayer.sylva`

MemoryLayer integrates OmniHarness's vector store and episodic memory with Sylva's ML pipeline for enhanced semantic retrieval.

### Semantic Store

```sylva
import omniharness.MemoryLayer;

// Store with automatic embedding generation
MemoryLayer.store("OmniHarness uses Wasmtime for sandboxing", collection: "facts");

// Store with explicit importance (affects retrieval ranking)
MemoryLayer.store(
    content: "User prefers concise technical answers",
    collection: "user_prefs",
    importance: 0.9
);
```

### Semantic Search

```sylva
// Basic search
let results = MemoryLayer.search("sandbox security", collection: "facts", k: 5);

// With minimum score threshold
let results = MemoryLayer.search(
    query: "user preferences",
    collection: "user_prefs",
    k: 3,
    min_score: 0.8
);

for result in results {
    println!("[{:.2}] {}", result.score, result.content);
}
```

### RAG Pipeline

```sylva
// Retrieve + Augment + Generate pattern
fn rag_query(question: String, model: String) -> String {
    // 1. Retrieve relevant context
    let context = MemoryLayer.search(question, k: 5);
    let context_text = context.map(|r| r.content).join("\n");

    // 2. Augment prompt with context
    let augmented = "Context:\n{context_text}\n\nQuestion: {question}";

    // 3. Generate via HarnessCore
    let harness = HarnessCore.connect();
    harness.chat(augmented, model: model).text
}
```

### Sylva ML Layers

MemoryLayer uses 5 Sylva ML layers:

| Layer | Function |
|-------|---------|
| L1 — Embedding | Convert text to 128-dim vector |
| L2 — Clustering | Group related memories |
| L3 — Scoring | Combine similarity + recency + importance |
| L4 — Ranking | Reorder results for optimal RAG context |
| L5 — Feedback | Update importance from retrieval outcomes |

---

## PolicyVerifier.axiom — Formal Safety

**File:** `OmniHarness/omni-integration/PolicyVerifier.axiom`

PolicyVerifier enforces 9 formal theorems before any request reaches OmniHarness. The theorems are first-order logic predicates; if any fails, the request is blocked.

### The 9 Theorems

```axiom
// 1. Request cannot be null or empty
theorem RequestSafety(req: ChatRequest):
    req != null && req.message.length > 0

// 2. Model must be in the known provider table
theorem ModelExists(req: ChatRequest):
    exists provider in PROVIDERS: req.model.startsWith(provider.prefix)

// 3. Token count must not exceed model's context window
theorem ContextBound(req: ChatRequest, model: ModelSpec):
    tokenCount(req.message) + tokenCount(req.system) <= model.context_window

// 4. All requested tools must be registered
theorem ToolAuthorized(req: ChatRequest, registry: ToolRegistry):
    forall tool in req.tools: registry.contains(tool)

// 5. Submitted WASM code must satisfy sandbox constraints
theorem SandboxCompliant(code: WasmModule):
    code.memory_pages <= 1024 &&  // 64 MB
    code.fuel_limit <= 100_000_000

// 6. Per-session memory must not exceed quota
theorem MemoryQuota(session_id: String, store: MemoryStore):
    store.count(session_id) < OMNIHARNESS_MEMORY_QUOTA

// 7. Per-minute request count must not exceed throttle
theorem RateLimit(client_id: String, window: TimeWindow):
    window.count(client_id) <= OMNIHARNESS_RATE_LIMIT

// 8. Message must not match blocked content patterns
theorem ContentPolicy(req: ChatRequest):
    !BLOCKED_PATTERNS.any(p => req.message.matches(p))

// 9. Every event must be logged to the Merkle chain
theorem AuditLogged(event: Event, chain: MerkleChain):
    chain.contains(event.id)
```

### Using PolicyVerifier in Titan

```titan
import omniharness.PolicyVerifier;

fn safe_chat(harness: HarnessCore, message: String) -> Result<String, PolicyError> {
    // Verify before sending
    let req = ChatRequest { message: message, model: "anthropic/claude-sonnet-4-5" };
    PolicyVerifier.verify(req)?;  // returns Err if any theorem fails

    Ok(harness.chat(message).text)
}
```

---

## HarnessUI.vera — Chat UI Components

**File:** `OmniHarness/omni-integration/HarnessUI.vera`

HarnessUI provides pre-built Vera UI components for embedding AI chat into Omnisystem applications.

### Components

```vera
import omniharness.HarnessUI;

// Full chat interface
component MyApp {
    render() {
        <ChatPanel
            model="anthropic/claude-sonnet-4-5"
            placeholder="Ask anything..."
            show_thoughts={true}
        />
    }
}

// Inline chat bubble
component InlineChat {
    render() {
        <ChatBubble
            session_id={self.session}
            on_response={|r| self.handle(r)}
        />
    }
}

// Model selector dropdown
component ModelPicker {
    render() {
        <ModelBadge
            providers={["anthropic", "openai", "ollama"]}
            on_change={|m| self.set_model(m)}
        />
    }
}

// Thought panel (shows ReAct reasoning steps)
component AgentView {
    render() {
        <ThoughtPanel
            session_id={self.session}
            live={true}
        />
    }
}
```

### Available Components

| Component | Description |
|-----------|-------------|
| `ChatPanel` | Full chat interface with input, history, streaming |
| `ChatBubble` | Compact inline chat trigger |
| `MessageList` | Scrollable message history |
| `ModelBadge` | Current model indicator + selector |
| `ThoughtPanel` | Live ReAct reasoning step display |
| `MemoryBadge` | Memory collection stats indicator |
| `ToolTrace` | Tool call execution trace |
| `StreamIndicator` | Animated typing/streaming indicator |

---

## HarnessLayout.nexus — Responsive Layout

**File:** `OmniHarness/omni-integration/HarnessLayout.nexus`

HarnessLayout provides design tokens and breakpoints for embedding OmniHarness UI into Nexus responsive layouts.

### Design Tokens

```nexus
// Typography
--harness-font-size-base: 14px
--harness-font-family: "JetBrains Mono", monospace
--harness-line-height: 1.6

// Colors (light/dark aware)
--harness-bg: var(--omni-surface)
--harness-border: var(--omni-border)
--harness-user-bubble: var(--omni-primary-light)
--harness-assistant-bubble: var(--omni-surface-raised)
--harness-thought-color: var(--omni-muted)

// Spacing
--harness-panel-padding: 16px
--harness-bubble-gap: 8px
--harness-input-height: 48px
```

### Responsive Breakpoints

```nexus
breakpoints {
    mobile: max-width 480px    // Compact: collapsed panel, bottom sheet
    tablet: max-width 1024px   // Sidebar: 320px fixed panel
    desktop: min-width 1025px  // Full: 400px resizable panel
}

layout HarnessSidebar {
    @mobile { display: none; }          // Hidden on mobile (use ChatBubble instead)
    @tablet { width: 320px; }
    @desktop { width: clamp(320px, 30vw, 500px); }
}
```

---

## GPUAcceleration.helix — GPU Pipelines

**File:** `OmniHarness/omni-integration/GPUAcceleration.helix`

GPUAcceleration provides 5 Helix compute pipelines that accelerate OmniHarness workloads on available GPU hardware.

### Pipeline Usage

```helix
import omniharness.GPUAcceleration;

// Generate embeddings in batch (GPU: ~10,000 docs/sec on RTX 4090)
let embeddings = GPUAcceleration.EmbeddingPipeline.run(texts: my_documents);

// Cosine similarity search over large vector collection
let results = GPUAcceleration.SimilarityPipeline.search(
    query_vec: my_query_embedding,
    collection: my_vectors,
    k: 10
);

// Tokenize a batch of strings for context counting
let token_counts = GPUAcceleration.TokenizerPipeline.count(texts: messages);

// Run local GGUF model on GPU
let response = GPUAcceleration.InferencePipeline.generate(
    model: "llama3.2.Q4_K_M.gguf",
    prompt: "Hello",
    max_tokens: 512
);
```

### Auto-detection

GPUAcceleration detects and prioritizes available backends:
1. CUDA (NVIDIA)
2. ROCm (AMD)
3. Metal (Apple Silicon)
4. CPU fallback (automatic, no error)

---

## Merkle Chain ↔ Omnisystem Event System

OmniHarness's Merkle event chain connects to Omnisystem's broader event system through the Aether actor bus:

```
OmniHarness Merkle Chain (JSONL, SHA-256)
         │
         │ EventService.StreamEvents (gRPC)
         ▼
ModelBridge.aether EventBus actor
         │
         │ Aether message dispatch
         ▼
Omnisystem Applications (subscribe to AI events)
```

Applications can subscribe to AI events:

```aether
import omniharness.ModelBridge;

actor MyApp {
    fn init() {
        ModelBridge.subscribe("chat_response", |event| {
            self.on_ai_response(event);
        });
        ModelBridge.subscribe("tool_executed", |event| {
            self.on_tool_result(event);
        });
    }
}
```

---

**See also:** [ARCHITECTURE.md](ARCHITECTURE.md) | [API.md](API.md) | [MEMORY.md](MEMORY.md)
