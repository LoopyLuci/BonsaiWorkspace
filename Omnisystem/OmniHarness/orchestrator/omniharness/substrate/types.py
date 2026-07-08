"""Shared, dependency-free types for the OmniHarness substrate."""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Awaitable, Callable, Dict, List, Optional

# An injected model-inference function: (model_id, messages, system) -> assistant text.
# messages is a list of {"role": "...", "content": "..."} dicts.
LLMFn = Callable[[str, List[Dict[str, str]], Optional[str]], Awaitable[str]]


@dataclass
class Message:
    role: str
    content: str

    def as_dict(self) -> Dict[str, str]:
        return {"role": self.role, "content": self.content}


@dataclass
class AgentSpec:
    """A named agent role used by swarms and the evolutionary optimizer."""
    id: str
    name: str
    system: str
    model: str
    temperature: float = 0.3
    role: str = "worker"                 # orchestrator | worker | critic | judge | proposer
    tools: List[str] = field(default_factory=lambda: ["*"])
    metadata: Dict[str, str] = field(default_factory=dict)

    def clone(self, **overrides) -> "AgentSpec":
        data = {
            "id": self.id, "name": self.name, "system": self.system, "model": self.model,
            "temperature": self.temperature, "role": self.role, "tools": list(self.tools),
            "metadata": dict(self.metadata),
        }
        data.update(overrides)
        return AgentSpec(**data)
