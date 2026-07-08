"""
Intelligent model router.
Reads API keys from environment, registers adapters, routes "provider/model" strings,
health-checks all providers, and falls back when a primary adapter fails.
"""

from __future__ import annotations

import logging
import os
from typing import Dict, List, Optional, Tuple

from omniharness.models.base import (
    ChatRequest,
    ChatResponse,
    ModelAdapter,
    ModelInfo,
)

log = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Provider name -> default model
# ---------------------------------------------------------------------------

_PROVIDER_DEFAULTS: Dict[str, str] = {
    "anthropic":  "claude-sonnet-4-6",
    "openai":     "gpt-4o",
    "cohere":     "command-r-plus",
    "mistral":    "mistral-large-latest",
    "gemini":     "gemini-2.0-flash",
    "groq":       "llama-3.3-70b-versatile",
    "openrouter": "openai/gpt-4o",
    "together":   "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo",
    "fireworks":  "accounts/fireworks/models/llama-v3p1-70b-instruct",
    "ollama":     "llama3.2",
    "local":      "",
}

# ---------------------------------------------------------------------------
# Lazy imports — adapters are only imported if their provider key is present
# ---------------------------------------------------------------------------


def _load_anthropic(api_key: str) -> ModelAdapter:
    from omniharness.models.anthropic_adapter import AnthropicAdapter
    return AnthropicAdapter(api_key=api_key)


def _load_openai(api_key: str) -> ModelAdapter:
    from omniharness.models.openai_adapter import OpenAIAdapter
    return OpenAIAdapter(api_key=api_key)


def _load_cohere(api_key: str) -> ModelAdapter:
    from omniharness.models.cohere_adapter import CohereAdapter
    return CohereAdapter(api_key=api_key)


def _load_mistral(api_key: str) -> ModelAdapter:
    from omniharness.models.mistral_adapter import MistralAdapter
    return MistralAdapter(api_key=api_key)


def _load_gemini(api_key: str) -> ModelAdapter:
    from omniharness.models.gemini_adapter import GeminiAdapter
    return GeminiAdapter(api_key=api_key)


def _load_groq(api_key: str) -> ModelAdapter:
    from omniharness.models.groq_adapter import GroqAdapter
    return GroqAdapter(api_key=api_key)


def _load_openrouter(api_key: str) -> ModelAdapter:
    from omniharness.models.openrouter_adapter import OpenRouterAdapter
    return OpenRouterAdapter(api_key=api_key)


def _load_together(api_key: str) -> ModelAdapter:
    from omniharness.models.together_adapter import TogetherAdapter
    return TogetherAdapter(api_key=api_key)


def _load_fireworks(api_key: str) -> ModelAdapter:
    from omniharness.models.fireworks_adapter import FireworksAdapter
    return FireworksAdapter(api_key=api_key)


def _load_ollama(_: str) -> ModelAdapter:
    from omniharness.models.ollama_adapter import OllamaAdapter
    base = os.getenv("OLLAMA_BASE_URL", "http://localhost:11434")
    return OllamaAdapter(base_url=base)


def _load_local(_: str) -> ModelAdapter:
    from omniharness.models.local_adapter import LocalOpenAIAdapter
    base = os.getenv("LOCAL_OPENAI_BASE_URL", "http://localhost:8081/v1")
    return LocalOpenAIAdapter(base_url=base)


_LOADERS = {
    "ANTHROPIC_API_KEY":  ("anthropic",  _load_anthropic),
    "OPENAI_API_KEY":     ("openai",     _load_openai),
    "COHERE_API_KEY":     ("cohere",     _load_cohere),
    "MISTRAL_API_KEY":    ("mistral",    _load_mistral),
    "GOOGLE_API_KEY":     ("gemini",     _load_gemini),
    "GROQ_API_KEY":       ("groq",       _load_groq),
    "OPENROUTER_API_KEY": ("openrouter", _load_openrouter),
    "TOGETHER_API_KEY":   ("together",   _load_together),
    "FIREWORKS_API_KEY":  ("fireworks",  _load_fireworks),
    "OLLAMA_ENABLED":     ("ollama",     _load_ollama),
    "LOCAL_OPENAI_ENABLED": ("local",    _load_local),
}


class ModelRouter:
    """
    Routes ChatRequest objects to the appropriate ModelAdapter.

    Model ID format:
      "provider/model"   e.g. "anthropic/claude-sonnet-4-6"
      "model"            e.g. "gpt-4o"  (provider inferred from known model lists)
    """

    def __init__(self) -> None:
        self._registry: Dict[str, ModelAdapter] = {}
        self._last_autodiscover: float = 0.0

    # ------------------------------------------------------------------
    # Registration
    # ------------------------------------------------------------------

    def register(self, provider: str, adapter: ModelAdapter) -> None:
        """Manually register an adapter under a provider name."""
        self._registry[provider] = adapter
        log.info("Registered model adapter: %s", provider)

    def register_from_env(self) -> None:
        """
        Read well-known env vars and register adapters for any that are set.
        Reads .env via python-dotenv if present.
        """
        try:
            from dotenv import load_dotenv
            load_dotenv()
        except ImportError:
            pass

        for env_var, (provider, loader) in _LOADERS.items():
            value = os.getenv(env_var, "")
            if value:
                try:
                    adapter = loader(value)
                    self.register(provider, adapter)
                except Exception as exc:
                    log.warning(
                        "Failed to load adapter for %s (%s): %s", provider, env_var, exc
                    )

    # ------------------------------------------------------------------
    # Zero-config local model discovery
    # ------------------------------------------------------------------

    @staticmethod
    def _probe(url: str, timeout: float = 0.6) -> bool:
        """True if an HTTP endpoint responds at all (any status) within `timeout`."""
        import urllib.request
        import urllib.error
        try:
            urllib.request.urlopen(url, timeout=timeout)
            return True
        except urllib.error.HTTPError:
            return True  # responded (e.g. 404) => something is listening
        except Exception:
            return False

    def autodiscover_local(self, force: bool = False, min_interval: float = 20.0) -> None:
        """
        Probe for locally-running model runtimes and register adapters for any
        found — with NO API key and NO env var required, so a user with Ollama
        or LM Studio installed can start chatting immediately (zero setup).

        Discovers:
          • Ollama                          http://localhost:11434
          • OpenAI-compatible servers on common ports (LM Studio :1234,
            llama.cpp / llamafile :8080/:8081, Jan / vLLM / text-gen :5000, …)

        Throttled to at most once per `min_interval` seconds so it can be called
        cheaply on every /api/models request to pick up newly-started runtimes.
        Never removes adapters registered from env, and never re-registers a
        provider that is already present.
        """
        import time
        now = time.time()
        if not force and (now - self._last_autodiscover) < min_interval:
            return
        self._last_autodiscover = now

        # ── Ollama ──────────────────────────────────────────────────────────
        if "ollama" not in self._registry:
            base = os.getenv("OLLAMA_BASE_URL", "http://localhost:11434")
            if self._probe(base + "/api/tags"):
                try:
                    from omniharness.models.ollama_adapter import OllamaAdapter
                    self.register("ollama", OllamaAdapter(base_url=base))
                    log.info("Auto-discovered local Ollama runtime at %s", base)
                except Exception as exc:  # noqa: BLE001
                    log.warning("Failed to register auto-discovered Ollama: %s", exc)

        # ── OpenAI-compatible local servers (LM Studio, llama.cpp, Jan, …) ──
        if "local" not in self._registry:
            candidates = []
            explicit = os.getenv("LOCAL_OPENAI_BASE_URL", "").strip()
            if explicit:
                candidates.append(explicit)
            candidates += [
                "http://localhost:1234/v1",   # LM Studio
                "http://localhost:8081/v1",   # llama.cpp / llama-server
                "http://localhost:8080/v1",   # llamafile / misc
                "http://localhost:5000/v1",   # text-generation-webui / vLLM
                "http://localhost:11435/v1",  # alt
            ]
            for base in candidates:
                if self._probe(base + "/models"):
                    try:
                        from omniharness.models.local_adapter import LocalOpenAIAdapter
                        self.register("local", LocalOpenAIAdapter(base_url=base))
                        log.info("Auto-discovered local OpenAI-compatible server at %s", base)
                    except Exception as exc:  # noqa: BLE001
                        log.warning("Failed to register auto-discovered local server: %s", exc)
                    break

    # ------------------------------------------------------------------
    # Routing
    # ------------------------------------------------------------------

    def _parse_provider_model(self, model_id: str) -> Tuple[Optional[str], str]:
        """
        Parse "provider/model" -> (provider, model).
        If no slash, try to infer provider from known model lists.
        """
        if "/" in model_id:
            provider, model = model_id.split("/", 1)
            return provider, model

        # Try to infer provider by scanning registered adapters
        for provider, adapter in self._registry.items():
            known = {m.id for m in adapter.list_models()}
            if model_id in known:
                return provider, model_id

        return None, model_id

    def route(self, model_id: str) -> ModelAdapter:
        """
        Return the adapter that should handle the given model_id.
        Raises RuntimeError if no suitable adapter is registered.
        """
        provider, _ = self._parse_provider_model(model_id)

        if provider and provider in self._registry:
            return self._registry[provider]

        if not self._registry:
            raise RuntimeError("No model adapters registered. Check your API keys.")

        # Fall back to first registered adapter
        first_provider, adapter = next(iter(self._registry.items()))
        log.warning(
            "No adapter for model '%s' (provider='%s'). Falling back to '%s'.",
            model_id,
            provider,
            first_provider,
        )
        return adapter

    # ------------------------------------------------------------------
    # Chat helpers (with fallback)
    # ------------------------------------------------------------------

    async def stream(self, request: ChatRequest):
        """Route and stream a chat request."""
        adapter = self.route(request.model_id)
        async for chunk in adapter.stream(request):
            yield chunk

    def list_providers(self) -> List[str]:
        return list(self._registry.keys())

    async def health(self, provider: str) -> bool:
        """Check health of a single provider."""
        adapter = self._registry.get(provider)
        if not adapter:
            return False
        try:
            return await adapter.health()
        except Exception:
            return False

    async def chat(self, request: ChatRequest) -> ChatResponse:
        """Route and execute a chat request, falling back on error."""
        provider, model_part = self._parse_provider_model(request.model_id)

        # Try primary
        try:
            adapter = self.route(request.model_id)
            return await adapter.chat(request)
        except RuntimeError as exc:
            log.error("Primary adapter failed for '%s': %s", request.model_id, exc)

        # Try remaining adapters
        tried = {provider}
        for prov, adapter in self._registry.items():
            if prov in tried:
                continue
            tried.add(prov)
            try:
                log.info("Falling back to provider '%s'", prov)
                fallback_req = request.model_copy(
                    update={"model_id": f"{prov}/{_PROVIDER_DEFAULTS.get(prov, model_part)}"}
                )
                return await adapter.chat(fallback_req)
            except Exception as fallback_exc:
                log.warning("Fallback to '%s' also failed: %s", prov, fallback_exc)

        raise RuntimeError(
            f"All adapters failed for model '{request.model_id}'. "
            "Check your API keys and network connectivity."
        )

    # ------------------------------------------------------------------
    # Health & catalogue
    # ------------------------------------------------------------------

    async def health_all(self) -> Dict[str, bool]:
        """Check health of all registered providers concurrently."""
        import asyncio

        async def _check(provider: str, adapter: ModelAdapter) -> Tuple[str, bool]:
            try:
                ok = await adapter.health()
            except Exception:
                ok = False
            return provider, ok

        tasks = [_check(p, a) for p, a in self._registry.items()]
        results = await asyncio.gather(*tasks)
        return dict(results)

    def list_all_models(self) -> List[ModelInfo]:
        """Aggregate model lists from all registered adapters."""
        models: List[ModelInfo] = []
        for adapter in self._registry.values():
            models.extend(adapter.list_models())
        return models

    @property
    def providers(self) -> List[str]:
        return list(self._registry.keys())
