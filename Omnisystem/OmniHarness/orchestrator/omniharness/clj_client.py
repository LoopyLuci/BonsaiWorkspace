"""HTTP client to the Clojure orchestrator's HTN planner / policy API
(clj-orchestrator, http_server.clj — default http://localhost:8090).

Optional sidecar, same graceful-degradation contract as GrpcClient in
grpc_client.py: clj-orchestrator may not be running, and every call here
fails soft (returns None / False) rather than raising, so the rest of the
orchestrator works identically with or without it.
"""
from __future__ import annotations

import os
from typing import Any

import httpx

CLJ_BASE_URL = os.environ.get("CLJ_ORCHESTRATOR_URL", "http://localhost:8090")


class CljClient:
    """Thin async client for the Clojure orchestrator's planner/policy HTTP API."""

    def __init__(self, base_url: str = CLJ_BASE_URL, timeout: float = 3.0) -> None:
        self._base_url = base_url
        self._timeout = timeout

    async def health(self) -> bool:
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                r = await client.get(f"{self._base_url}/health")
                return r.status_code == 200
        except Exception:
            return False

    async def verify_kernel_chain(self) -> dict[str, Any] | None:
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                r = await client.get(f"{self._base_url}/kernel/verify")
                r.raise_for_status()
                return r.json()
        except Exception:
            return None

    async def plan(self, task_name: str, params: dict[str, Any] | None = None) -> dict[str, Any] | None:
        """HTN plan for a compound task via clj-orchestrator's planner. Returns
        None if clj-orchestrator isn't reachable — callers should fall back to
        their own planning logic (e.g. substrate/swarm.py's Python planner)."""
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                r = await client.post(
                    f"{self._base_url}/plan",
                    json={"task_name": task_name, "params": params or {}},
                )
                r.raise_for_status()
                return r.json()
        except Exception:
            return None

    async def plan_execute(self, task_name: str, params: dict[str, Any] | None = None) -> dict[str, Any] | None:
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                r = await client.post(
                    f"{self._base_url}/plan/execute",
                    json={"task_name": task_name, "params": params or {}},
                )
                r.raise_for_status()
                return r.json()
        except Exception:
            return None

    async def policy_check(self, action: str, args: dict[str, Any] | None = None) -> dict[str, Any] | None:
        try:
            async with httpx.AsyncClient(timeout=self._timeout) as client:
                r = await client.post(
                    f"{self._base_url}/policy/check",
                    json={"action": action, "args": args or {}},
                )
                r.raise_for_status()
                return r.json()
        except Exception:
            return None
