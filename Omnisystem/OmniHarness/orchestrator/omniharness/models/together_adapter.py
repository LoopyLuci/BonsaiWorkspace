"""
Together AI adapter — OpenAI-compatible endpoint.
Supports Llama 3, Mixtral, Qwen, DBRX, and hundreds more open models.
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

_BASE_URL = "https://api.together.xyz/v1"

_MODELS: List[ModelInfo] = [
    ModelInfo(id="meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo", provider="together", context_window=130_000, supports_tools=True,  description="Llama 3.1 405B — most capable open model"),
    ModelInfo(id="meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo",  provider="together", context_window=130_000, supports_tools=True,  description="Llama 3.1 70B — fast and capable"),
    ModelInfo(id="meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo",   provider="together", context_window=130_000, supports_tools=True,  description="Llama 3.1 8B — ultra-fast"),
    ModelInfo(id="meta-llama/Llama-3.3-70B-Instruct-Turbo",       provider="together", context_window=130_000, supports_tools=True,  description="Llama 3.3 70B"),
    ModelInfo(id="mistralai/Mixtral-8x22B-Instruct-v0.1",         provider="together", context_window=65_536,  supports_tools=False, description="Mixtral 8x22B"),
    ModelInfo(id="mistralai/Mistral-7B-Instruct-v0.3",            provider="together", context_window=32_768,  supports_tools=False, description="Mistral 7B"),
    ModelInfo(id="Qwen/Qwen2.5-72B-Instruct-Turbo",               provider="together", context_window=32_768,  supports_tools=True,  description="Qwen 2.5 72B"),
    ModelInfo(id="deepseek-ai/DeepSeek-V3",                       provider="together", context_window=65_536,  supports_tools=False, description="DeepSeek V3"),
    ModelInfo(id="google/gemma-2-27b-it",                         provider="together", context_window=8_192,   supports_tools=False, description="Gemma 2 27B"),
]

_MODEL_IDS = {m.id for m in _MODELS}


class TogetherAdapter(ModelAdapter):
    """Adapter for Together AI — wraps their OpenAI-compatible API."""

    provider_name = "together"

    def __init__(self, api_key: str, default_model: str = "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo") -> None:
        self._inner = OpenAIAdapter(api_key=api_key, base_url=_BASE_URL, default_model=default_model)
        self._default_model = default_model

    def _resolve_model(self, model_id: str) -> str:
        if model_id.startswith("together/"):
            model_id = model_id[len("together/"):]
        return model_id if model_id in _MODEL_IDS else self._default_model

    async def chat(self, request: ChatRequest) -> ChatResponse:
        req = request.model_copy(update={"model_id": self._resolve_model(request.model_id)})
        resp = await self._inner.chat(req)
        return resp

    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        req = request.model_copy(update={"model_id": self._resolve_model(request.model_id)})
        async for chunk in self._inner.stream(req):
            yield chunk

    async def health(self) -> bool:
        try:
            return await self._inner.health()
        except Exception as exc:
            log.warning("Together health check failed: %s", exc)
            return False

    def list_models(self) -> List[ModelInfo]:
        return list(_MODELS)
