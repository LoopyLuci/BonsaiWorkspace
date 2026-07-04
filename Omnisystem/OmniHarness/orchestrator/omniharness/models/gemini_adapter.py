"""
Google Gemini adapter — uses the google-generativeai SDK.
Converts messages to Gemini parts/role format.
Safety settings set to BLOCK_NONE (user-controlled content).
"""

from __future__ import annotations

import asyncio
import logging
from typing import Any, AsyncIterator, Dict, List, Optional

import google.generativeai as genai
from google.generativeai.types import HarmBlockThreshold, HarmCategory

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
    ModelInfo(id="gemini-2.0-flash",         provider="gemini", context_window=1_048_576, supports_tools=True, supports_vision=True,  description="Gemini 2.0 Flash — fast multimodal"),
    ModelInfo(id="gemini-2.0-flash-lite",     provider="gemini", context_window=1_048_576, supports_tools=True, supports_vision=True,  description="Gemini 2.0 Flash Lite"),
    ModelInfo(id="gemini-2.0-pro",            provider="gemini", context_window=2_000_000, supports_tools=True, supports_vision=True,  description="Gemini 2.0 Pro"),
    ModelInfo(id="gemini-1.5-pro",            provider="gemini", context_window=2_000_000, supports_tools=True, supports_vision=True,  description="Gemini 1.5 Pro"),
    ModelInfo(id="gemini-1.5-flash",          provider="gemini", context_window=1_000_000, supports_tools=True, supports_vision=True,  description="Gemini 1.5 Flash"),
    ModelInfo(id="gemini-1.5-flash-8b",       provider="gemini", context_window=1_000_000, supports_tools=True, supports_vision=True,  description="Gemini 1.5 Flash 8B"),
]

_MODEL_IDS = {m.id for m in _MODELS}

# Block nothing — user is in control
_SAFETY_SETTINGS = {
    HarmCategory.HARM_CATEGORY_HARASSMENT: HarmBlockThreshold.BLOCK_NONE,
    HarmCategory.HARM_CATEGORY_HATE_SPEECH: HarmBlockThreshold.BLOCK_NONE,
    HarmCategory.HARM_CATEGORY_SEXUALLY_EXPLICIT: HarmBlockThreshold.BLOCK_NONE,
    HarmCategory.HARM_CATEGORY_DANGEROUS_CONTENT: HarmBlockThreshold.BLOCK_NONE,
}


def _to_gemini_role(role: str) -> str:
    """Map OpenAI-style roles to Gemini roles."""
    if role in ("assistant", "model"):
        return "model"
    return "user"  # system, tool, user -> user


def _build_history(
    messages: List[ChatMessage],
) -> tuple[Optional[str], List[Dict[str, Any]], str]:
    """
    Split messages into (system_instruction, history, last_user_message).
    Gemini's GenerativeModel takes system_instruction separately.
    history is all but the last message.
    """
    system: Optional[str] = None
    parts: List[Dict[str, Any]] = []

    for msg in messages:
        if msg.role == "system":
            system = (system + "\n\n" + msg.content) if system else msg.content
            continue
        parts.append({"role": _to_gemini_role(msg.role), "parts": [msg.content]})

    if not parts:
        return system, [], ""

    last = parts.pop()
    last_text = last["parts"][0] if last["parts"] else ""
    return system, parts, last_text


def _convert_tool_def(t: ToolDef) -> Dict[str, Any]:
    """Convert ToolDef to Gemini FunctionDeclaration dict."""
    return {
        "name": t.name,
        "description": t.description,
        "parameters": t.parameters,
    }


class GeminiAdapter(ModelAdapter):
    """Adapter for Google Gemini models."""

    provider_name = "gemini"

    def __init__(self, api_key: str, default_model: str = "gemini-2.0-flash") -> None:
        genai.configure(api_key=api_key)
        self._api_key = api_key
        self._default_model = default_model

    def _resolve_model(self, model_id: str) -> str:
        if "/" in model_id:
            _, model_id = model_id.split("/", 1)
        return model_id if model_id in _MODEL_IDS else self._default_model

    def _make_model(
        self,
        model_id: str,
        system: Optional[str],
        tools: Optional[List[ToolDef]],
    ) -> genai.GenerativeModel:
        kwargs: Dict[str, Any] = {
            "model_name": model_id,
            "safety_settings": _SAFETY_SETTINGS,
            "generation_config": genai.GenerationConfig(
                temperature=None,  # set per call
            ),
        }
        if system:
            kwargs["system_instruction"] = system
        if tools:
            kwargs["tools"] = [
                {"function_declarations": [_convert_tool_def(t) for t in tools]}
            ]
        return genai.GenerativeModel(**kwargs)

    async def chat(self, request: ChatRequest) -> ChatResponse:
        t0 = self._now_ms()
        model_id = self._resolve_model(request.model_id)
        system_from_msgs, history, last_user = _build_history(request.messages)
        system = request.system or system_from_msgs

        model = self._make_model(model_id, system, request.tools)
        gen_cfg = genai.GenerationConfig(
            temperature=request.temperature,
            max_output_tokens=request.max_tokens,
        )

        chat_session = model.start_chat(history=history)

        try:
            response = await asyncio.to_thread(
                chat_session.send_message,
                last_user,
                generation_config=gen_cfg,
                safety_settings=_SAFETY_SETTINGS,
            )
        except Exception as exc:
            raise RuntimeError(f"Gemini API error: {exc}") from exc

        text = response.text or ""
        finish_reason = "stop"
        try:
            finish_reason = str(response.candidates[0].finish_reason)
        except Exception:
            pass

        # Tool calls
        tool_calls: List[ToolCall] = []
        try:
            for candidate in response.candidates:
                for part in candidate.content.parts:
                    if part.function_call:
                        fc = part.function_call
                        tool_calls.append(
                            ToolCall(
                                id=fc.name,
                                name=fc.name,
                                arguments=dict(fc.args),
                            )
                        )
        except Exception:
            pass

        # Token counts
        in_tok = out_tok = 0
        try:
            usage = response.usage_metadata
            in_tok = usage.prompt_token_count or 0
            out_tok = usage.candidates_token_count or 0
        except Exception:
            pass

        return ChatResponse(
            content=text,
            model_used=model_id,
            finish_reason=finish_reason,
            input_tokens=in_tok,
            output_tokens=out_tok,
            tool_calls=tool_calls,
            latency_ms=self._now_ms() - t0,
        )

    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        model_id = self._resolve_model(request.model_id)
        system_from_msgs, history, last_user = _build_history(request.messages)
        system = request.system or system_from_msgs

        model = self._make_model(model_id, system, request.tools)
        gen_cfg = genai.GenerationConfig(
            temperature=request.temperature,
            max_output_tokens=request.max_tokens,
        )
        chat_session = model.start_chat(history=history)

        # Gemini streaming is synchronous — run in thread and yield chunks
        queue: asyncio.Queue[Optional[str]] = asyncio.Queue()

        def _stream_sync() -> None:
            try:
                for chunk in chat_session.send_message(
                    last_user,
                    generation_config=gen_cfg,
                    safety_settings=_SAFETY_SETTINGS,
                    stream=True,
                ):
                    text = chunk.text if chunk.text else ""
                    if text:
                        asyncio.get_event_loop().call_soon_threadsafe(queue.put_nowait, text)
                asyncio.get_event_loop().call_soon_threadsafe(queue.put_nowait, None)
            except Exception as exc:
                asyncio.get_event_loop().call_soon_threadsafe(
                    queue.put_nowait, f"[ERROR: {exc}]"
                )
                asyncio.get_event_loop().call_soon_threadsafe(queue.put_nowait, None)

        loop = asyncio.get_event_loop()
        loop.run_in_executor(None, _stream_sync)

        while True:
            token = await queue.get()
            if token is None:
                break
            yield token

    async def health(self) -> bool:
        try:
            models = await asyncio.to_thread(genai.list_models)
            return any(True for _ in models)
        except Exception as exc:
            log.warning("Gemini health check failed: %s", exc)
            return False

    def list_models(self) -> List[ModelInfo]:
        return list(_MODELS)
