"""
OmniHarness Substrate — the universal layer for safe, controllable agentic AI:
swarms, RAG, model ensembles, governance, and self-improvement / distillation.

Every engine takes an injected async `LLMFn` (model, messages, system) -> text,
so the core logic is provider-agnostic and unit-testable without any provider SDK.
The FastAPI server wires the real ModelRouter; tests inject a fake generator.
"""

from .types import LLMFn, AgentSpec, Message
from .governance import (
    Budget,
    Usage,
    CapabilityPolicy,
    AuditLog,
    KillSwitch,
    Governor,
    BudgetExceeded,
    PolicyViolation,
    Aborted,
)
from .rag import RagPipeline, chunk_text
from .ensemble import run_ensemble, layered_moa
from .swarm import SwarmCoordinator, Blackboard
from .evolution import (
    TrajectoryStore,
    PreferenceStore,
    EvolutionaryOptimizer,
    DistillationDatasetBuilder,
)

__all__ = [
    "LLMFn", "AgentSpec", "Message",
    "Budget", "Usage", "CapabilityPolicy", "AuditLog", "KillSwitch", "Governor",
    "BudgetExceeded", "PolicyViolation", "Aborted",
    "RagPipeline", "chunk_text",
    "run_ensemble", "layered_moa",
    "SwarmCoordinator", "Blackboard",
    "TrajectoryStore", "PreferenceStore", "EvolutionaryOptimizer", "DistillationDatasetBuilder",
]
