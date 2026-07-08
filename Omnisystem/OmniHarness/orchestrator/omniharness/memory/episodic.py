"""Episodic conversation memory backed by SQLite."""
from __future__ import annotations

import json
import time
from dataclasses import dataclass
from typing import Any

import aiosqlite

from ..models.base import ChatMessage


@dataclass
class Turn:
    id:         int
    session_id: str
    role:       str
    content:    str
    metadata:   dict
    created_at: float


class EpisodicMemory:
    def __init__(self, db_path: str = "data/episodes.db") -> None:
        self._path = db_path
        self._db:  aiosqlite.Connection | None = None

    async def init(self) -> None:
        import os
        os.makedirs(os.path.dirname(os.path.abspath(self._path)), exist_ok=True)
        self._db = await aiosqlite.connect(self._path)
        await self._db.execute("""
            CREATE TABLE IF NOT EXISTS turns (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role       TEXT NOT NULL,
                content    TEXT NOT NULL,
                metadata   TEXT DEFAULT '{}',
                created_at REAL NOT NULL
            )
        """)
        await self._db.execute("CREATE INDEX IF NOT EXISTS idx_session ON turns(session_id, created_at)")
        await self._db.commit()

    async def add_turn(
        self,
        session_id: str,
        role:       str,
        content:    str,
        metadata:   dict | None = None,
    ) -> int:
        async with self._db.execute(
            "INSERT INTO turns (session_id, role, content, metadata, created_at) VALUES (?, ?, ?, ?, ?)",
            (session_id, role, content, json.dumps(metadata or {}), time.time()),
        ) as cur:
            rowid = cur.lastrowid
        await self._db.commit()
        return rowid

    async def get_history(
        self,
        session_id: str,
        max_turns:  int = 50,
    ) -> list[ChatMessage]:
        async with self._db.execute(
            "SELECT role, content FROM turns WHERE session_id = ? ORDER BY created_at DESC LIMIT ?",
            (session_id, max_turns),
        ) as cur:
            rows = await cur.fetchall()
        return [ChatMessage(role=r[0], content=r[1]) for r in reversed(rows)]

    async def search_history(self, session_id: str, query: str) -> list[ChatMessage]:
        """Simple keyword search over past turns."""
        q = f"%{query}%"
        async with self._db.execute(
            "SELECT role, content FROM turns WHERE session_id = ? AND content LIKE ? ORDER BY created_at",
            (session_id, q),
        ) as cur:
            rows = await cur.fetchall()
        return [ChatMessage(role=r[0], content=r[1]) for r in rows]

    async def get_all_sessions(self) -> list[str]:
        async with self._db.execute(
            "SELECT DISTINCT session_id FROM turns ORDER BY MAX(created_at) DESC"
        ) as cur:
            rows = await cur.fetchall()
        return [r[0] for r in rows]

    async def delete_session(self, session_id: str) -> int:
        async with self._db.execute(
            "DELETE FROM turns WHERE session_id = ?", (session_id,)
        ) as cur:
            count = cur.rowcount
        await self._db.commit()
        return count

    async def summarize_if_long(
        self,
        session_id: str,
        router: Any,   # ModelRouter
        model_id: str = "claude-haiku-4-5-20251001",
        threshold_turns: int = 40,
        keep_recent: int = 10,
    ) -> bool:
        """Summarize old turns into a single summary message when history > threshold."""
        from ..models.base import ChatRequest
        history = await self.get_history(session_id, max_turns=1000)
        if len(history) <= threshold_turns:
            return False

        old_turns  = history[:-keep_recent]
        recent     = history[-keep_recent:]

        combined = "\n".join(f"{m.role}: {m.content}" for m in old_turns)
        req = ChatRequest(
            model_id=model_id,
            messages=[ChatMessage(role="user", content=f"Summarize this conversation concisely:\n\n{combined[:8000]}")],
            max_tokens=512,
            temperature=0.3,
        )
        resp = await router.chat(req)
        summary = resp.content

        # Delete old turns and insert summary
        await self._db.execute(
            "DELETE FROM turns WHERE session_id = ? AND id NOT IN (SELECT id FROM turns WHERE session_id = ? ORDER BY created_at DESC LIMIT ?)",
            (session_id, session_id, keep_recent),
        )
        await self._db.execute(
            "INSERT INTO turns (session_id, role, content, metadata, created_at) VALUES (?, ?, ?, ?, ?)",
            (session_id, "system", f"[Conversation summary]: {summary}", json.dumps({"summarized": True}), time.time() - 1),
        )
        await self._db.commit()
        return True

    async def close(self) -> None:
        if self._db:
            await self._db.close()
