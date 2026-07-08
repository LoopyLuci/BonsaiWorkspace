"""
Cohere adapter — Command-R family models.
Uses the cohere v5 SDK; handles chat_history format and tool use.
"""

from __future__ import annotations

import logging
from typing import Any, AsyncIterator, Dict, List, Optional

import cohere
from cohere import AsyncClientV2

from omniharness.models.base import (
    ChatMessage,
    ChatRequest,
    ChatResponse,
    ModelAdapter,
    ModelInfo,
    ToolCall,
    ToolDef,
)

log = logging.getLogger(__name__)

_MODELS: List[ModelInfo] = [
    ModelInfo(id="command-r-plus",         provider="cohere", context_window=128_000, supports_tools=True,  description="Cohere Command R+"),
    ModelInfo(id="command-r",              provider="cohere", context_window=128_000, supports_tools=True,  description="Cohere Command R"),
    ModelInfo(id="command-a-03-2025",      provider="cohere", context_window=256_000, supports_tools=True,  description="Cohere Command A (2025)"),
    ModelInfo(id="command-r7b-12-2024",    provider="cohere", context_window=128_000, supports_tools=True,  description="Cohere Command R 7B"),
    ModelInfo(id="command-nightly",        provider="cohere", context_window=128_000, supports_tools=False, description="Cohere Command Nightly"),
]

_MODEL_IDS = {m.id for m in _MODELS}


def _split_messages(
    messages: List[ChatMessage],
) -> tuple[Optional[str], List[Dict[str, Any]], str]:
    """
    Cohere v2 API uses a flat messages list like OpenAI.
    Returns (system_prompt, chat_history_without_last, last_user_message).
    """
    system: Optional[str] = None
    history: List[Dict[str, Any]] = []
    last_user = ""

    for msg in messages:
        if msg.role == "system":
            system = (system + "\n\n" + msg.content) if system else msg.content
        elif msg.role == "tool":
            history.append({"role": "tool", "content": msg.content})
        else:
            history.append({"role": msg.role, "content": msg.content})

    # Pop the last user message to pass as the `message` param
    if history and history[-1]["role"] == "user":
        last_user = history.pop()["content"]

    return system, history, last_user


def _convert_tool_def(t: ToolDef) -> Dict[str, Any]:
    """Convert ToolDef to Cohere v2 tool format."""
    props = t.parameters.get("properties", {})
    required = t.parameters.get("required", [])
    params: Dict[str, Any] = {}
    for name, schema in props.items():
        params[name] = {
            "description": schema.get("description", ""),
            "type": schema.get("type", "string"),
            "required": name in required,
        }
    return {
        "name": t.name,
        "description": t.description,
        "parameter_definitions": params,
    }


def _parse_tool_calls(tool_calls) -> List[ToolCall]:
    if not tool_calls:
        return []
    result: List[ToolCall] = []
    for tc in tool_calls:
        result.append(
            ToolCall(
                id=getattr(tc, "id", tc.name),
                name=tc.name,
                arguments=tc.parameters if isinstance(tc.parameters, dict) else {},
            )
        )
    return result


class CohereAdapter(ModelAdapter):
    """Adapter for Cohere Command-R family."""

    provider_name = "cohere"

    def __init__(self, api_key: str, default_model: str = "command-r-plus") -> None:
        self._client = AsyncClientV2(api_key=api_key)
        self._default_model = default_model

    def _resolve_model(self, model_id: str) -> str:
        if "/" in model_id:
            _, model_id = model_id.split("/", 1)
        return model_id if model_id in _MODEL_IDS else self._default_model

    async def chat(self, request: ChatRequest) -> ChatResponse:
        t0 = self._now_ms()
        model = self._resolve_model(request.model_id)
        system, history, last_user = _split_messages(request.messages)

        # Build full messages list for v2 API
        msgs: List[Dict[str, Any]] = []
        if system or request.system:
            msgs.append({"role": "system", "content": request.system or system or ""})
        msgs.extend(history)
        if last_user:
            msgs.append({"role": "user", "content": last_user})

        kwargs: Dict[str, Any] = {
            "model": model,
            "messages": msgs,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        }

        if request.tools:
            kwargs["tools"] = [_convert_tool_def(t) for t in request.tools]

        try:
            response = await self._client.chat(**kwargs)
        except cohere.CohereAPIError as exc:
            raise RuntimeError(f"Cohere API error: {exc}") from exc

        message = response.message
        text = ""
        tool_calls: List[ToolCall] = []

        for block in (message.content or []):
            if getattr(block, "type", None) == "text":
                text += block.text
            elif getattr(block, "type", None) == "tool_use":
                tool_calls.append(
                    ToolCall(
                        id=getattr(block, "id", block.name),
                        name=block.name,
                        arguments=block.input if isinstance(block.input, dict) else {},
                    )
                )

        usage = response.usage
        input_tokens = getattr(usage, "billed_units", None)
        in_tok = out_tok = 0
        if input_tokens:
            in_tok = getattr(input_tokens, "input_tokens", 0) or 0
            out_tok = getattr(input_tokens, "output_tokens", 0) or 0

        return ChatResponse(
            content=text,
            model_used=model,
            finish_reason=response.finish_reason or "COMPLETE",
            input_tokens=in_tok,
            output_tokens=out_tok,
            tool_calls=tool_calls,
            latency_ms=self._now_ms() - t0,
        )

    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        model = self._resolve_model(request.model_id)
        system, history, last_user = _split_messages(request.messages)

        msgs: List[Dict[str, Any]] = []
        if system or request.system:
            msgs.append({"role": "system", "content": request.system or system or ""})
        msgs.extend(history)
        if last_user:
            msgs.append({"role": "user", "content": last_user})

        kwargs: Dict[str, Any] = {
            "model": model,
            "messages": msgs,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        }

        try:
            async for event in self._client.chat_stream(**kwargs):
                if hasattr(event, "delta") and hasattr(event.delta, "message"):
                    block = event.delta.message
                    if hasattr(block, "content") and block.content:
                        for part in block.content:
                            if getattr(part, "type", None) == "text":
                                yield part.text
        except cohere.CohereAPIError as exc:
            raise RuntimeError(f"Cohere stream error: {exc}") from exc

    async def health(self) -> bool:
        try:
            # Lightweight check — list models
            await self._client.models.list()
            return True
        except Exception as exc:
            log.warning("Cohere health check failed: %s", exc)
            return False

    def list_models(self) -> List[ModelInfo]:
        return list(_MODELS)
