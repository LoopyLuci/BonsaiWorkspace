"""OmniHarness ReAct engine — Reason + Act loop with native tool calling support."""
from __future__ import annotations

import json
import re
import time
from dataclasses import dataclass, field
from typing import Any

from ..models.base import ChatMessage, ChatRequest, ChatResponse
from ..models.router import ModelRouter
from .tools import ToolRegistry


@dataclass
class ReActStep:
    step:         int
    thought:      str
    action:       str
    action_input: Any
    observation:  str
    latency_ms:   float = 0.0
    used_native:  bool = False  # True if native tool_calls were used


@dataclass
class ReActResult:
    answer:       str
    steps:        list[ReActStep]
    success:      bool
    total_tokens: int
    elapsed_ms:   float


SYSTEM_PROMPT = """\
You are a highly capable AI assistant with access to tools.
When given a task:
1. Think step-by-step about what to do (Thought:)
2. Choose an action (Action: tool_name)
3. Provide the input (ActionInput: {{...}})
4. After seeing the result, reason again
5. When done, respond with Action: FinalAnswer and ActionInput: {{\"answer\": \"your answer\"}}

Always use valid JSON for ActionInput. Available tools: {tools}
"""

REACT_FORMAT = """
Thought: {thought}
Action: {action}
ActionInput: {action_input}
""".strip()


class ReActEngine:
    def __init__(self, router: ModelRouter, tools: ToolRegistry) -> None:
        self.router = router
        self.tools  = tools

    async def run(
        self,
        objective:  str,
        model_id:   str = "claude-sonnet-4-6",
        max_steps:  int = 20,
        temperature: float = 0.7,
        extra_context: str = "",
    ) -> ReActResult:
        t_start     = time.monotonic()
        history:    list[ChatMessage] = []
        steps:      list[ReActStep]   = []
        total_tokens = 0

        tool_names  = ", ".join(t.name for t in self.tools.list_all())
        system      = SYSTEM_PROMPT.format(tools=tool_names)
        anthropic_tools = self.tools.as_anthropic_tools()

        if extra_context:
            history.append(ChatMessage(role="user", content=f"Context:\n{extra_context}"))
            history.append(ChatMessage(role="assistant", content="Understood. I'll use this context."))

        history.append(ChatMessage(role="user", content=f"Task: {objective}"))

        for step_num in range(max_steps):
            t0 = time.monotonic()
            req = ChatRequest(
                model_id=model_id,
                messages=history,
                temperature=temperature,
                max_tokens=4096,
                system=system,
                tools=[],  # pass as anthropic native tools if supported
            )

            resp: ChatResponse = await self.router.chat(req)
            total_tokens += resp.input_tokens + resp.output_tokens
            latency = (time.monotonic() - t0) * 1000

            # ── Native tool_calls path (function calling) ──────────

            if resp.tool_calls:
                tc = resp.tool_calls[0]
                thought_text  = resp.content or "(reasoning via tool call)"
                action        = tc.name
                action_input  = tc.arguments

                history.append(ChatMessage(role="assistant", content=resp.content or ""))
                t_exec = time.monotonic()
                observation = await self.tools.execute(action, action_input)
                exec_ms     = (time.monotonic() - t_exec) * 1000

                history.append(ChatMessage(
                    role="tool", content=observation,
                    name=action, tool_call_id=tc.id,
                ))

                steps.append(ReActStep(
                    step=step_num, thought=thought_text, action=action,
                    action_input=action_input, observation=observation,
                    latency_ms=latency + exec_ms, used_native=True,
                ))

                if action.lower() in ("final_answer", "finalanswer"):
                    try:
                        ans = json.loads(action_input)
                        answer = ans.get("answer", str(ans))
                    except Exception:
                        answer = str(action_input)
                    return ReActResult(
                        answer=answer, steps=steps, success=True,
                        total_tokens=total_tokens,
                        elapsed_ms=(time.monotonic() - t_start) * 1000,
                    )
                continue

            # ── Text-format ReAct parsing ─────────────────────────

            parsed = self._parse_react_text(resp.content)
            thought      = parsed.get("thought", "")
            action       = parsed.get("action", "FinalAnswer")
            action_input = parsed.get("action_input", {})

            history.append(ChatMessage(role="assistant", content=resp.content))

            if action.lower() in ("final_answer", "finalanswer", "final answer"):
                if isinstance(action_input, dict):
                    answer = action_input.get("answer", str(action_input))
                else:
                    answer = str(action_input) or resp.content
                steps.append(ReActStep(
                    step=step_num, thought=thought, action=action,
                    action_input=action_input, observation="",
                    latency_ms=latency,
                ))
                return ReActResult(
                    answer=answer, steps=steps, success=True,
                    total_tokens=total_tokens,
                    elapsed_ms=(time.monotonic() - t_start) * 1000,
                )

            # Execute tool
            args_str = json.dumps(action_input) if isinstance(action_input, dict) else str(action_input)
            t_exec   = time.monotonic()
            observation = await self.tools.execute(action, args_str)
            exec_ms     = (time.monotonic() - t_exec) * 1000

            history.append(ChatMessage(role="user", content=f"Observation: {observation}"))

            steps.append(ReActStep(
                step=step_num, thought=thought, action=action,
                action_input=action_input, observation=observation,
                latency_ms=latency + exec_ms,
            ))

        # Max steps reached — ask for final answer
        history.append(ChatMessage(role="user", content="You have reached the maximum number of steps. Provide your final answer now."))
        req = ChatRequest(model_id=model_id, messages=history, temperature=0.3, max_tokens=1024, system=system)
        final_resp = await self.router.chat(req)
        total_tokens += final_resp.input_tokens + final_resp.output_tokens

        return ReActResult(
            answer=final_resp.content,
            steps=steps,
            success=False,
            total_tokens=total_tokens,
            elapsed_ms=(time.monotonic() - t_start) * 1000,
        )

    def _parse_react_text(self, text: str) -> dict:
        result = {}
        thought_m = re.search(r"Thought:\s*(.*?)(?=Action:|$)", text, re.DOTALL | re.IGNORECASE)
        action_m  = re.search(r"Action:\s*(\w[\w\s]*?)(?=ActionInput:|$|\n)", text, re.DOTALL | re.IGNORECASE)
        input_m   = re.search(r"ActionInput:\s*(.*?)(?=$)", text, re.DOTALL | re.IGNORECASE)

        result["thought"] = thought_m.group(1).strip() if thought_m else text.strip()
        result["action"]  = action_m.group(1).strip()  if action_m  else "FinalAnswer"

        if input_m:
            raw = input_m.group(1).strip()
            try:
                result["action_input"] = json.loads(raw)
            except json.JSONDecodeError:
                # Try to extract JSON object
                obj_m = re.search(r"\{.*\}", raw, re.DOTALL)
                if obj_m:
                    try:
                        result["action_input"] = json.loads(obj_m.group())
                    except Exception:
                        result["action_input"] = {"value": raw}
                else:
                    result["action_input"] = {"answer": raw}
        else:
            result["action_input"] = {}

        return result
