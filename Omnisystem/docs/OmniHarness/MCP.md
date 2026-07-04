# Model Context Protocol (MCP) in OmniHarness

OmniHarness is both an **MCP client** and an **MCP server**, so it plugs into the
wider MCP ecosystem in both directions:

- **As a client** — the OmniHarness AI panel connects to any MCP servers you
  configure (filesystem, git, GitHub, Postgres, Puppeteer, your own…). Their tools
  are merged into the agent's toolset alongside the built-in VS Code tools, so any
  model — local or API — can use them.
- **As a server** — OmniHarness exposes its own capabilities (vector memory + any
  model it can reach) as an MCP server, so external MCP clients (Claude Desktop,
  other IDEs) can use OmniHarness.

MCP is JSON-RPC 2.0. OmniHarness supports both the **stdio** transport (spawn a
subprocess) and **Streamable HTTP** (a URL endpoint).

---

## Using OmniHarness as an MCP client

Open the OmniHarness AI panel → **⚙ Settings → MCP Servers**.

Three example servers ship disabled by default:

| Server | Command | Provides |
|--------|---------|----------|
| Filesystem | `npx -y @modelcontextprotocol/server-filesystem .` | Sandboxed file read/write |
| Git | `npx -y @modelcontextprotocol/server-git` | Repo status, diffs, log, commits |
| OmniHarness | `python -m omniharness.mcp_server` | OmniHarness memory + any model |

**Add a server** (stdio):

```
Name:      GitHub
Transport: stdio
Command:   npx
Arguments: -y
           @modelcontextprotocol/server-github
Env:       GITHUB_TOKEN=ghp_xxx
```

**Add a server** (Streamable HTTP):

```
Name:      My HTTP Server
Transport: http
URL:       http://localhost:3000/mcp
Headers:   Authorization=Bearer xxx
```

Enable a server and its tools appear to any agent that allows all tools,
namespaced as `mcp__<server>__<tool>` (e.g. `mcp__git__git_status`). The
namespacing prevents collisions with the built-in tools and between servers.

MCP tools work in **both** tool modes:

- **Native** — their JSON Schemas are sent to the provider as function
  definitions (Anthropic, OpenAI, Groq, Mistral, OpenRouter, Together, Fireworks).
- **Text** — they are listed in the system prompt and invoked through the
  universal fenced-block protocol (any model, including local ones).

The manager reconnects servers whenever you enable/disable or edit them; the
settings panel shows each server's live connection status and tool count.

---

## OmniHarness as an MCP server

`omniharness.mcp_server` is a dependency-free stdio MCP server exposing:

| Tool | Description |
|------|-------------|
| `omni_memory_search` | Semantic search over OmniHarness vector memory |
| `omni_memory_store` | Store text into vector memory |
| `omni_list_models` | List every model OmniHarness can reach (local + API) |
| `omni_chat` | Send a prompt to any model and return the reply |

**Run it directly:**

```bash
cd OmniHarness/orchestrator
python -m omniharness.mcp_server
```

**Register it with Claude Desktop** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "omniharness": {
      "command": "python",
      "args": ["-m", "omniharness.mcp_server"],
      "cwd": "C:/…/OmniHarness/orchestrator",
      "env": { "ANTHROPIC_API_KEY": "sk-ant-…" }
    }
  }
}
```

Now Claude Desktop can search OmniHarness memory and route prompts to any model
OmniHarness manages — including your local GGUF models.

---

## Architecture

```
 OmniHarness AI panel (VS Code)
   AgentRunner
     ├── VscodeTools           (read/edit/run/search the workspace)
     └── McpManager            (aggregates MCP-server tools, namespaced)
           ├── McpClient  ──stdio──►  npx @modelcontextprotocol/server-*
           └── McpClient  ──http──►   https://…/mcp

 External MCP clients (Claude Desktop, …)
     └──stdio──►  python -m omniharness.mcp_server
                    ├── omni_memory_*   → OmniHarness vector memory
                    └── omni_chat/models → OmniHarness model router (any model)
```

Client source: `vscode-omnisystem/src/harness/mcp/` (`McpClient.ts`,
`McpManager.ts`, `McpTypes.ts`). Server source:
`OmniHarness/orchestrator/omniharness/mcp_server.py`.

---

## Notes & limits

- **stdio launch on Windows** uses a shell so `npx`/`python` resolve on PATH.
- **HTTP transport** captures the `Mcp-Session-Id` header from `initialize` and
  replays it; it parses both `application/json` and `text/event-stream` responses.
- MCP tools are offered to agents whose tool list is “all tools” (`*`); read-only
  agents (Ask, Architect) are intentionally not given arbitrary MCP tools.
- Server→client requests (e.g. sampling) are not yet supported; the client nacks
  them so servers don't hang.

**See also:** [VSCODE_PANEL.md](VSCODE_PANEL.md) · [MODELS.md](MODELS.md) · [CONFIGURATION.md](CONFIGURATION.md)
