# OmniHarness Model Reference

Complete reference for every LLM provider and model supported by OmniHarness — including context windows, tool support, vision support, required environment variables, and how to specify each model in requests.

---

## How Model Routing Works

Pass a model string in any request. OmniHarness infers the provider from the prefix:

```json
{ "model": "anthropic/claude-opus-4-5" }
```

If no model is specified, `OMNIHARNESS_DEFAULT_MODEL` is used (see [CONFIGURATION.md](CONFIGURATION.md)).

---

## Provider Overview

| Provider | Prefix | Key Env Var | Local? |
|----------|--------|-------------|--------|
| Anthropic | `anthropic/` | `ANTHROPIC_API_KEY` | No |
| OpenAI | `gpt-`, `o1`, `o3` | `OPENAI_API_KEY` | No |
| Google Gemini | `gemini/` | `GOOGLE_API_KEY` | No |
| Groq | `groq/` | `GROQ_API_KEY` | No |
| Mistral | `mistral/` | `MISTRAL_API_KEY` | No |
| Cohere | `cohere/` | `COHERE_API_KEY` | No |
| OpenRouter | `openrouter/` | `OPENROUTER_API_KEY` | No |
| Together AI | `together/` | `TOGETHER_API_KEY` | No |
| Fireworks AI | `fireworks/` | `FIREWORKS_API_KEY` | No |
| Ollama | `ollama/` | `OLLAMA_BASE_URL` | Yes |

---

## Anthropic

**Adapter:** `OmniHarness/orchestrator/omniharness/models/anthropic_adapter.py`
**Env var:** `ANTHROPIC_API_KEY=sk-ant-...`

| Model String | Context | Tools | Vision | Notes |
|---|---|---|---|---|
| `anthropic/claude-opus-4-5` | 200k | Yes | Yes | Most capable, highest cost |
| `anthropic/claude-sonnet-4-5` | 200k | Yes | Yes | Recommended default |
| `anthropic/claude-haiku-4-5` | 200k | Yes | Yes | Fastest, lowest cost |
| `anthropic/claude-opus-4` | 200k | Yes | Yes | Previous gen Opus |
| `anthropic/claude-sonnet-4` | 200k | Yes | Yes | Previous gen Sonnet |
| `anthropic/claude-haiku-3-5` | 200k | Yes | Yes | Previous gen Haiku |

**Extended thinking:** Supported on Opus and Sonnet models. Enable with `"thinking": {"type": "enabled", "budget_tokens": 5000}` in the request extras.

**Example:**
```bash
curl -X POST http://localhost:8000/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Explain quantum entanglement",
    "model": "anthropic/claude-sonnet-4-5"
  }'
```

---

## OpenAI

**Adapter:** `OmniHarness/orchestrator/omniharness/models/openai_adapter.py`
**Env var:** `OPENAI_API_KEY=sk-...`

| Model String | Context | Tools | Vision | Notes |
|---|---|---|---|---|
| `gpt-4o` | 128k | Yes | Yes | Multimodal flagship |
| `gpt-4o-mini` | 128k | Yes | Yes | Cost-effective |
| `gpt-4-turbo` | 128k | Yes | Yes | Previous generation |
| `o1` | 200k | No | No | Reasoning model |
| `o1-mini` | 128k | No | No | Faster reasoning |
| `o3-mini` | 200k | Yes | No | Latest reasoning |

**Example:**
```bash
curl -X POST http://localhost:8000/api/chat \
  -d '{"message": "hello", "model": "gpt-4o"}'
```

---

## Google Gemini

**Adapter:** `OmniHarness/orchestrator/omniharness/models/gemini_adapter.py`
**Env var:** `GOOGLE_API_KEY=AIza...`

| Model String | Context | Tools | Vision | Notes |
|---|---|---|---|---|
| `gemini/gemini-1.5-pro` | 2M | Yes | Yes | Largest context window |
| `gemini/gemini-1.5-flash` | 1M | Yes | Yes | Fast, low cost |
| `gemini/gemini-2.0-flash` | 1M | Yes | Yes | Latest flash |
| `gemini/gemini-2.0-pro` | 2M | Yes | Yes | Latest pro |

**Example:**
```bash
curl -X POST http://localhost:8000/api/chat \
  -d '{"message": "hello", "model": "gemini/gemini-1.5-flash"}'
```

---

## Groq

**Adapter:** `OmniHarness/orchestrator/omniharness/models/groq_adapter.py`
**Env var:** `GROQ_API_KEY=gsk_...`

Groq provides extremely fast inference (often 300+ tokens/sec) via custom LPU hardware.

| Model String | Context | Tools | Vision | Notes |
|---|---|---|---|---|
| `groq/llama-3.3-70b-versatile` | 128k | Yes | No | Best quality on Groq |
| `groq/llama-3.1-8b-instant` | 128k | Yes | No | Fastest |
| `groq/mixtral-8x7b-32768` | 32k | Yes | No | MoE model |
| `groq/gemma2-9b-it` | 8k | No | No | Compact |

---

## Mistral

**Adapter:** `OmniHarness/orchestrator/omniharness/models/mistral_adapter.py`
**Env var:** `MISTRAL_API_KEY=...`

| Model String | Context | Tools | Vision | Notes |
|---|---|---|---|---|
| `mistral/mistral-large-latest` | 128k | Yes | No | Best quality |
| `mistral/mistral-small-latest` | 128k | Yes | No | Balanced |
| `mistral/codestral-latest` | 256k | Yes | No | Code specialist |
| `mistral/open-mixtral-8x22b` | 64k | Yes | No | Open weights |

---

## Cohere

**Adapter:** `OmniHarness/orchestrator/omniharness/models/cohere_adapter.py`
**Env var:** `COHERE_API_KEY=...`

| Model String | Context | Tools | Vision | Notes |
|---|---|---|---|---|
| `cohere/command-r-plus` | 128k | Yes | No | Best for RAG |
| `cohere/command-r` | 128k | Yes | No | Efficient RAG |
| `cohere/command` | 4k | No | No | Legacy |

Cohere models natively support grounded generation and document retrieval — well suited for the OmniHarness RAG memory pipeline.

---

## OpenRouter

**Adapter:** `OmniHarness/orchestrator/omniharness/models/openrouter_adapter.py`
**Env var:** `OPENROUTER_API_KEY=sk-or-...`

OpenRouter is a unified gateway to 200+ models. Use any model from openrouter.ai with the `openrouter/` prefix:

```
openrouter/meta-llama/llama-3.1-405b
openrouter/anthropic/claude-opus-4-5
openrouter/google/gemini-pro-1.5
openrouter/mistralai/mixtral-8x22b
```

Context windows and capabilities depend on the underlying model. Check https://openrouter.ai/models for the full list.

---

## Together AI

**Adapter:** `OmniHarness/orchestrator/omniharness/models/together_adapter.py`
**Env var:** `TOGETHER_API_KEY=...`

| Model String | Context | Tools | Notes |
|---|---|---|---|
| `together/meta-llama/Llama-3-70b-chat-hf` | 8k | No | Popular open model |
| `together/mistralai/Mixtral-8x7B-Instruct-v0.1` | 32k | No | MoE |
| `together/Qwen/Qwen2.5-72B-Instruct-Turbo` | 32k | Yes | High quality |

---

## Fireworks AI

**Adapter:** `OmniHarness/orchestrator/omniharness/models/fireworks_adapter.py`
**Env var:** `FIREWORKS_API_KEY=...`

| Model String | Context | Tools | Notes |
|---|---|---|---|
| `fireworks/accounts/fireworks/models/llama-v3p3-70b-instruct` | 128k | Yes | Fast inference |
| `fireworks/accounts/fireworks/models/mixtral-8x7b-instruct` | 32k | No | MoE |
| `fireworks/accounts/fireworks/models/qwen2p5-72b-instruct` | 32k | Yes | Quality |

---

## Ollama (Local)

**Adapter:** `OmniHarness/orchestrator/omniharness/models/ollama_adapter.py`
**Env var:** `OLLAMA_BASE_URL=http://localhost:11434`

Ollama runs models locally. No API key required. Install from https://ollama.com and pull models with `ollama pull <model>`.

| Model String | Typical Size | Context | Notes |
|---|---|---|---|
| `ollama/llama3.2` | 2B, 3B | 128k | Recommended for local |
| `ollama/llama3.1` | 8B, 70B | 128k | Larger option |
| `ollama/mistral` | 7B | 32k | Instruction tuned |
| `ollama/codellama` | 7B, 13B, 34B | 16k | Code focused |
| `ollama/phi3` | 3.8B | 128k | Microsoft, efficient |
| `ollama/qwen2.5` | 0.5B–72B | 128k | Strong multilingual |
| `ollama/deepseek-r1` | 7B–671B | 128k | Reasoning model |

**List installed models:**
```bash
omniharness models --provider ollama
```

**Pull a new model:**
```bash
ollama pull llama3.2
```

---

## Checking Available Models

```bash
# List all models across all configured providers
omniharness models

# Filter by provider
omniharness models --provider anthropic
omniharness models --provider ollama

# Via REST API
curl http://localhost:8000/api/models
```

---

## Vision / Image Support

Models with Vision=Yes accept base64-encoded images in the `images` field:

```json
{
  "message": "What is in this image?",
  "model": "anthropic/claude-sonnet-4-5",
  "images": ["data:image/png;base64,iVBORw0KGgo..."]
}
```

---

## Tool Use

Models with Tools=Yes can call registered tools. Send tools in the request:

```json
{
  "message": "Search for recent news about Omnisystem",
  "model": "anthropic/claude-sonnet-4-5",
  "tools": ["web_search", "http_request"]
}
```

---

**See also:** [CONFIGURATION.md](CONFIGURATION.md) | [API.md](API.md) | [CLI.md](CLI.md)
