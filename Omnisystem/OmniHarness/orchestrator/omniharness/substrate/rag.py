"""
RAG — retrieval-augmented generation pipeline.

Ingest documents → chunk (with overlap) → embed → store → retrieve top-k by cosine
→ assemble a grounded context block for any model. Dependency-free: ships a local
hash embedder + cosine store, but accepts an external embed function (e.g. an API
embedding model) for higher accuracy.
"""
from __future__ import annotations

import math
import re
import time
import uuid
from dataclasses import dataclass, field
from typing import Callable, Dict, List, Optional

EmbedFn = Callable[[str], List[float]]

_EMBED_DIM = 256


def _hash_embed(text: str, dim: int = _EMBED_DIM) -> List[float]:
    """Deterministic local embedding: hashed bag-of-tokens, L2-normalized."""
    vec = [0.0] * dim
    for tok in re.findall(r"[a-z0-9]+", text.lower()):
        h = 1469598103934665603
        for b in tok.encode():
            h ^= b
            h = (h * 1099511628211) & 0xFFFFFFFFFFFFFFFF
        vec[h % dim] += 1.0
    norm = math.sqrt(sum(v * v for v in vec)) or 1.0
    return [v / norm for v in vec]


def _cosine(a: List[float], b: List[float]) -> float:
    return sum(x * y for x, y in zip(a, b))


def chunk_text(text: str, size: int = 1200, overlap: int = 200) -> List[str]:
    """Split text into overlapping chunks, preferring paragraph/sentence breaks."""
    text = text.strip()
    if not text:
        return []
    if len(text) <= size:
        return [text]
    chunks: List[str] = []
    start = 0
    n = len(text)
    while start < n:
        end = min(start + size, n)
        if end < n:
            # Prefer to break on a paragraph, then a sentence, then whitespace.
            window = text[start:end]
            for sep in ("\n\n", ". ", "\n", " "):
                idx = window.rfind(sep)
                if idx > size * 0.5:
                    end = start + idx + len(sep)
                    break
        chunks.append(text[start:end].strip())
        if end >= n:
            break
        start = max(end - overlap, start + 1)
    return [c for c in chunks if c]


@dataclass
class Chunk:
    id: str
    doc_id: str
    text: str
    embedding: List[float]
    metadata: Dict[str, str] = field(default_factory=dict)
    score: float = 0.0


@dataclass
class RetrievedChunk:
    doc_id: str
    text: str
    score: float
    metadata: Dict[str, str]


class RagPipeline:
    """In-memory RAG store + retrieval. Swap in an external EmbedFn for accuracy."""

    def __init__(self, embed: Optional[EmbedFn] = None) -> None:
        self._embed = embed or _hash_embed
        self._chunks: List[Chunk] = []

    def ingest(self, doc_id: str, text: str, metadata: Optional[Dict[str, str]] = None,
               chunk_size: int = 1200, overlap: int = 200) -> int:
        """Chunk, embed, and store a document. Returns the number of chunks added."""
        added = 0
        for piece in chunk_text(text, chunk_size, overlap):
            self._chunks.append(Chunk(
                id=str(uuid.uuid4()), doc_id=doc_id, text=piece,
                embedding=self._embed(piece), metadata=dict(metadata or {}),
            ))
            added += 1
        return added

    def retrieve(self, query: str, k: int = 5, doc_id: Optional[str] = None,
                 min_score: float = 0.0) -> List[RetrievedChunk]:
        q = self._embed(query)
        scored: List[RetrievedChunk] = []
        for c in self._chunks:
            if doc_id and c.doc_id != doc_id:
                continue
            s = _cosine(q, c.embedding)
            if s >= min_score:
                scored.append(RetrievedChunk(c.doc_id, c.text, s, c.metadata))
        scored.sort(key=lambda r: r.score, reverse=True)
        return scored[:k]

    def assemble_context(self, query: str, k: int = 5, doc_id: Optional[str] = None) -> str:
        """Retrieve and format a grounded context block for prompting."""
        hits = self.retrieve(query, k, doc_id)
        if not hits:
            return ""
        lines = ["# Retrieved context", ""]
        for i, h in enumerate(hits, 1):
            src = h.metadata.get("source", h.doc_id)
            lines.append(f"[{i}] (source: {src}, relevance {h.score:.2f})\n{h.text}\n")
        return "\n".join(lines)

    def augment_system(self, system: Optional[str], query: str, k: int = 5) -> Optional[str]:
        """Prepend retrieved context to a system prompt for grounded answering."""
        ctx = self.assemble_context(query, k)
        if not ctx:
            return system
        instruction = (
            "Use the retrieved context below to answer. Cite sources by their [n] "
            "markers. If the context is insufficient, say so.\n\n" + ctx
        )
        return f"{instruction}\n\n{system}" if system else instruction

    def stats(self) -> Dict[str, int]:
        docs = {c.doc_id for c in self._chunks}
        return {"documents": len(docs), "chunks": len(self._chunks)}
