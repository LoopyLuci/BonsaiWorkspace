# OmniHarness AI Panel (VS Code)

The OmniHarness AI panel is a Claude-Code-style chat/agent surface built directly
into the Omnisystem VS Code extension. It is the primary way to use **any AI model
— local (Ollama) or API (Anthropic, OpenAI, Google, Groq, Mistral, Cohere,
OpenRouter, Together, Fireworks)** — to work on your project without leaving the
editor.

> **Where:** click the **OmniHarness AI** icon in the Activity Bar, or run
> **“Omnisystem / OmniHarness: Open OmniHarness AI Panel”** from the Command Palette.

---

## Architecture

```
 VS Code sidebar webview  (media/harness/harness.js + .css)
          │  postMessage
          ▼
 OmniHarnessViewProvider  (src/harness/OmniHarnessViewProvider.ts)
   ├── AgentRunner        — agentic loop, runs IN the extension host
   │     ├── OmniHarnessClient → orchestrator /api/chat/stream  (model inference)
   │     └── VscodeTools        → read/edit/run/search the live workspace
   ├── HarnessStore       — providers + API keys (SecretStorage) + custom agents
   └── orchestrator lifecycle (start/stop `uvicorn omniharness.server:app`)
```

The **agent loop runs inside the extension** so tool calls act on the real editor,
while **model inference is delegated to the OmniHarness orchestrator** — that is how
one panel drives every provider through a single gateway.

Tool calls run in one of two modes, chosen automatically per model:

- **Native function calling** — tool schemas are sent to the provider and
  structured `tool_calls` come back. Used for Anthropic, OpenAI, Groq, Mistral,
  OpenRouter, Together, and Fireworks (multi-turn round-trip verified).
- **Universal text protocol** — the model emits a fenced
  ` ```tool {"tool": …, "args": …} ``` ` block (or `{"final": …}` when done).
  Works with *any* model, including small local Ollama models that lack native
  function-calling, and providers (Gemini, Cohere) whose structured multi-turn
  round-trip isn’t verified.

Control this with `omnisystem.harness.toolMode`: `auto` (default — native when the
model supports it, else text), `native` (force native), or `text` (force text).

---

## Quick start

1. **Start the orchestrator.** If it is not running, the panel shows a **Start
   Server** button (it runs `python -m uvicorn omniharness.server:app`). You can
   also run **“OmniHarness: Start OmniHarness Orchestrator”**.
2. **Add a provider key.** Open the panel’s **⚙ Settings** → *AI Providers* → paste
   a key → **Save** → **Apply keys to server (.env)** and restart when prompted.
   Keys are stored in VS Code SecretStorage and mirrored to `OmniHarness/.env`.
3. **Pick an agent and model** from the header dropdowns.
4. **Ask.** e.g. *“Add input validation to `src/api/handlers.ts` and run the tests.”*

For local models: run `ollama serve`, `ollama pull llama3.2`, then choose
`ollama/llama3.2` in the model picker — no API key needed.

---

## Agents

Agents are presets controlling the system prompt, model, temperature, and which
tools are permitted. Built-ins:

| Agent | Tools | Approval | Purpose |
|-------|-------|----------|---------|
| **Coder** | all | asks | Full read/edit/run coding agent |
| **Ask** | read-only | auto | Answers questions without changing anything |
| **Architect** | read-only | auto | Explores and proposes an implementation plan |

Create your own in **Settings → Custom Agents**: name, model, system prompt,
temperature, tool allow-list, and auto-approve.

---

## Tools (workspace control)

| Tool | Mutating | What it does |
|------|----------|--------------|
| `read_file` | no | Read a workspace file |
| `list_dir` | no | List a directory |
| `search` | no | Search file contents (path:line:text) |
| `open_file` | no | Open a file at a line in the editor |
| `get_diagnostics` | no | Compiler/linter problems |
| `get_selection` | no | Active file + current selection |
| `write_file` | **yes** | Create/overwrite a file |
| `edit_file` | **yes** | Replace a unique text span |
| `run_command` | **yes** | Run a shell command in the workspace root |

Mutating tools pass through an **approval gate** shown inline in the chat. Control
it with `omnisystem.harness.approvalMode`: `always` (prompt every time),
`auto-read` (auto-approve reads, prompt for writes/commands), or `yolo`
(auto-approve everything).

---

## Settings reference

| Setting | Default | Description |
|---------|---------|-------------|
| `omnisystem.harness.serverUrl` | `http://localhost:8080` | Orchestrator URL (model gateway) |
| `omnisystem.harness.defaultModel` | `anthropic/claude-sonnet-4-6` | Default model id (`provider/model`) |
| `omnisystem.harness.approvalMode` | `always` | `always` \| `auto-read` \| `yolo` |
| `omnisystem.harness.toolMode` | `auto` | `auto` \| `native` \| `text` — how tools are invoked |
| `omnisystem.harness.orchestratorPath` | *(auto)* | Path to the `OmniHarness/` folder |
| `omnisystem.harness.pythonPath` | `python` | Interpreter to launch the orchestrator |
| `omnisystem.harness.autoStartServer` | `false` | Start the orchestrator when the panel opens |

---

## Commands

| Command | Description |
|---------|-------------|
| `omnisystem.harnessFocus` | Open/focus the AI panel |
| `omnisystem.harnessNewSession` | Clear the conversation |
| `omnisystem.harnessSettings` | Jump to the panel settings |
| `omnisystem.harnessStartServer` / `harnessStopServer` | Manage the orchestrator process |
| `omnisystem.harnessAddSelection` | Send the current editor selection as context |

---

## MCP (Model Context Protocol)

The panel is a full MCP **client** — add MCP servers (filesystem, git, GitHub, your
own…) in **Settings → MCP Servers** and their tools become available to the agent,
namespaced `mcp__server__tool`, in both native and text tool modes. OmniHarness is
also an MCP **server** (`python -m omniharness.mcp_server`) other clients can use.
See **[MCP.md](MCP.md)** for the full guide.

---

**See also:** [MCP.md](MCP.md) · [README.md](README.md) · [MODELS.md](MODELS.md) · [API.md](API.md) · [CONFIGURATION.md](CONFIGURATION.md)
