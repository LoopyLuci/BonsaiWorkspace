"""
Mistral adapter — uses the OpenAI-compatible REST endpoint at api.mistral.ai.
Bearer auth, streaming, tool use.
"""

from __future__ import annotations

import json
import logging
from typing import Any, AsyncIterator, Dict, List, Optional

import httpx

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

_BASE_URL = "https://api.mistral.ai/v1"

_MODELS: List[ModelInfo] = [
    ModelInfo(id="mistral-large-latest",    provider="mistral", context_window=128_000, supports_tools=True,  description="Mistral Large — most capable"),
    ModelInfo(id="mistral-small-latest",    provider="mistral", context_window=32_000,  supports_tools=True,  description="Mistral Small — fast & cheap"),
    ModelInfo(id="codestral-latest",        provider="mistral", context_window=32_000,  supports_tools=True,  description="Codestral — code specialist"),
    ModelInfo(id="mistral-medium-latest",   provider="mistral", context_window=32_000,  supports_tools=True,  description="Mistral Medium"),
    ModelInfo(id="open-mistral-7b",         provider="mistral", context_window=32_000,  supports_tools=False, description="Open Mistral 7B"),
    ModelInfo(id="open-mixtral-8x7b",       provider="mistral", context_window=32_000,  supports_tools=False, description="Open Mixtral 8x7B"),
    ModelInfo(id="open-mixtral-8x22b",      provider="mistral", context_window=64_000,  supports_tools=True,  description="Open Mixtral 8x22B"),
]

_MODEL_IDS = {m.id for m in _MODELS}


def _convert_tool_def(t: ToolDef) -> Dict[str, Any]:
    return {
        "type": "function",
        "function": {
            "name": t.name,
            "description": t.description,
            "parameters": t.parameters,
        },
    }


def _convert_messages(
    messages: List[ChatMessage], system: Optional[str]
) -> List[Dict[str, Any]]:
    result: List[Dict[str, Any]] = []
    if system:
        result.append({"role": "system", "content": system})
    for msg in messages:
        if msg.role == "system":
            # Already handled or merged
            continue
        d: Dict[str, Any] = {"role": msg.role, "content": msg.content}
        if msg.role == "tool" and msg.tool_call_id:
            d["tool_call_id"] = msg.tool_call_id
        if msg.role == "assistant" and msg.tool_calls:
            d["tool_calls"] = [
                {
                    "id": tc.id,
                    "type": "function",
                    "function": {"name": tc.name, "arguments": json.dumps(tc.arguments or {})},
                }
                for tc in msg.tool_calls
            ]
            if not msg.content:
                d["content"] = None
        result.append(d)
    return result


def _parse_tool_calls(raw) -> List[ToolCall]:
    if not raw:
        return []
    result: List[ToolCall] = []
    for tc in raw:
        try:
            args = json.loads(tc["function"]["arguments"])
        except (json.JSONDecodeError, KeyError):
            args = {}
        result.append(
            ToolCall(
                id=tc.get("id", ""),
                name=tc["function"]["name"],
                arguments=args,
            )
        )
    return result


class MistralAdapter(ModelAdapter):
    """Adapter for Mistral AI via their OpenAI-compatible endpoint."""

    provider_name = "mistral"

    def __init__(self, api_key: str, default_model: str = "mistral-large-latest") -> None:
        self._api_key = api_key
        self._default_model = default_model
        self._http = httpx.AsyncClient(
            base_url=_BASE_URL,
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            },
            timeout=httpx.Timeout(120.0),
        )

    def _resolve_model(self, model_id: str) -> str:
        if "/" in model_id:
            _, model_id = model_id.split("/", 1)
        return model_id if model_id in _MODEL_IDS else self._default_model

    def _build_payload(
        self, request: ChatRequest, model: str, stream: bool = False
    ) -> Dict[str, Any]:
        system = request.system or None
        # Extract inline system messages
        for msg in request.messages:
            if msg.role == "system":
                system = (system + "\n\n" + msg.content) if system else msg.content

        messages = _convert_messages(request.messages, system)
        payload: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": stream,
        }
        if request.tools:
            payload["tools"] = [_convert_tool_def(t) for t in request.tools]
            payload["tool_choice"] = "auto"
        return payload

    async def chat(self, request: ChatRequest) -> ChatResponse:
        t0 = self._now_ms()
        model = self._resolve_model(request.model_id)
        payload = self._build_payload(request, model, stream=False)

        try:
            resp = await self._http.post("/chat/completions", json=payload)
            resp.raise_for_status()
        except httpx.HTTPError as exc:
            raise RuntimeError(f"Mistral request failed: {exc}") from exc

        data = resp.json()
        choice = data["choices"][0]
        msg = choice["message"]
        text = msg.get("content") or ""
        tool_calls = _parse_tool_calls(msg.get("tool_calls"))
        usage = data.get("usage", {})

        return ChatResponse(
            content=text,
            model_used=model,
            finish_reason=choice.get("finish_reason", "stop"),
            input_tokens=usage.get("prompt_tokens", 0),
            output_tokens=usage.get("completion_tokens", 0),
            tool_calls=tool_calls,
            latency_ms=self._now_ms() - t0,
        )

    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        model = self._resolve_model(request.model_id)
        payload = self._build_payload(request, model, stream=True)

        try:
            async with self._http.stream("POST", "/chat/completions", json=payload) as resp:
                resp.raise_for_status()
                async for line in resp.aiter_lines():
                    if not line.startswith("data:"):
                        continue
                    data_str = line[5:].strip()
                    if data_str == "[DONE]":
                        break
                    try:
                        chunk = json.loads(data_str)
                    except json.JSONDecodeError:
                        continue
                    delta = chunk["choices"][0].get("delta", {})
                    token = delta.get("content") or ""
                    if token:
                        yield token
        except httpx.HTTPError as exc:
            raise RuntimeError(f"Mistral stream error: {exc}") from exc

    async def health(self) -> bool:
        try:
            resp = await self._http.get("/models", timeout=5.0)
            return resp.status_code == 200
        except Exception as exc:
            log.warning("Mistral health check failed: %s", exc)
            return False

    def list_models(self) -> List[ModelInfo]:
        return list(_MODELS)
