"""
Governance — the safety and control spine of the substrate.

Every autonomous run (swarm, ensemble, evolution) executes inside a Governor that
enforces resource budgets, capability policy, a tamper-evident audit log, and a
kill switch. Nothing runs unbounded; everything is recorded; the user can stop it.
"""
from __future__ import annotations

import hashlib
import json
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional


class BudgetExceeded(RuntimeError):
    """Raised when a run exceeds its resource budget."""


class PolicyViolation(RuntimeError):
    """Raised when a run attempts a disallowed model/tool/capability."""


class Aborted(RuntimeError):
    """Raised when the kill switch is tripped."""


@dataclass
class Budget:
    """Hard limits for a single governed run. 0 / None means unlimited."""
    max_model_calls: int = 200
    max_tokens: int = 2_000_000
    max_cost_usd: float = 10.0
    max_steps: int = 500
    max_wallclock_s: float = 1800.0
    max_parallel: int = 16


@dataclass
class Usage:
    model_calls: int = 0
    tokens: int = 0
    cost_usd: float = 0.0
    steps: int = 0
    started_at: float = field(default_factory=time.monotonic)

    @property
    def elapsed_s(self) -> float:
        return time.monotonic() - self.started_at

    def snapshot(self) -> Dict[str, Any]:
        return {
            "model_calls": self.model_calls,
            "tokens": self.tokens,
            "cost_usd": round(self.cost_usd, 4),
            "steps": self.steps,
            "elapsed_s": round(self.elapsed_s, 2),
        }


@dataclass
class CapabilityPolicy:
    """What a governed run is permitted to touch. Empty allow-list = allow all."""
    allowed_models: List[str] = field(default_factory=list)     # empty = any
    allowed_tools: List[str] = field(default_factory=list)      # empty = any
    denied_tools: List[str] = field(default_factory=list)
    allow_network: bool = True
    allow_filesystem_write: bool = True
    max_agents: int = 64

    def model_allowed(self, model: str) -> bool:
        return not self.allowed_models or model in self.allowed_models

    def tool_allowed(self, tool: str) -> bool:
        if tool in self.denied_tools:
            return False
        return not self.allowed_tools or tool in self.allowed_tools


class AuditLog:
    """Append-only, SHA-256 hash-chained event log — tamper-evident by construction."""

    def __init__(self) -> None:
        self._events: List[Dict[str, Any]] = []
        self._head = "0" * 64

    def append(self, kind: str, payload: Optional[Dict[str, Any]] = None) -> str:
        ts = time.time()
        body = json.dumps(payload or {}, sort_keys=True, default=str)
        digest = hashlib.sha256(f"{self._head}{ts}{kind}{body}".encode()).hexdigest()
        self._events.append({
            "seq": len(self._events),
            "timestamp": ts,
            "kind": kind,
            "payload": payload or {},
            "prev": self._head,
            "hash": digest,
        })
        self._head = digest
        return digest

    def verify(self) -> bool:
        head = "0" * 64
        for ev in self._events:
            body = json.dumps(ev["payload"], sort_keys=True, default=str)
            digest = hashlib.sha256(f"{head}{ev['timestamp']}{ev['kind']}{body}".encode()).hexdigest()
            if digest != ev["hash"] or ev["prev"] != head:
                return False
            head = digest
        return True

    def events(self) -> List[Dict[str, Any]]:
        return list(self._events)


class KillSwitch:
    """A cooperative stop signal checked by every governed loop."""

    def __init__(self) -> None:
        self._tripped = False
        self._reason = ""

    def trip(self, reason: str = "user requested stop") -> None:
        self._tripped = True
        self._reason = reason

    @property
    def tripped(self) -> bool:
        return self._tripped

    @property
    def reason(self) -> str:
        return self._reason


class Governor:
    """
    Wraps a run with budget, policy, audit, and kill-switch enforcement.
    Call `checkpoint()` at each step and `record_call(...)` after each model call.
    """

    def __init__(
        self,
        budget: Optional[Budget] = None,
        policy: Optional[CapabilityPolicy] = None,
        kill_switch: Optional[KillSwitch] = None,
        audit: Optional[AuditLog] = None,
    ) -> None:
        self.budget = budget or Budget()
        self.policy = policy or CapabilityPolicy()
        self.kill = kill_switch or KillSwitch()
        self.audit = audit or AuditLog()
        self.usage = Usage()

    # Rough cost table (USD per 1K tokens, blended) for accounting only.
    _COST_PER_1K = 0.005

    def checkpoint(self, note: str = "") -> None:
        """Enforce kill switch, wall-clock, and step limits. Call once per step."""
        if self.kill.tripped:
            self.audit.append("aborted", {"reason": self.kill.reason, "note": note})
            raise Aborted(self.kill.reason)
        self.usage.steps += 1
        if self.budget.max_steps and self.usage.steps > self.budget.max_steps:
            self.audit.append("budget_exceeded", {"limit": "max_steps", "value": self.usage.steps})
            raise BudgetExceeded(f"max_steps ({self.budget.max_steps}) exceeded")
        if self.budget.max_wallclock_s and self.usage.elapsed_s > self.budget.max_wallclock_s:
            self.audit.append("budget_exceeded", {"limit": "max_wallclock_s", "value": self.usage.elapsed_s})
            raise BudgetExceeded(f"max_wallclock_s ({self.budget.max_wallclock_s}) exceeded")

    def check_model(self, model: str) -> None:
        if not self.policy.model_allowed(model):
            self.audit.append("policy_violation", {"model": model})
            raise PolicyViolation(f"model '{model}' is not permitted by policy")
        if self.budget.max_model_calls and self.usage.model_calls >= self.budget.max_model_calls:
            self.audit.append("budget_exceeded", {"limit": "max_model_calls", "value": self.usage.model_calls})
            raise BudgetExceeded(f"max_model_calls ({self.budget.max_model_calls}) exceeded")

    def check_tool(self, tool: str) -> None:
        if not self.policy.tool_allowed(tool):
            self.audit.append("policy_violation", {"tool": tool})
            raise PolicyViolation(f"tool '{tool}' is not permitted by policy")

    def record_call(self, model: str, tokens: int) -> None:
        self.usage.model_calls += 1
        self.usage.tokens += max(0, tokens)
        self.usage.cost_usd += (max(0, tokens) / 1000.0) * self._COST_PER_1K
        self.audit.append("model_call", {"model": model, "tokens": tokens, "usage": self.usage.snapshot()})
        if self.budget.max_tokens and self.usage.tokens > self.budget.max_tokens:
            raise BudgetExceeded(f"max_tokens ({self.budget.max_tokens}) exceeded")
        if self.budget.max_cost_usd and self.usage.cost_usd > self.budget.max_cost_usd:
            raise BudgetExceeded(f"max_cost_usd ({self.budget.max_cost_usd}) exceeded")

    def parallelism(self, requested: int) -> int:
        """Clamp a requested fan-out to the policy/budget limits."""
        return max(1, min(requested, self.budget.max_parallel, self.policy.max_agents))

    def report(self) -> Dict[str, Any]:
        return {
            "usage": self.usage.snapshot(),
            "budget": self.budget.__dict__,
            "audit_valid": self.audit.verify(),
            "audit_events": len(self.audit.events()),
            "killed": self.kill.tripped,
        }
