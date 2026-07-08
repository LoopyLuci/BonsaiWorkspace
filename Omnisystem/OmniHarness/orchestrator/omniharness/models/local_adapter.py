"""
Local OpenAI-compatible adapter — for GGUF/local models served by llama.cpp
(`llama-server`), LM Studio, Jan, vLLM, or any server that speaks the OpenAI
Chat Completions API. Model ids are passed through unchanged and discovered from
the server's /models endpoint.

Point it at your server with LOCAL_OPENAI_BASE_URL, e.g.:
  llama-server -m D:\\Models\\general\\gemma-4-31B-it-UD-Q2_K_XL\\...gguf --port 8081
  LOCAL_OPENAI_BASE_URL=http://localhost:8081/v1
Then use models as `local/<model-name>`.
"""
from __future__ import annotations

import json
import logging
import urllib.request
from typing import List, Optional

from omniharness.models.base import ModelInfo
from omniharness.models.openai_adapter import OpenAIAdapter

log = logging.getLogger(__name__)


class LocalOpenAIAdapter(OpenAIAdapter):
    """OpenAI-compatible adapter for a local inference server."""

    provider_name = "local"

    def __init__(self, base_url: str, api_key: str = "not-needed") -> None:
        super().__init__(api_key=api_key, base_url=base_url, default_model="")
        self._base_url = base_url.rstrip("/")
        self._cached: Optional[List[ModelInfo]] = None

    def _resolve_model(self, model_id: str) -> str:
        # Pass any model id straight through (strip a leading "local/" prefix).
        if "/" in model_id:
            _, model_id = model_id.split("/", 1)
        return model_id

    def list_models(self) -> List[ModelInfo]:
        if self._cached is not None:
            return self._cached
        models: List[ModelInfo] = []
        try:
            with urllib.request.urlopen(self._base_url + "/models", timeout=3) as resp:
                data = json.loads(resp.read().decode())
            for m in data.get("data", []):
                mid = m.get("id")
                if mid:
                    models.append(ModelInfo(
                        id=mid, provider="local", context_window=0,
                        # Default off: small quantized local models handle native
                        # tool-calling unreliably. Force native via toolMode if desired.
                        supports_tools=False, supports_vision=False,
                        description="Local (OpenAI-compatible)",
                    ))
        except Exception as exc:  # noqa: BLE001
            log.warning("Local model discovery failed at %s/models: %s", self._base_url, exc)
        self._cached = models
        return models
