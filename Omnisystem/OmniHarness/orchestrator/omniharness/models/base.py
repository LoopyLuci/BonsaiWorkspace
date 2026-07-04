"""
Abstract base types and interfaces for all model adapters.
All providers must implement ModelAdapter.
"""

from __future__ import annotations

import time
from abc import ABC, abstractmethod
from typing import Any, AsyncIterator, Dict, List, Optional

from pydantic import BaseModel, Field


# ---------------------------------------------------------------------------
# Message & tool primitives
# ---------------------------------------------------------------------------


class ToolCall(BaseModel):
    """A tool call requested by the model."""

    id: str
    name: str
    arguments: Dict[str, Any] = Field(default_factory=dict)


class ChatMessage(BaseModel):
    """A single message in a conversation."""

    role: str  # "system" | "user" | "assistant" | "tool"
    content: str = ""
    name: Optional[str] = None
    tool_call_id: Optional[str] = None
    # For assistant turns that requested tools — preserved across turns so the
    # provider accepts the following tool-result messages (native function calling).
    tool_calls: Optional[List[ToolCall]] = None

    model_config = {"extra": "allow"}


class ToolDef(BaseModel):
    """Definition of a tool that can be offered to a model."""

    name: str
    description: str
    parameters: Dict[str, Any] = Field(
        default_factory=lambda: {"type": "object", "properties": {}}
    )


# ---------------------------------------------------------------------------
# Request / Response
# ---------------------------------------------------------------------------


class ChatRequest(BaseModel):
    """Unified chat request sent to any adapter."""

    model_id: str
    messages: List[ChatMessage]
    temperature: float = 0.7
    max_tokens: int = 4096
    system: Optional[str] = None
    tools: Optional[List[ToolDef]] = None
    stream: bool = False
    metadata: Dict[str, Any] = Field(default_factory=dict)


class ChatResponse(BaseModel):
    """Unified response returned from any adapter."""

    content: str
    model_used: str
    finish_reason: str = "stop"
    input_tokens: int = 0
    output_tokens: int = 0
    tool_calls: List[ToolCall] = Field(default_factory=list)
    latency_ms: float = 0.0


# ---------------------------------------------------------------------------
# Model catalogue
# ---------------------------------------------------------------------------


class ModelInfo(BaseModel):
    """Metadata about a single model."""

    id: str
    provider: str
    context_window: int = 0
    supports_tools: bool = False
    supports_vision: bool = False
    description: str = ""


# ---------------------------------------------------------------------------
# Abstract adapter
# ---------------------------------------------------------------------------


class ModelAdapter(ABC):
    """All provider-specific adapters implement this interface."""

    provider_name: str = "unknown"

    @abstractmethod
    async def chat(self, request: ChatRequest) -> ChatResponse:
        """Send a chat request and return a complete response."""
        ...

    @abstractmethod
    async def stream(self, request: ChatRequest) -> AsyncIterator[str]:
        """Stream tokens as they are generated."""
        ...

    @abstractmethod
    async def health(self) -> bool:
        """Return True if the provider is reachable and authenticated."""
        ...

    @abstractmethod
    def list_models(self) -> List[ModelInfo]:
        """Return the models this adapter supports."""
        ...

    # Convenience timing helper
    @staticmethod
    def _now_ms() -> float:
        return time.monotonic() * 1_000
