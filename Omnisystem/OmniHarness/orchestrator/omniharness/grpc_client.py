"""gRPC client to the OmniHarness Rust kernel."""
from __future__ import annotations

import asyncio
import json
import time
from typing import Any


class GrpcClient:
    """Async gRPC client wrapping all OmniHarness kernel services."""

    def __init__(self, host: str = "localhost", port: int = 50051) -> None:
        self._host = host
        self._port = port
        self._channel = None
        self._stubs:  dict[str, Any] = {}

    async def connect(self) -> None:
        import grpc
        from grpc.aio import insecure_channel

        # Import generated proto stubs
        try:
            import omniharness_pb2 as pb
            import omniharness_pb2_grpc as stub
        except ImportError:
            raise RuntimeError(
                "Proto stubs not generated. Run: "
                "python -m grpc_tools.protoc -I../proto --python_out=. --grpc_python_out=. ../proto/omniharness.proto"
            )

        self._pb      = pb
        addr          = f"{self._host}:{self._port}"
        self._channel = insecure_channel(addr)
        self._stubs   = {
            "event_store": stub.EventStoreServiceStub(self._channel),
            "model":       stub.ModelServiceStub(self._channel),
            "memory":      stub.MemoryServiceStub(self._channel),
            "tool":        stub.ToolServiceStub(self._channel),
            "session":     stub.SessionServiceStub(self._channel),
            "harness":     stub.HarnessServiceStub(self._channel),
        }

    async def close(self) -> None:
        if self._channel:
            await self._channel.close()

    # ── Event Store ───────────────────────────────────────────────

    async def append_event(self, module: str, etype: str, payload: Any, session_id: str = "") -> str:
        req  = self._pb.AppendRequest(
            module_source=module, event_type=etype,
            payload_json=json.dumps(payload) if not isinstance(payload, str) else payload,
            session_id=session_id,
        )
        resp = await self._stubs["event_store"].AppendEvent(req)
        return resp.event_hash

    async def verify_chain(self) -> dict:
        resp = await self._stubs["event_store"].VerifyChain(self._pb.VerifyRequest())
        return {"valid": resp.is_valid, "tip": resp.tip_hash, "depth": resp.depth}

    # ── Model ─────────────────────────────────────────────────────

    async def chat(self, model_id: str, messages: list[dict], **kwargs) -> dict:
        msgs = [self._pb.ChatMessage(role=m["role"], content=m["content"]) for m in messages]
        req  = self._pb.ChatRequest(
            model_id=model_id, messages=msgs,
            temperature=kwargs.get("temperature", 0.7),
            max_tokens=kwargs.get("max_tokens", 4096),
            system=kwargs.get("system", ""),
        )
        resp = await self._stubs["model"].Chat(req)
        return {
            "content":       resp.content,
            "model_used":    resp.model_used,
            "finish_reason": resp.finish_reason,
            "input_tokens":  resp.input_tokens,
            "output_tokens": resp.output_tokens,
            "latency_ms":    resp.latency_ms,
        }

    async def list_models(self, provider: str = "") -> list[dict]:
        resp = await self._stubs["model"].ListModels(self._pb.ListModelsRequest(provider=provider))
        return [{"id": m.id, "provider": m.provider, "display_name": m.display_name} for m in resp.models]

    # ── Memory ────────────────────────────────────────────────────

    async def store_memory(self, collection: str, content: str, metadata: dict | None = None) -> str:
        req  = self._pb.StoreRequest(collection=collection, content=content, metadata=metadata or {}, embed=True)
        resp = await self._stubs["memory"].Store(req)
        return resp.id

    async def search_memory(self, collection: str, query: str, top_k: int = 5, threshold: float = 0.0) -> list[dict]:
        req  = self._pb.SemanticSearchRequest(collection=collection, query=query, top_k=top_k, threshold=threshold)
        resp = await self._stubs["memory"].SearchSemantic(req)
        return [{"id": e.id, "content": e.content, "score": e.score, "metadata": dict(e.metadata)} for e in resp.results]

    # ── Tools ─────────────────────────────────────────────────────

    async def execute_tool(self, name: str, arguments: dict, timeout_ms: int = 30000) -> dict:
        req  = self._pb.ToolExecuteRequest(
            name=name,
            arguments=json.dumps(arguments),
            timeout_ms=timeout_ms,
        )
        resp = await self._stubs["tool"].Execute(req)
        return {"result": resp.result, "success": resp.success, "error": resp.error, "latency_ms": resp.latency_ms}

    async def list_tools(self) -> list[dict]:
        resp = await self._stubs["tool"].List(self._pb.ToolListRequest())
        return [{"name": t.name, "description": t.description} for t in resp.tools]

    # ── Sessions ──────────────────────────────────────────────────

    async def create_session(self, title: str, model_id: str) -> dict:
        req  = self._pb.CreateSessionReq(title=title, model_id=model_id)
        resp = await self._stubs["session"].CreateSession(req)
        s    = resp.session
        return {"id": s.id, "title": s.title, "model_id": s.model_id}

    async def get_session(self, session_id: str) -> dict | None:
        resp = await self._stubs["session"].GetSession(self._pb.GetSessionReq(id=session_id))
        if not resp.found: return None
        s = resp.session
        return {
            "id": s.id, "title": s.title, "model_id": s.model_id,
            "history": [{"role": m.role, "content": m.content} for m in resp.history],
        }

    async def list_sessions(self, limit: int = 50) -> list[dict]:
        resp = await self._stubs["session"].ListSessions(self._pb.ListSessionsReq(limit=limit))
        return [{"id": s.id, "title": s.title, "model_id": s.model_id, "updated_at": s.updated_at} for s in resp.sessions]

    # ── Harness Status ────────────────────────────────────────────

    async def status(self) -> dict:
        resp = await self._stubs["harness"].Status(self._pb.StatusRequest())
        return {
            "version": resp.version, "healthy": resp.healthy,
            "uptime_secs": resp.uptime_secs, "events_stored": resp.events_stored,
        }

    # ── Retry wrapper ─────────────────────────────────────────────

    async def with_retry(self, fn, *args, retries: int = 3, delay: float = 1.0, **kwargs):
        last_err = None
        for i in range(retries):
            try:
                return await fn(*args, **kwargs)
            except Exception as e:
                last_err = e
                if i < retries - 1:
                    await asyncio.sleep(delay * (2 ** i))
        raise last_err
