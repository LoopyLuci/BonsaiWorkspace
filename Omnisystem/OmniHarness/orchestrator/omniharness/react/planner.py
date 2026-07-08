"""Hierarchical Task Network (HTN) planner for OmniHarness."""
from __future__ import annotations

import dataclasses
from dataclasses import dataclass, field
from typing import Any, Callable


@dataclass
class Task:
    name:        str
    params:      dict[str, Any]   = field(default_factory=dict)
    is_primitive: bool            = False
    priority:    int              = 0


@dataclass
class Operator:
    """Primitive task: has preconditions and effects."""
    name:          str
    preconditions: Callable[[dict], bool]
    effects:       Callable[[dict], dict]
    execute_fn:    Callable[[dict, dict], str]   # (state, params) -> result string


@dataclass
class Method:
    """Compound decomposition: maps a compound task name → list of subtasks."""
    compound_name: str
    condition:     Callable[[dict, dict], bool]   # (state, params) -> bool
    decompose:     Callable[[dict, dict], list[Task]]


class HTNPlanner:
    def __init__(self) -> None:
        self._operators: dict[str, Operator]    = {}
        self._methods:   dict[str, list[Method]] = {}
        self._register_builtins()

    # ── Registration ──────────────────────────────────────────────

    def register_operator(self, op: Operator) -> None:
        self._operators[op.name] = op

    def register_method(self, method: Method) -> None:
        self._methods.setdefault(method.compound_name, []).append(method)

    # ── Planning ──────────────────────────────────────────────────

    def plan(self, task: Task, state: dict) -> list[Task] | None:
        """Depth-first forward-chaining HTN search. Returns primitive plan or None."""
        return self._seek_plan([task], state, [])

    def _seek_plan(
        self,
        tasks: list[Task],
        state: dict,
        plan:  list[Task],
    ) -> list[Task] | None:
        if not tasks:
            return plan

        head, *tail = tasks

        if head.is_primitive or head.name in self._operators:
            op = self._operators.get(head.name)
            if op and not op.preconditions(state):
                return None
            new_state = op.effects(state, head.params) if op else state
            return self._seek_plan(tail, {**state, **new_state}, plan + [head])

        # Compound: try each applicable method
        for method in self._methods.get(head.name, []):
            if method.condition(state, head.params):
                subtasks = method.decompose(state, head.params)
                result   = self._seek_plan(subtasks + tail, state, plan)
                if result is not None:
                    return result

        return None

    async def execute_plan(self, plan: list[Task], state: dict) -> dict:
        results = []
        for task in plan:
            op = self._operators.get(task.name)
            if op:
                try:
                    import asyncio
                    if asyncio.iscoroutinefunction(op.execute_fn):
                        result = await op.execute_fn(state, task.params)
                    else:
                        result = op.execute_fn(state, task.params)
                    state  = {**state, **op.effects(state, task.params)}
                    results.append({"task": task.name, "result": result, "ok": True})
                except Exception as e:
                    results.append({"task": task.name, "error": str(e), "ok": False})
                    break
        return {"results": results, "final_state": state}

    # ── Built-in operators and methods ────────────────────────────

    def _register_builtins(self) -> None:
        # ping
        self.register_operator(Operator(
            name="ping",
            preconditions=lambda s: True,
            effects=lambda s, p: {},
            execute_fn=lambda s, p: f"PONG from {p.get('host', 'localhost')}",
        ))

        # read_file
        self.register_operator(Operator(
            name="read_file",
            preconditions=lambda s: True,
            effects=lambda s, p: {"last_file_content": "..."},
            execute_fn=lambda s, p: open(p["path"]).read()[:2000],
        ))

        # write_file
        self.register_operator(Operator(
            name="write_file",
            preconditions=lambda s: "content" in s or True,
            effects=lambda s, p: {"last_write": p.get("path")},
            execute_fn=lambda s, p: (open(p["path"], "w").write(p.get("content", "")), f"Wrote {p['path']}")[1],
        ))

        # llm_query (calls model via state["model_fn"] if provided)
        def llm_execute(s: dict, p: dict) -> str:
            model_fn = s.get("model_fn")
            if model_fn:
                import asyncio
                return asyncio.get_event_loop().run_until_complete(model_fn(p.get("prompt", "")))
            return f"[LLM would respond to: {p.get('prompt', '')}]"

        self.register_operator(Operator(
            name="llm_query",
            preconditions=lambda s: True,
            effects=lambda s, p: {},
            execute_fn=llm_execute,
        ))

        # Methods
        # "research" → search_web + llm_query to summarize
        self.register_method(Method(
            compound_name="research",
            condition=lambda s, p: True,
            decompose=lambda s, p: [
                Task("http_get", {"url": f"https://api.duckduckgo.com/?q={p.get('query','')}&format=json"}),
                Task("llm_query", {"prompt": f"Summarize the research findings for: {p.get('query','')}"}),
            ],
        ))

        # "code_change" → read + write
        self.register_method(Method(
            compound_name="code_change",
            condition=lambda s, p: "path" in p,
            decompose=lambda s, p: [
                Task("read_file",  {"path": p["path"]}),
                Task("write_file", {"path": p["path"], "content": p.get("new_content", "")}),
            ],
        ))
