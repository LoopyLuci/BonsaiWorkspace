# OmniHarness Orchestrator

The OmniHarness orchestrator is a FastAPI service that provides a single gateway
to any language model — local (Ollama, LM Studio, llama.cpp) or API (Anthropic,
OpenAI, Google, Groq, Mistral, Cohere, OpenRouter, Together, Fireworks) — plus
ReAct agents, memory, RAG, ensembles, a multi-agent swarm substrate, and an
optional gRPC bridge to the OmniHarness Rust kernel.

## Zero-setup

You normally do not need to start or install anything by hand. The Omnisystem
VS Code extension starts this server automatically, installs its dependencies
on first run, keeps it alive (auto-restart with backoff), and auto-discovers any
locally-running model runtime.

## Manual run

```bash
# Install runtime dependencies (no package build required)
python -m pip install -r requirements.txt

# Start the server (imported directly from this source tree)
python -m uvicorn omniharness.server:app --host 0.0.0.0 --port 8080
```

Provider API keys are read from the environment or a `.env` file in this
directory. Local runtimes (Ollama on :11434, OpenAI-compatible servers on
common ports) are discovered automatically with no key required.

## Key endpoints

| Method | Path                | Purpose                          |
|--------|---------------------|----------------------------------|
| GET    | `/api/health`       | Reachability + provider health   |
| GET    | `/api/models`       | Aggregated model catalogue       |
| POST   | `/api/chat`         | Chat completion (+ tool calling) |
| POST   | `/api/chat/stream`  | Streaming chat (SSE)             |
| POST   | `/api/swarm/run`    | Multi-agent swarm                |
| POST   | `/api/ensemble/run` | Multi-model ensemble             |
| POST   | `/api/rag/*`        | Retrieval-augmented generation   |
