"""
OpenRouter adapter — unified gateway to 200+ models.
Model IDs are passed through directly: "openai/gpt-4o", "anthropic/claude-opus-4", etc.
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

_BASE_URL = "https://openrouter.ai/api/v1"

# Well-known models exposed for discoverability — OpenRouter supports many more
_KNOWN_MODELS: List[ModelInfo] = [
    ModelInfo(id="openai/gpt-4o",                     provider="openrouter", context_window=128_000, supports_tools=True,  description="OpenAI GPT-4o via OpenRouter"),
    ModelInfo(id="openai/gpt-4o-mini",                provider="openrouter", context_window=128_000, supports_tools=True,  description="OpenAI GPT-4o Mini via OpenRouter"),
    ModelInfo(id="anthropic/claude-opus-4",           provider="openrouter", context_window=200_000, supports_tools=True,  description="Anthropic Claude Opus 4 via OpenRouter"),
    ModelInfo(id="anthropic/claude-sonnet-4-6",       provider="openrouter", context_window=200_000, supports_tools=True,  description="Anthropic Claude Sonnet 4.6 via OpenRouter"),
    ModelInfo(id="google/gemini-2.0-flash",           provider="openrouter", context_window=1_000_000,supports_tools=True, description="Gemini 2.0 Flash via OpenRouter"),
    ModelInfo(id="meta-llama/llama-3.3-70b-instruct", provider="openrouter", context_window=128_000, supports_tools=True,  description="LLaMA 3.3 70B via OpenRouter"),
    ModelInfo(id="mistralai/mistral-large",           provider="openrouter", context_window=128_000, supports_tools=True,  description="Mistral Large via OpenRouter"),
    ModelInfo(id="cohere/command-r-plus",             provider="openrouter", context_window=128_000, supports_tools=True,  description="Cohere Command R+ via OpenRouter"),
    ModelInfo(id="deepseek/deepseek-r1",              provider="openrouter", context_window=64_000,  supports_tools=False, description="DeepSeek R1 via OpenRouter"),
    ModelInfo(id="x-ai/grok-3",                      provider="openrouter", context_window=128_000, supports_tools=True,  description="xAI Grok-3 via OpenRouter"),
]


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
        except (json.JSONDecodeError, KeyError, TypeError):
            args = {}
        result.append(
            ToolCall(
                id=tc.get("id", ""),
                name=tc["function"]["name"],
                arguments=args,
            )
        )
    return result


class OpenRouterAdapter(ModelAdapter):
    """Adapter for OpenRouter — passthrough to 200+ models."""

    provider_name = "openrouter"

    def __init__(
        self,
        api_key: str,
        site_url: str = "https://omnisystem.dev",
        app_name: str = "OmniHarness",
        default_model: str = "openai/gpt-4o",
    ) -> None:
        self._api_key = api_key
        self._default_model = default_model
        self._http = httpx.AsyncClient(
            base_url=_BASE_URL,
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
                "HTTP-Referer": site_url,
                "X-Title": app_name,
            },
            timeout=httpx.Timeout(120.0),
        )

    def _resolve_model(self, model_id: str) -> str:
        # OpenRouter model IDs already include provider prefix (e.g. "openai/gpt-4o")
        # If no slash, use as-is or fall back to default
        return model_id if model_id else self._default_model

    def _build_payload(
        self,
        request: ChatRequest,
        model: str,
        stream: bool = False,
    ) -> Dict[str, Any]:
        system = request.system
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
            raise RuntimeError(f"OpenRouter request failed: {exc}") from exc

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
                    choices = chunk.get("choices", [])
                    if not choices:
                        continue
                    delta = choices[0].get("delta", {})
                    token = delta.get("content") or ""
                    if token:
                        yield token
        except httpx.HTTPError as exc:
            raise RuntimeError(f"OpenRouter stream error: {exc}") from exc

    async def health(self) -> bool:
        try:
            resp = await self._http.get("/models", timeout=5.0)
            return resp.status_code == 200
        except Exception as exc:
            log.warning("OpenRouter health check failed: %s", exc)
            return False

    def list_models(self) -> List[ModelInfo]:
        return list(_KNOWN_MODELS)
