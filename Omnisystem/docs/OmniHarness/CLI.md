# OmniHarness CLI Reference

Complete reference for the `omniharness` command-line interface — every command, every flag, with defaults and examples.

---

## Installation

The CLI is installed as part of the Python orchestrator:

```powershell
cd OmniHarness/orchestrator
pip install -e .
```

After installation, `omniharness` is available in your PATH.

---

## Global Flags

These flags apply to every command:

| Flag | Default | Description |
|------|---------|-------------|
| `--api-url` | `http://localhost:8000` | OmniHarness REST API base URL |
| `--model` | `$OMNIHARNESS_DEFAULT_MODEL` | Model to use (overrides env var) |
| `--no-color` | off | Disable rich terminal color output |
| `--json` | off | Output raw JSON instead of formatted text |
| `--token` | `$OMNIHARNESS_TOKEN` | Bearer token for authenticated endpoints |
| `--verbose` / `-v` | off | Show debug output (gRPC calls, timings) |
| `--help` | — | Show help and exit |

---

## Commands

### omniharness chat

Send a single message to a model and print the response.

```
omniharness chat [OPTIONS] MESSAGE
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--model` / `-m` | default model | Model identifier (e.g., `anthropic/claude-sonnet-4-5`) |
| `--session` / `-s` | new session | Session ID to continue an existing conversation |
| `--system` | none | System prompt |
| `--stream` | on | Stream response tokens as they arrive |
| `--no-stream` | — | Wait for full response before printing |
| `--temperature` / `-t` | 0.7 | Temperature (0.0–1.0) |
| `--max-tokens` | 4096 | Maximum output tokens |
| `--image` | none | Path to image file for vision-capable models |

**Examples:**

```bash
# Basic chat
omniharness chat "What is 2+2?"

# Specific model
omniharness chat --model gpt-4o "Explain recursion"

# Local Ollama model
omniharness chat --model ollama/llama3.2 "Hello!"

# Continue a session
omniharness chat --session sess_abc123 "And what about Paris?"

# With system prompt
omniharness chat --system "You are a pirate." "Greet me"

# With image (vision)
omniharness chat --model anthropic/claude-sonnet-4-5 --image ./screenshot.png "Describe this image"

# Raw JSON output
omniharness chat --json "hello" | jq .response
```

**Interactive mode** (no MESSAGE argument):

```bash
omniharness chat
> Type your message (Ctrl+C to exit)
You: hello
Assistant: Hi! How can I help?
You: _
```

---

### omniharness agent

Run an autonomous agent with ReAct loop and tool use.

```
omniharness agent [OPTIONS] GOAL
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--model` / `-m` | default model | Model to use for the agent |
| `--tools` / `-t` | all tools | Comma-separated list of tools to enable |
| `--max-iterations` | 10 | Maximum ReAct loop iterations |
| `--session` / `-s` | new | Session ID |
| `--system` | none | System prompt injected before goal |
| `--show-thoughts` | on | Print reasoning steps as they happen |
| `--no-thoughts` | — | Only print the final answer |

**Examples:**

```bash
# Simple agent task
omniharness agent "Find the population of Tokyo"

# Limit tools
omniharness agent --tools web_search "What is the latest news about AI?"

# Multi-step research
omniharness agent \
  --model anthropic/claude-opus-4-5 \
  --max-iterations 15 \
  "Research the top 3 quantum computing companies and summarize their latest announcements"

# Suppress intermediate thoughts
omniharness agent --no-thoughts "Calculate the compound interest on $10,000 at 5% for 10 years"
```

**Output format:**

```
[Thought 1] I should search for Tokyo's population...
[Action] web_search("Tokyo population 2026")
[Observation] Tokyo population: approximately 13.96 million...
[Thought 2] I have the answer.

Result: Tokyo has a population of approximately 13.96 million people.
```

---

### omniharness models

List available models and their capabilities.

```
omniharness models [OPTIONS]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--provider` / `-p` | all | Filter by provider name |
| `--tools` | off | Only show models with tool support |
| `--vision` | off | Only show models with vision support |
| `--sort` | `provider` | Sort by: `provider`, `context`, `name` |

**Examples:**

```bash
# List all models
omniharness models

# Filter by provider
omniharness models --provider anthropic
omniharness models --provider ollama

# Only models with tool support
omniharness models --tools

# Sort by context window
omniharness models --sort context
```

**Output:**

```
Provider      Model                           Context   Tools  Vision
────────────  ──────────────────────────────  ────────  ─────  ──────
anthropic     claude-sonnet-4-5               200k      ✓      ✓
anthropic     claude-opus-4-5                 200k      ✓      ✓
anthropic     claude-haiku-4-5                200k      ✓      ✓
gemini        gemini-1.5-pro                  2M        ✓      ✓
ollama        llama3.2                        128k      ✗      ✗
```

---

### omniharness health

Check that the orchestrator and Rust kernel are running correctly.

```
omniharness health [OPTIONS]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--kernel` | on | Also check gRPC kernel connection |
| `--providers` | off | Ping each configured provider API |

**Examples:**

```bash
omniharness health
# → ✓ Orchestrator: ok (v1.0.0, uptime 3600s)
# → ✓ Kernel: connected (gRPC localhost:50051)

omniharness health --providers
# → ✓ Anthropic: ok
# → ✓ OpenAI: ok
# → ✗ Groq: API key not configured
# → ✓ Ollama: ok (3 models installed)
```

---

### omniharness serve

Start the Python orchestrator server.

```
omniharness serve [OPTIONS]
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--host` | `0.0.0.0` | Bind host |
| `--port` | `8000` | Bind port |
| `--workers` | `1` | Number of Uvicorn workers |
| `--reload` | off | Auto-reload on code changes (dev mode) |
| `--kernel-addr` | `localhost:50051` | gRPC kernel address |
| `--log-level` | `info` | Log level: debug, info, warning, error |

**Examples:**

```bash
# Start with defaults
omniharness serve

# Custom port
omniharness serve --port 9000

# Development mode with reload
omniharness serve --reload --log-level debug

# Production with multiple workers
omniharness serve --workers 4 --host 127.0.0.1
```

---

### omniharness remember

Store a piece of text in the vector memory.

```
omniharness remember [OPTIONS] CONTENT
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--collection` / `-c` | `episodic` | Memory collection name |
| `--importance` / `-i` | `0.5` | Importance score (0.0–1.0) |
| `--file` / `-f` | none | Store contents of a file instead of inline text |

**Examples:**

```bash
# Store a fact
omniharness remember "The Omnisystem project was started in 2025"

# With collection and importance
omniharness remember \
  --collection user_prefs \
  --importance 0.9 \
  "User prefers concise technical answers"

# Store a file
omniharness remember --collection documents --file ./notes.txt
```

---

### omniharness recall

Search the vector memory and print matching results.

```
omniharness recall [OPTIONS] QUERY
```

**Options:**

| Flag | Default | Description |
|------|---------|-------------|
| `--collection` / `-c` | `episodic` | Collection to search |
| `--k` | `5` | Number of results to return |
| `--min-score` | `0.6` | Minimum similarity score (0.0–1.0) |
| `--show-scores` | off | Print similarity scores |

**Examples:**

```bash
# Basic recall
omniharness recall "user preferences"

# From a specific collection
omniharness recall --collection documents --k 10 "quantum computing"

# With scores
omniharness recall --show-scores "Tokyo population"
# → [0.94] The user asked about Tokyo's population...
# → [0.82] Tokyo is Japan's capital city...
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Connection error (server not running) |
| 4 | Authentication error |
| 5 | Provider error (API key missing/invalid) |

---

## Environment Variables for CLI

The CLI reads these variables from the environment (or from `OmniHarness/.env` if it exists):

| Variable | CLI Equivalent |
|----------|----------------|
| `OMNIHARNESS_API_URL` | `--api-url` |
| `OMNIHARNESS_DEFAULT_MODEL` | `--model` |
| `OMNIHARNESS_TOKEN` | `--token` |

---

**See also:** [API.md](API.md) | [MODELS.md](MODELS.md) | [CONFIGURATION.md](CONFIGURATION.md)
