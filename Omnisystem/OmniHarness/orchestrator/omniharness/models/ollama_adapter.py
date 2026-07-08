"""
Ollama adapter — local models via the Ollama HTTP API.
Talks directly to http://localhost:11434 with no authentication.
"""

from __future__ import annotations

import logging
from typing import Any, AsyncIterator, Dict, List, Optional

import httpx

from omniharness.models.base import (
    ChatMessage,
    ChatRequest,
    ChatResponse,
    ModelAdapter,
    ModelInfo,
)

log = logging.getLogger(__name__)

_DEFAULT_BASE = "http://localhost:11434"


def _convert_messages(messages: List[ChatMessage]) -> List[Dict[str, Any]]:
    result: List[Dict[str, Any]] = []
    for msg in messages:
        result.append({"role": msg.role, "content": msg.content})
    return result


class OllamaAdapter(ModelAdapter):
    """Adapter for locally-running Ollama models."""

    provider_name = "ollama"

    def __init__(
        self,
        base_url: str = _DEFAULT_BASE,
        default_model: str = "llama3.2",
        timeout: float = 120.0,
    ) -> None:
        self._base = base_url.rstrip("/")
        self._default_model = default_model
        self._timeout = timeout
        self._http = httpx.AsyncClient(
            base_url=self._base,
            timeout=httpx.Timeout(self._timeout),
        )

    def _resolve_model(self, model_id: str) -> str:
        if "/" in model_id:
            _, model_id = model_id.split("/", 1)
        return model_id or self._default_model

    async def chat(self, request: ChatRequest) -> ChatResponse:
        t0 = self._now_ms()
        model = self._resolve_model(request.model_id)

        messages = _convert_messages(request.messages)
        if request.system:
            messages = [{"role": "system", "content": request.system}] + messages

        payload: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "stream": False,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens,
            },
        }

        try:
            resp = await self._http.post("/api/chat", json=payload)
            resp.raise_for_status()
        except httpx.HTTPError as exc:
            raise RuntimeError(f"Ollama request failed: {exc}") from exc

        data = resp.json()
        msg = data.get("message", {})
        content = msg.get("content", "")
        done_reason = data.get("done_reason", "stop")

        prompt_tokens = data.get("prompt_eval_count", 0)
        output_tokens = data.get("eval_count", 0)

        return ChatResponse(
            content=content,
            model_used=model,
            finish_reason=done_reason,
            input_tokens=prompt_tokens,
            output_tokens=output_tokens,
            latency_ms=self._now_ms() - t0,
        )

    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        import json as _json

        model = self._resolve_model(request.model_id)
        messages = _convert_messages(request.messages)
        if request.system:
            messages = [{"role": "system", "content": request.system}] + messages

        payload: Dict[str, Any] = {
            "model": model,
            "messages": messages,
            "stream": True,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens,
            },
        }

        try:
            async with self._http.stream("POST", "/api/chat", json=payload) as resp:
                resp.raise_for_status()
                async for line in resp.aiter_lines():
                    if not line.strip():
                        continue
                    try:
                        chunk = _json.loads(line)
                    except _json.JSONDecodeError:
                        continue
                    msg = chunk.get("message", {})
                    token = msg.get("content", "")
                    if token:
                        yield token
                    if chunk.get("done"):
                        break
        except httpx.HTTPError as exc:
            raise RuntimeError(f"Ollama stream error: {exc}") from exc

    async def health(self) -> bool:
        try:
            resp = await self._http.get("/api/tags", timeout=5.0)
            return resp.status_code == 200
        except Exception as exc:
            log.warning("Ollama health check failed: %s", exc)
            return False

    async def _fetch_local_models(self) -> List[ModelInfo]:
        """Query Ollama for installed models."""
        try:
            resp = await self._http.get("/api/tags", timeout=10.0)
            resp.raise_for_status()
            data = resp.json()
            models: List[ModelInfo] = []
            for m in data.get("models", []):
                name = m.get("name", "unknown")
                models.append(
                    ModelInfo(
                        id=name,
                        provider="ollama",
                        context_window=0,
                        description=f"Local Ollama model: {name}",
                    )
                )
            return models
        except Exception:
            return []

    # Cache of real installed models, refreshed by the sync list_models() below.
    _sync_cache: Optional[List[ModelInfo]] = None
    _sync_cache_ts: float = 0.0

    def list_models(self) -> List[ModelInfo]:
        """
        Synchronously enumerate the models actually installed in Ollama by
        querying /api/tags with a short blocking request (so the aggregate
        /api/models endpoint returns real local models, not a placeholder).
        Cached briefly so repeated calls are cheap; refreshes automatically so
        newly `ollama pull`-ed models appear without any manual reload.
        """
        import time
        import json as _json
        import urllib.request

        now = time.time()
        if self._sync_cache is not None and (now - self._sync_cache_ts) < 15.0:
            return self._sync_cache

        models: List[ModelInfo] = []
        try:
            with urllib.request.urlopen(self._base + "/api/tags", timeout=3) as resp:
                data = _json.loads(resp.read().decode())
            for m in data.get("models", []):
                name = m.get("name", "")
                if not name:
                    continue
                # Ollama models handle native tool-calling unreliably; expose as
                # text-protocol capable by default (override via toolMode).
                models.append(ModelInfo(
                    id=name,
                    provider="ollama",
                    context_window=0,
                    supports_tools=False,
                    supports_vision=("llava" in name or "vision" in name),
                    description=f"Local Ollama model: {name}",
                ))
        except Exception as exc:  # noqa: BLE001
            log.debug("Ollama sync enumeration failed at %s/api/tags: %s", self._base, exc)

        self._sync_cache = models
        self._sync_cache_ts = now
        return models
