"""
Fireworks AI adapter — OpenAI-compatible endpoint.
Fast inference for Llama, Mixtral, Qwen, and FireFunction models.
"""

from __future__ import annotations

import logging
from typing import AsyncIterator, List

from omniharness.models.base import (
    ChatRequest,
    ChatResponse,
    ModelAdapter,
    ModelInfo,
)
from omniharness.models.openai_adapter import OpenAIAdapter

log = logging.getLogger(__name__)

_BASE_URL = "https://api.fireworks.ai/inference/v1"

_MODELS: List[ModelInfo] = [
    ModelInfo(id="accounts/fireworks/models/llama-v3p1-405b-instruct",   provider="fireworks", context_window=131_072, supports_tools=True,  description="Llama 3.1 405B — highest quality"),
    ModelInfo(id="accounts/fireworks/models/llama-v3p1-70b-instruct",    provider="fireworks", context_window=131_072, supports_tools=True,  description="Llama 3.1 70B"),
    ModelInfo(id="accounts/fireworks/models/llama-v3p1-8b-instruct",     provider="fireworks", context_window=131_072, supports_tools=True,  description="Llama 3.1 8B — fastest"),
    ModelInfo(id="accounts/fireworks/models/llama-v3p3-70b-instruct",    provider="fireworks", context_window=131_072, supports_tools=True,  description="Llama 3.3 70B"),
    ModelInfo(id="accounts/fireworks/models/mixtral-8x22b-instruct",     provider="fireworks", context_window=65_536,  supports_tools=False, description="Mixtral 8x22B"),
    ModelInfo(id="accounts/fireworks/models/qwen2p5-72b-instruct",       provider="fireworks", context_window=32_768,  supports_tools=True,  description="Qwen 2.5 72B"),
    ModelInfo(id="accounts/fireworks/models/firefunction-v2",            provider="fireworks", context_window=32_768,  supports_tools=True,  description="FireFunction v2 — optimized for tool use"),
    ModelInfo(id="accounts/fireworks/models/deepseek-v3",               provider="fireworks", context_window=65_536,  supports_tools=False, description="DeepSeek V3"),
    ModelInfo(id="accounts/fireworks/models/gemma2-9b-it",              provider="fireworks", context_window=8_192,   supports_tools=False, description="Gemma 2 9B"),
]

_MODEL_IDS = {m.id for m in _MODELS}


class FireworksAdapter(ModelAdapter):
    """Adapter for Fireworks AI — wraps their OpenAI-compatible API."""

    provider_name = "fireworks"

    def __init__(self, api_key: str, default_model: str = "accounts/fireworks/models/llama-v3p1-70b-instruct") -> None:
        self._inner = OpenAIAdapter(api_key=api_key, base_url=_BASE_URL, default_model=default_model)
        self._default_model = default_model

    def _resolve_model(self, model_id: str) -> str:
        if model_id.startswith("fireworks/"):
            model_id = model_id[len("fireworks/"):]
        return model_id if model_id in _MODEL_IDS else self._default_model

    async def chat(self, request: ChatRequest) -> ChatResponse:
        req = request.model_copy(update={"model_id": self._resolve_model(request.model_id)})
        return await self._inner.chat(req)

    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        req = request.model_copy(update={"model_id": self._resolve_model(request.model_id)})
        async for chunk in self._inner.stream(req):
            yield chunk

    async def health(self) -> bool:
        try:
            return await self._inner.health()
        except Exception as exc:
            log.warning("Fireworks health check failed: %s", exc)
            return False

    def list_models(self) -> List[ModelInfo]:
        return list(_MODELS)
