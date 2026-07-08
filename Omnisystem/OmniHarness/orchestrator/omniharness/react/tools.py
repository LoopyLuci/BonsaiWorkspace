"""OmniHarness tool registry — built-in tools and dynamic registration."""
from __future__ import annotations

import asyncio
import json
import math
import os
import re
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Awaitable

import aiofiles
import httpx


@dataclass
class Tool:
    name: str
    description: str
    schema: dict
    fn: Callable[..., Awaitable[str]]
    tags: list[str] = field(default_factory=list)


class ToolRegistry:
    def __init__(self) -> None:
        self._tools: dict[str, Tool] = {}
        self._register_builtins()

    # ── Registration ──────────────────────────────────────────────

    def register(self, tool: Tool) -> None:
        self._tools[tool.name] = tool

    def register_fn(
        self,
        name: str,
        description: str,
        schema: dict,
        fn: Callable[..., Awaitable[str]],
        tags: list[str] | None = None,
    ) -> None:
        self.register(Tool(name=name, description=description, schema=schema, fn=fn, tags=tags or []))

    def unregister(self, name: str) -> bool:
        return bool(self._tools.pop(name, None))

    def get(self, name: str) -> Tool | None:
        return self._tools.get(name)

    def list_all(self) -> list[Tool]:
        return list(self._tools.values())

    def as_openai_functions(self) -> list[dict]:
        return [
            {"type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.schema}}
            for t in self._tools.values()
        ]

    def as_anthropic_tools(self) -> list[dict]:
        return [
            {"name": t.name, "description": t.description, "input_schema": t.schema}
            for t in self._tools.values()
        ]

    # ── Execution ─────────────────────────────────────────────────

    async def execute(self, name: str, arguments: str | dict, timeout: float = 30.0) -> str:
        tool = self._tools.get(name)
        if not tool:
            return f"Error: tool '{name}' not found. Available: {', '.join(self._tools)}"
        try:
            args = arguments if isinstance(arguments, dict) else json.loads(arguments or "{}")
        except json.JSONDecodeError as e:
            return f"Error: invalid JSON arguments: {e}"
        try:
            return await asyncio.wait_for(tool.fn(**args), timeout=timeout)
        except asyncio.TimeoutError:
            return f"Error: tool '{name}' timed out after {timeout}s"
        except TypeError as e:
            return f"Error: wrong arguments for '{name}': {e}"
        except Exception as e:
            return f"Error: {type(e).__name__}: {e}"

    # ── Built-in tools ────────────────────────────────────────────

    def _register_builtins(self) -> None:

        async def read_file(path: str) -> str:
            async with aiofiles.open(path, "r", encoding="utf-8", errors="replace") as f:
                content = await f.read()
            lines = content.splitlines()
            if len(lines) > 500:
                content = "\n".join(lines[:500]) + f"\n... ({len(lines)-500} more lines)"
            return content

        async def write_file(path: str, content: str) -> str:
            os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
            async with aiofiles.open(path, "w", encoding="utf-8") as f:
                await f.write(content)
            return f"Written {len(content)} chars to '{path}'"

        async def append_file(path: str, content: str) -> str:
            async with aiofiles.open(path, "a", encoding="utf-8") as f:
                await f.write(content)
            return f"Appended {len(content)} chars to '{path}'"

        async def list_dir(path: str = ".") -> str:
            items = []
            for entry in os.scandir(path):
                kind = "DIR" if entry.is_dir() else f"{entry.stat().st_size}B"
                items.append(f"{entry.name}  [{kind}]")
            return "\n".join(sorted(items)) or "(empty)"

        async def http_get(url: str, headers: dict | None = None) -> str:
            async with httpx.AsyncClient(timeout=30, follow_redirects=True) as c:
                r = await c.get(url, headers=headers or {})
                r.raise_for_status()
                return r.text[:8000]

        async def http_post(url: str, body: dict | None = None, headers: dict | None = None) -> str:
            async with httpx.AsyncClient(timeout=30) as c:
                r = await c.post(url, json=body or {}, headers=headers or {})
                r.raise_for_status()
                return r.text[:8000]

        async def calculator(expression: str) -> str:
            # Safe math eval using ast
            import ast, operator
            ops = {
                ast.Add: operator.add, ast.Sub: operator.sub,
                ast.Mult: operator.mul, ast.Div: operator.truediv,
                ast.Pow: operator.pow, ast.USub: operator.neg,
                ast.UAdd: operator.pos, ast.Mod: operator.mod,
            }
            allowed_names = {k: v for k, v in math.__dict__.items() if not k.startswith("_")}
            allowed_names.update({"abs": abs, "round": round, "int": int, "float": float})

            def _eval(node: ast.AST) -> float:
                if isinstance(node, ast.Constant):
                    return float(node.value)
                if isinstance(node, ast.Name):
                    if node.id in allowed_names:
                        return float(allowed_names[node.id])
                    raise ValueError(f"Unknown name: {node.id}")
                if isinstance(node, ast.BinOp):
                    op = ops.get(type(node.op))
                    if not op: raise ValueError(f"Unsupported op: {node.op}")
                    return op(_eval(node.left), _eval(node.right))
                if isinstance(node, ast.UnaryOp):
                    op = ops.get(type(node.op))
                    if not op: raise ValueError(f"Unsupported unary: {node.op}")
                    return op(_eval(node.operand))
                if isinstance(node, ast.Call):
                    if not isinstance(node.func, ast.Name): raise ValueError("Only simple calls")
                    fn = allowed_names.get(node.func.id)
                    if not callable(fn): raise ValueError(f"Not callable: {node.func.id}")
                    return float(fn(*[_eval(a) for a in node.args]))
                raise ValueError(f"Unsupported AST node: {type(node)}")

            try:
                tree = ast.parse(expression.strip(), mode="eval")
                result = _eval(tree.body)
                return str(result)
            except Exception as e:
                return f"Error evaluating '{expression}': {e}"

        async def search_web(query: str, max_results: int = 5) -> str:
            # DuckDuckGo instant answers API (no key needed)
            async with httpx.AsyncClient(timeout=15) as c:
                r = await c.get(
                    "https://api.duckduckgo.com/",
                    params={"q": query, "format": "json", "no_html": 1},
                )
                data = r.json()
            results = []
            if data.get("AbstractText"):
                results.append(f"Summary: {data['AbstractText']}")
            for topic in data.get("RelatedTopics", [])[:max_results]:
                if isinstance(topic, dict) and "Text" in topic:
                    results.append(f"- {topic['Text']}")
            return "\n".join(results) if results else f"No results for '{query}'"

        async def get_current_time() -> str:
            from datetime import datetime, timezone
            return datetime.now(timezone.utc).isoformat()

        async def json_parse(text: str) -> str:
            try:
                obj = json.loads(text)
                return json.dumps(obj, indent=2)
            except json.JSONDecodeError as e:
                return f"JSON parse error: {e}"

        async def regex_search(pattern: str, text: str) -> str:
            try:
                matches = re.findall(pattern, text)
                return json.dumps(matches[:100])
            except re.error as e:
                return f"Regex error: {e}"

        self.register_fn("read_file",      "Read file contents",                {"type":"object","properties":{"path":{"type":"string"}},"required":["path"]},              read_file)
        self.register_fn("write_file",     "Write content to file",             {"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}, write_file)
        self.register_fn("append_file",    "Append content to file",            {"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}, append_file)
        self.register_fn("list_dir",       "List directory contents",           {"type":"object","properties":{"path":{"type":"string","default":"."}}},                    list_dir)
        self.register_fn("http_get",       "HTTP GET request",                  {"type":"object","properties":{"url":{"type":"string"},"headers":{"type":"object"}},"required":["url"]},             http_get)
        self.register_fn("http_post",      "HTTP POST request with JSON body",  {"type":"object","properties":{"url":{"type":"string"},"body":{"type":"object"},"headers":{"type":"object"}},"required":["url"]}, http_post)
        self.register_fn("calculator",     "Evaluate math expression",          {"type":"object","properties":{"expression":{"type":"string"}},"required":["expression"]},  calculator)
        self.register_fn("search_web",     "Search the web via DuckDuckGo",     {"type":"object","properties":{"query":{"type":"string"},"max_results":{"type":"integer","default":5}},"required":["query"]}, search_web)
        self.register_fn("get_time",       "Get current UTC time",              {"type":"object","properties":{}},                                                           get_current_time)
        self.register_fn("json_parse",     "Parse and pretty-print JSON",       {"type":"object","properties":{"text":{"type":"string"}},"required":["text"]},              json_parse)
        self.register_fn("regex_search",   "Search text with regex pattern",    {"type":"object","properties":{"pattern":{"type":"string"},"text":{"type":"string"}},"required":["pattern","text"]}, regex_search)


# Global registry instance
_registry: ToolRegistry | None = None

def get_registry() -> ToolRegistry:
    global _registry
    if _registry is None:
        _registry = ToolRegistry()
    return _registry
