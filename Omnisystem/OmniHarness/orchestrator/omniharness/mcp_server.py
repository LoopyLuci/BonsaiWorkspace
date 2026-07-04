"""
OmniHarness MCP server — exposes OmniHarness capabilities (memory + any model)
as Model Context Protocol tools over stdio, so external MCP clients (Claude
Desktop, the OmniHarness VS Code panel, other IDEs) can use them.

Protocol: JSON-RPC 2.0 over newline-delimited stdio (MCP stdio transport).
Dependency-free beyond the OmniHarness package itself.

Run:  python -m omniharness.mcp_server
"""
from __future__ import annotations

import asyncio
import json
import sys
import threading
from concurrent.futures import Future
from typing import Any, Dict, List, Optional

from .models.router import ModelRouter
from .models.base import ChatRequest, ChatMessage
from .memory.vector import VectorClient

MCP_PROTOCOL_VERSION = "2024-11-05"
SERVER_INFO = {"name": "OmniHarness", "version": "1.0.0"}


# ── Background asyncio loop (for async router / memory calls) ────────────────

class _Runtime:
    """A persistent event loop in a daemon thread + lazily-built components."""

    def __init__(self) -> None:
        self.loop = asyncio.new_event_loop()
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()
        self._router: Optional[ModelRouter] = None
        self._vec: Optional[VectorClient] = None

    def _run(self) -> None:
        asyncio.set_event_loop(self.loop)
        self.loop.run_forever()

    def submit(self, coro) -> Any:
        fut: Future = asyncio.run_coroutine_threadsafe(coro, self.loop)
        return fut.result()

    def router(self) -> ModelRouter:
        if self._router is None:
            r = ModelRouter()
            r.register_from_env()
            self._router = r
        return self._router

    def vec(self) -> VectorClient:
        if self._vec is None:
            self._vec = VectorClient()
        return self._vec


RT = _Runtime()


# ── Tool definitions ────────────────────────────────────────────────────────

TOOLS: List[Dict[str, Any]] = [
    {
        "name": "omni_memory_search",
        "description": "Semantic search over OmniHarness vector memory. Returns the top matches with scores.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "The search query"},
                "collection": {"type": "string", "description": "Memory collection (default: 'default')"},
                "top_k": {"type": "integer", "description": "Number of results (default 5)"},
            },
            "required": ["query"],
        },
    },
    {
        "name": "omni_memory_store",
        "description": "Store a piece of text into OmniHarness vector memory for later retrieval.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "content": {"type": "string", "description": "The text to remember"},
                "collection": {"type": "string", "description": "Memory collection (default: 'default')"},
            },
            "required": ["content"],
        },
    },
    {
        "name": "omni_list_models",
        "description": "List all AI models available through OmniHarness (local + API providers).",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "omni_chat",
        "description": "Send a prompt to any model available through OmniHarness and return the reply.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "prompt": {"type": "string", "description": "The user prompt"},
                "model_id": {"type": "string", "description": "Model id, e.g. anthropic/claude-sonnet-4-6, gpt-4o, ollama/llama3.2"},
                "system": {"type": "string", "description": "Optional system prompt"},
            },
            "required": ["prompt"],
        },
    },
]


# ── Tool execution ──────────────────────────────────────────────────────────

def _run_tool(name: str, args: Dict[str, Any]) -> str:
    if name == "omni_memory_search":
        query = args.get("query", "")
        collection = args.get("collection", "default")
        top_k = int(args.get("top_k", 5))
        results = RT.submit(RT.vec().search(collection, query, top_k, 0.0))
        if not results:
            return "(no matches)"
        return "\n".join(f"[{getattr(e, 'score', 0):.2f}] {e.content}" for e in results)

    if name == "omni_memory_store":
        content = args.get("content", "")
        collection = args.get("collection", "default")
        eid = RT.submit(RT.vec().store(collection, content, {}))
        return f"Stored in '{collection}' (id={eid})."

    if name == "omni_list_models":
        models = RT.router().list_all_models()
        if not models:
            return "(no models — set a provider API key or enable Ollama/local)"
        return "\n".join(f"{m.provider}/{m.id} (ctx {m.context_window}, tools={m.supports_tools})" for m in models)

    if name == "omni_chat":
        prompt = args.get("prompt", "")
        model_id = args.get("model_id") or "anthropic/claude-sonnet-4-6"
        system = args.get("system")
        req = ChatRequest(
            model_id=model_id,
            messages=[ChatMessage(role="user", content=prompt)],
            system=system,
        )
        resp = RT.submit(RT.router().chat(req))
        return resp.content

    raise ValueError(f"Unknown tool: {name}")


# ── JSON-RPC dispatch ───────────────────────────────────────────────────────

def _handle(msg: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    method = msg.get("method")
    mid = msg.get("id")

    # Notifications (no id) never get a response.
    if method == "notifications/initialized":
        return None

    if method == "initialize":
        return {
            "jsonrpc": "2.0", "id": mid,
            "result": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": SERVER_INFO,
            },
        }

    if method == "tools/list":
        return {"jsonrpc": "2.0", "id": mid, "result": {"tools": TOOLS}}

    if method == "tools/call":
        params = msg.get("params") or {}
        name = params.get("name", "")
        args = params.get("arguments") or {}
        try:
            text = _run_tool(name, args)
            return {
                "jsonrpc": "2.0", "id": mid,
                "result": {"content": [{"type": "text", "text": text}], "isError": False},
            }
        except Exception as exc:  # noqa: BLE001 — report as tool error, not a crash
            return {
                "jsonrpc": "2.0", "id": mid,
                "result": {"content": [{"type": "text", "text": f"Error: {exc}"}], "isError": True},
            }

    if method == "ping":
        return {"jsonrpc": "2.0", "id": mid, "result": {}}

    # Unknown method with an id → JSON-RPC "method not found".
    if mid is not None:
        return {"jsonrpc": "2.0", "id": mid, "error": {"code": -32601, "message": f"Method not found: {method}"}}
    return None


def main() -> None:
    out = sys.stdout
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        try:
            response = _handle(msg)
        except Exception as exc:  # noqa: BLE001
            response = {
                "jsonrpc": "2.0", "id": msg.get("id"),
                "error": {"code": -32603, "message": f"Internal error: {exc}"},
            }
        if response is not None:
            out.write(json.dumps(response) + "\n")
            out.flush()


if __name__ == "__main__":
    main()
