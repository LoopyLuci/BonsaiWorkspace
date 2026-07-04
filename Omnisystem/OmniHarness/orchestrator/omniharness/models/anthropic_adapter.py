"""
Anthropic adapter — Claude family models.
Handles system prompts, tool use, streaming, and token accounting.
"""

from __future__ import annotations

import json
import logging
from typing import Any, AsyncIterator, Dict, List, Optional

import anthropic

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

# ---------------------------------------------------------------------------
# Known Claude models
# ---------------------------------------------------------------------------

_MODELS: List[ModelInfo] = [
    ModelInfo(id="claude-opus-4-8",           provider="anthropic", context_window=200_000, supports_tools=True,  supports_vision=True,  description="Most powerful Claude"),
    ModelInfo(id="claude-sonnet-4-6",          provider="anthropic", context_window=200_000, supports_tools=True,  supports_vision=True,  description="Balanced Claude (default)"),
    ModelInfo(id="claude-sonnet-5",            provider="anthropic", context_window=200_000, supports_tools=True,  supports_vision=True,  description="Next-gen Sonnet"),
    ModelInfo(id="claude-haiku-4-5-20251001",  provider="anthropic", context_window=200_000, supports_tools=True,  supports_vision=True,  description="Fast, cheap Claude"),
    ModelInfo(id="claude-fable-5",             provider="anthropic", context_window=200_000, supports_tools=True,  supports_vision=False, description="Claude Fable"),
    ModelInfo(id="claude-3-5-sonnet-20241022", provider="anthropic", context_window=200_000, supports_tools=True,  supports_vision=True,  description="Claude 3.5 Sonnet"),
    ModelInfo(id="claude-3-5-haiku-20241022",  provider="anthropic", context_window=200_000, supports_tools=True,  supports_vision=True,  description="Claude 3.5 Haiku"),
    ModelInfo(id="claude-3-opus-20240229",     provider="anthropic", context_window=200_000, supports_tools=True,  supports_vision=True,  description="Claude 3 Opus"),
]

_MODEL_IDS = {m.id for m in _MODELS}


def _convert_tool_def(t: ToolDef) -> Dict[str, Any]:
    """Convert our ToolDef to Anthropic tool format."""
    return {
        "name": t.name,
        "description": t.description,
        "input_schema": t.parameters,
    }


def _extract_messages(
    messages: List[ChatMessage],
) -> tuple[Optional[str], List[Dict[str, Any]]]:
    """Split system prompt out; convert the rest to Anthropic message dicts."""
    system: Optional[str] = None
    converted: List[Dict[str, Any]] = []

    for msg in messages:
        if msg.role == "system":
            # Anthropic takes a single system string — concatenate multiples
            system = (system + "\n\n" + msg.content) if system else msg.content
            continue

        if msg.role == "tool":
            # Tool result message
            converted.append(
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id or "unknown",
                            "content": msg.content,
                        }
                    ],
                }
            )
        elif msg.role == "assistant" and msg.tool_calls:
            # Reconstruct the assistant turn as text + tool_use blocks so the
            # following tool_result references a tool_use_id present in history.
            blocks: List[Dict[str, Any]] = []
            if msg.content:
                blocks.append({"type": "text", "text": msg.content})
            for tc in msg.tool_calls:
                blocks.append(
                    {
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.name,
                        "input": tc.arguments or {},
                    }
                )
            converted.append({"role": "assistant", "content": blocks})
        else:
            converted.append({"role": msg.role, "content": msg.content})

    return system, converted


def _parse_tool_calls(content_blocks: list) -> List[ToolCall]:
    """Pull tool_use blocks out of Anthropic content."""
    calls: List[ToolCall] = []
    for block in content_blocks:
        if getattr(block, "type", None) == "tool_use":
            calls.append(
                ToolCall(
                    id=block.id,
                    name=block.name,
                    arguments=block.input if isinstance(block.input, dict) else {},
                )
            )
    return calls


def _text_from_blocks(content_blocks: list) -> str:
    """Concatenate all text blocks into a single string."""
    parts: List[str] = []
    for block in content_blocks:
        if getattr(block, "type", None) == "text":
            parts.append(block.text)
    return "".join(parts)


class AnthropicAdapter(ModelAdapter):
    """Adapter for Anthropic's Claude family."""

    provider_name = "anthropic"

    def __init__(self, api_key: str, default_model: str = "claude-sonnet-4-6") -> None:
        self._client = anthropic.AsyncAnthropic(api_key=api_key)
        self._default_model = default_model

    def _resolve_model(self, model_id: str) -> str:
        """Strip provider prefix if present."""
        if "/" in model_id:
            _, model_id = model_id.split("/", 1)
        return model_id if model_id in _MODEL_IDS else self._default_model

    async def chat(self, request: ChatRequest) -> ChatResponse:
        t0 = self._now_ms()
        model = self._resolve_model(request.model_id)

        system_from_field, messages = _extract_messages(request.messages)
        system = request.system or system_from_field

        kwargs: Dict[str, Any] = {
            "model": model,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "messages": messages,
        }
        if system:
            kwargs["system"] = system
        if request.tools:
            kwargs["tools"] = [_convert_tool_def(t) for t in request.tools]

        try:
            response = await self._client.messages.create(**kwargs)
        except anthropic.APIError as exc:
            raise RuntimeError(f"Anthropic API error: {exc}") from exc

        content_blocks = response.content
        text = _text_from_blocks(content_blocks)
        tool_calls = _parse_tool_calls(content_blocks)

        return ChatResponse(
            content=text,
            model_used=model,
            finish_reason=response.stop_reason or "stop",
            input_tokens=response.usage.input_tokens,
            output_tokens=response.usage.output_tokens,
            tool_calls=tool_calls,
            latency_ms=self._now_ms() - t0,
        )

    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        model = self._resolve_model(request.model_id)
        system_from_field, messages = _extract_messages(request.messages)
        system = request.system or system_from_field

        kwargs: Dict[str, Any] = {
            "model": model,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "messages": messages,
        }
        if system:
            kwargs["system"] = system
        if request.tools:
            kwargs["tools"] = [_convert_tool_def(t) for t in request.tools]

        try:
            async with self._client.messages.stream(**kwargs) as stream:
                async for text in stream.text_stream:
                    yield text
        except anthropic.APIError as exc:
            raise RuntimeError(f"Anthropic stream error: {exc}") from exc

    async def health(self) -> bool:
        try:
            # Attempt a minimal API call — list models
            await self._client.models.list()
            return True
        except Exception as exc:
            log.warning("Anthropic health check failed: %s", exc)
            return False

    def list_models(self) -> List[ModelInfo]:
        return list(_MODELS)
