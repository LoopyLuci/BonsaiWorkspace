"""Vector memory client — wraps gRPC MemoryService with local numpy fallback."""
from __future__ import annotations

import json
import math
import time
from dataclasses import dataclass, field
from typing import Any


@dataclass
class MemoryEntry:
    id:         str
    collection: str
    content:    str
    metadata:   dict
    embedding:  list[float]
    created_at: float
    score:      float = 1.0


class LocalVectorStore:
    """Pure-Python in-memory vector store with cosine similarity. No external deps."""

    def __init__(self) -> None:
        self._data: dict[str, list[MemoryEntry]] = {}

    def store(
        self,
        collection: str,
        content:    str,
        metadata:   dict | None = None,
        embedding:  list[float] | None = None,
    ) -> str:
        import uuid
        eid = str(uuid.uuid4())
        emb = embedding or self._hash_embed(content)
        entry = MemoryEntry(
            id=eid, collection=collection, content=content,
            metadata=metadata or {}, embedding=emb, created_at=time.time(),
        )
        self._data.setdefault(collection, []).append(entry)
        return eid

    def retrieve(self, collection: str, eid: str) -> MemoryEntry | None:
        for e in self._data.get(collection, []):
            if e.id == eid:
                return e
        return None

    def search(
        self,
        collection: str,
        query:      str,
        top_k:      int   = 5,
        threshold:  float = 0.0,
    ) -> list[MemoryEntry]:
        q_emb   = self._hash_embed(query)
        entries = self._data.get(collection, [])
        scored: list[tuple[MemoryEntry, float]] = []
        for e in entries:
            score = self._cosine(q_emb, e.embedding)
            if score >= threshold:
                scored.append((e, score))
        scored.sort(key=lambda x: x[1], reverse=True)
        result = []
        for entry, score in scored[:top_k]:
            import copy
            e2 = copy.copy(entry)
            e2.score = score
            result.append(e2)
        return result

    def delete(self, collection: str, eid: str) -> bool:
        entries = self._data.get(collection, [])
        before  = len(entries)
        self._data[collection] = [e for e in entries if e.id != eid]
        return len(self._data[collection]) < before

    def list_collections(self) -> list[str]:
        return list(self._data.keys())

    def size(self, collection: str) -> int:
        return len(self._data.get(collection, []))

    @staticmethod
    def _cosine(a: list[float], b: list[float]) -> float:
        n = min(len(a), len(b))
        if n == 0: return 0.0
        dot = sum(a[i] * b[i] for i in range(n))
        na  = math.sqrt(sum(x*x for x in a[:n]))
        nb  = math.sqrt(sum(x*x for x in b[:n]))
        return dot / (na * nb) if na and nb else 0.0

    @staticmethod
    def _hash_embed(text: str, dim: int = 128) -> list[float]:
        """Deterministic hash-based embedding for offline use."""
        emb = [0.0] * dim
        FNV_PRIME = 1_099_511_628_211
        FNV_BASIS = 14_695_981_039_346_656_037
        for i, ch in enumerate(text.encode("utf-8")):
            h = (FNV_BASIS ^ ch) * FNV_PRIME & 0xFFFFFFFFFFFFFFFF
            slot = (h ^ i) % dim
            emb[slot] += (ch - 128) / 128.0
        norm = math.sqrt(sum(x*x for x in emb))
        if norm > 0:
            emb = [x / norm for x in emb]
        return emb


class VectorClient:
    """
    Async vector store client. Uses gRPC when kernel is available,
    falls back to local in-memory store automatically.
    """

    def __init__(self, grpc_host: str = "localhost", grpc_port: int = 50051) -> None:
        self._host   = grpc_host
        self._port   = grpc_port
        self._local  = LocalVectorStore()
        self._grpc   = None   # lazy init

    async def _ensure_grpc(self) -> bool:
        if self._grpc is not None:
            return True
        try:
            import grpc
            from omniharness.grpc_client import GrpcClient
            c = GrpcClient(self._host, self._port)
            await c.connect()
            self._grpc = c
            return True
        except Exception:
            return False

    async def store(
        self,
        collection: str,
        content:    str,
        metadata:   dict | None = None,
    ) -> str:
        if await self._ensure_grpc():
            try:
                return await self._grpc.store_memory(collection, content, metadata or {})
            except Exception:
                pass
        return self._local.store(collection, content, metadata)

    async def search(
        self,
        collection: str,
        query:      str,
        top_k:      int   = 5,
        threshold:  float = 0.0,
    ) -> list[MemoryEntry]:
        if await self._ensure_grpc():
            try:
                return await self._grpc.search_memory(collection, query, top_k, threshold)
            except Exception:
                pass
        return self._local.search(collection, query, top_k, threshold)

    async def retrieve(self, collection: str, eid: str) -> MemoryEntry | None:
        return self._local.retrieve(collection, eid)

    async def delete(self, collection: str, eid: str) -> bool:
        return self._local.delete(collection, eid)

    async def list_collections(self) -> list[str]:
        return self._local.list_collections()
