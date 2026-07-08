"""
OpenAI adapter — GPT-4o, o1, o3 family models.
Full tool / function-call support, streaming, token accounting.
"""

from __future__ import annotations

import json
import logging
from typing import Any, AsyncIterator, Dict, List, Optional

import openai

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
# Known OpenAI models
# ---------------------------------------------------------------------------

_MODELS: List[ModelInfo] = [
    ModelInfo(id="gpt-4o",              provider="openai", context_window=128_000, supports_tools=True, supports_vision=True,  description="GPT-4o flagship"),
    ModelInfo(id="gpt-4o-mini",         provider="openai", context_window=128_000, supports_tools=True, supports_vision=True,  description="GPT-4o Mini, fast & cheap"),
    ModelInfo(id="gpt-4-turbo",         provider="openai", context_window=128_000, supports_tools=True, supports_vision=True,  description="GPT-4 Turbo"),
    ModelInfo(id="gpt-4",               provider="openai", context_window=8_192,   supports_tools=True, supports_vision=False, description="GPT-4 classic"),
    ModelInfo(id="gpt-3.5-turbo",       provider="openai", context_window=16_385,  supports_tools=True, supports_vision=False, description="GPT-3.5 Turbo"),
    ModelInfo(id="o1",                  provider="openai", context_window=200_000, supports_tools=True, supports_vision=True,  description="OpenAI o1 reasoning"),
    ModelInfo(id="o1-mini",             provider="openai", context_window=128_000, supports_tools=False,supports_vision=False, description="o1 Mini"),
    ModelInfo(id="o3-mini",             provider="openai", context_window=200_000, supports_tools=True, supports_vision=False, description="o3 Mini"),
    ModelInfo(id="o4-mini",             provider="openai", context_window=200_000, supports_tools=True, supports_vision=True,  description="o4 Mini"),
]

_MODEL_IDS = {m.id for m in _MODELS}

# o1 family does not support temperature / system messages in the same way
_NO_SYSTEM_MODELS = {"o1-mini", "o1-preview"}
_NO_TEMP_MODELS = {"o1", "o1-mini", "o1-preview", "o3-mini", "o4-mini"}


def _convert_tool_def(t: ToolDef) -> Dict[str, Any]:
    return {
        "type": "function",
        "function": {
            "name": t.name,
            "description": t.description,
            "parameters": t.parameters,
        },
    }


def _convert_messages(messages: List[ChatMessage]) -> List[Dict[str, Any]]:
    converted: List[Dict[str, Any]] = []
    for msg in messages:
        d: Dict[str, Any] = {"role": msg.role, "content": msg.content}
        if msg.name:
            d["name"] = msg.name
        if msg.role == "tool" and msg.tool_call_id:
            d["tool_call_id"] = msg.tool_call_id
        # Preserve an assistant turn's tool calls so a following tool message is
        # accepted by the API (OpenAI requires the matching tool_calls to precede
        # any tool-role message).
        if msg.role == "assistant" and msg.tool_calls:
            d["tool_calls"] = [
                {
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": json.dumps(tc.arguments or {}),
                    },
                }
                for tc in msg.tool_calls
            ]
            # OpenAI expects content to be null (not "") when tool_calls are present.
            if not msg.content:
                d["content"] = None
        converted.append(d)
    return converted


def _parse_tool_calls(raw_calls) -> List[ToolCall]:
    if not raw_calls:
        return []
    result: List[ToolCall] = []
    for tc in raw_calls:
        try:
            args = json.loads(tc.function.arguments) if tc.function.arguments else {}
        except json.JSONDecodeError:
            args = {"_raw": tc.function.arguments}
        result.append(ToolCall(id=tc.id, name=tc.function.name, arguments=args))
    return result


class OpenAIAdapter(ModelAdapter):
    """Adapter for OpenAI's GPT and o-series models."""

    provider_name = "openai"

    def __init__(
        self,
        api_key: str,
        base_url: Optional[str] = None,
        default_model: str = "gpt-4o",
    ) -> None:
        self._client = openai.AsyncOpenAI(
            api_key=api_key,
            base_url=base_url,
        )
        self._default_model = default_model

    def _resolve_model(self, model_id: str) -> str:
        if "/" in model_id:
            _, model_id = model_id.split("/", 1)
        return model_id if model_id in _MODEL_IDS else self._default_model

    def _build_params(
        self, request: ChatRequest, model: str
    ) -> Dict[str, Any]:
        messages = _convert_messages(request.messages)

        # Inject system as leading message if provided and model supports it
        if request.system and model not in _NO_SYSTEM_MODELS:
            messages = [{"role": "system", "content": request.system}] + messages

        params: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
        }

        if model not in _NO_TEMP_MODELS:
            params["temperature"] = request.temperature

        if request.tools:
            params["tools"] = [_convert_tool_def(t) for t in request.tools]
            params["tool_choice"] = "auto"

        return params

    async def chat(self, request: ChatRequest) -> ChatResponse:
        t0 = self._now_ms()
        model = self._resolve_model(request.model_id)
        params = self._build_params(request, model)

        try:
            response = await self._client.chat.completions.create(**params)
        except openai.OpenAIError as exc:
            raise RuntimeError(f"OpenAI API error: {exc}") from exc

        choice = response.choices[0]
        msg = choice.message
        text = msg.content or ""
        tool_calls = _parse_tool_calls(msg.tool_calls)
        usage = response.usage

        return ChatResponse(
            content=text,
            model_used=model,
            finish_reason=choice.finish_reason or "stop",
            input_tokens=usage.prompt_tokens if usage else 0,
            output_tokens=usage.completion_tokens if usage else 0,
            tool_calls=tool_calls,
            latency_ms=self._now_ms() - t0,
        )

    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        model = self._resolve_model(request.model_id)
        params = self._build_params(request, model)
        params["stream"] = True

        try:
            async with await self._client.chat.completions.create(**params) as stream:
                async for chunk in stream:
                    delta = chunk.choices[0].delta if chunk.choices else None
                    if delta and delta.content:
                        yield delta.content
        except openai.OpenAIError as exc:
            raise RuntimeError(f"OpenAI stream error: {exc}") from exc

    async def health(self) -> bool:
        try:
            await self._client.models.list()
            return True
        except Exception as exc:
            log.warning("OpenAI health check failed: %s", exc)
            return False

    def list_models(self) -> List[ModelInfo]:
        return list(_MODELS)
