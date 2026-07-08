"""Knowledge graph — nodes, edges, entity extraction, path finding."""
from __future__ import annotations

import json
import re
import time
from collections import deque
from dataclasses import dataclass, field
from typing import Any


@dataclass
class Node:
    id:         str
    label:      str
    type:       str              # PERSON, ORG, LOC, DATE, CONCEPT, ...
    properties: dict[str, Any] = field(default_factory=dict)
    created_at: float           = field(default_factory=time.time)


@dataclass
class Edge:
    id:         str
    source:     str              # node id
    target:     str              # node id
    relation:   str              # e.g. "works_at", "located_in", "related_to"
    properties: dict[str, Any] = field(default_factory=dict)
    created_at: float           = field(default_factory=time.time)


class KnowledgeGraph:
    def __init__(self) -> None:
        self._nodes: dict[str, Node] = {}
        self._edges: dict[str, Edge] = {}
        self._adj:   dict[str, list[str]] = {}   # node_id → list of edge ids

    # ── Nodes ─────────────────────────────────────────────────────

    def add_node(self, id: str, label: str, type: str = "CONCEPT", properties: dict | None = None) -> Node:
        node = Node(id=id, label=label, type=type, properties=properties or {})
        self._nodes[id]   = node
        self._adj.setdefault(id, [])
        return node

    def get_node(self, id: str) -> Node | None:
        return self._nodes.get(id)

    def remove_node(self, id: str) -> bool:
        if id not in self._nodes:
            return False
        # Remove incident edges
        for edge_id in list(self._adj.get(id, [])):
            self._edges.pop(edge_id, None)
        for edges in self._adj.values():
            for eid in list(edges):
                if eid in self._edges:
                    e = self._edges[eid]
                    if e.source == id or e.target == id:
                        edges.remove(eid)
                        self._edges.pop(eid, None)
        del self._adj[id]
        del self._nodes[id]
        return True

    def all_nodes(self) -> list[Node]:
        return list(self._nodes.values())

    # ── Edges ─────────────────────────────────────────────────────

    def add_edge(self, source: str, target: str, relation: str, properties: dict | None = None) -> Edge:
        import uuid
        eid  = str(uuid.uuid4())
        edge = Edge(id=eid, source=source, target=target, relation=relation, properties=properties or {})
        self._edges[eid] = edge
        self._adj.setdefault(source, []).append(eid)
        self._adj.setdefault(target, []).append(eid)
        return edge

    def remove_edge(self, edge_id: str) -> bool:
        edge = self._edges.pop(edge_id, None)
        if not edge:
            return False
        for adj in [self._adj.get(edge.source, []), self._adj.get(edge.target, [])]:
            if edge_id in adj:
                adj.remove(edge_id)
        return True

    def all_edges(self) -> list[Edge]:
        return list(self._edges.values())

    # ── Traversal ──────────────────────────────────────────────────

    def find_neighbors(self, node_id: str, depth: int = 1) -> list[Node]:
        visited = set()
        queue   = deque([(node_id, 0)])
        result  = []
        while queue:
            nid, d = queue.popleft()
            if nid in visited or d > depth:
                continue
            visited.add(nid)
            if nid != node_id and nid in self._nodes:
                result.append(self._nodes[nid])
            if d < depth:
                for eid in self._adj.get(nid, []):
                    edge = self._edges.get(eid)
                    if edge:
                        nbr = edge.target if edge.source == nid else edge.source
                        if nbr not in visited:
                            queue.append((nbr, d + 1))
        return result

    def shortest_path(self, source: str, target: str) -> list[str]:
        """BFS shortest path. Returns list of node ids."""
        if source not in self._nodes or target not in self._nodes:
            return []
        visited = {source: None}
        queue   = deque([source])
        while queue:
            cur = queue.popleft()
            if cur == target:
                path = []
                while cur is not None:
                    path.append(cur)
                    cur = visited[cur]
                return list(reversed(path))
            for eid in self._adj.get(cur, []):
                edge = self._edges.get(eid)
                if edge:
                    nbr = edge.target if edge.source == cur else edge.source
                    if nbr not in visited:
                        visited[nbr] = cur
                        queue.append(nbr)
        return []

    # ── Entity extraction ─────────────────────────────────────────

    def extract_entities(self, text: str) -> list[Node]:
        """Simple regex-based NER for PERSON, ORG, LOC, DATE."""
        patterns = [
            (r"\b([A-Z][a-z]+ [A-Z][a-z]+)\b",           "PERSON"),
            (r"\b((?:Inc\.|Corp\.|Ltd\.|LLC|GmbH|[A-Z]{2,}))\b", "ORG"),
            (r"\b(\d{4}-\d{2}-\d{2}|\d{1,2}/\d{1,2}/\d{4})\b",  "DATE"),
            (r"\b(https?://[^\s]+)\b",                     "URL"),
        ]
        found = []
        import uuid
        for pattern, etype in patterns:
            for m in re.finditer(pattern, text):
                label = m.group(1)
                nid   = label.lower().replace(" ", "_")
                if nid not in self._nodes:
                    n = self.add_node(nid, label, etype)
                    found.append(n)
        return found

    # ── Serialization ─────────────────────────────────────────────

    def export_json(self) -> str:
        return json.dumps({
            "nodes": [{"id": n.id, "label": n.label, "type": n.type, "properties": n.properties}
                      for n in self._nodes.values()],
            "edges": [{"id": e.id, "source": e.source, "target": e.target,
                       "relation": e.relation, "properties": e.properties}
                      for e in self._edges.values()],
        }, indent=2)

    def import_json(self, data: str) -> None:
        obj = json.loads(data)
        for n in obj.get("nodes", []):
            self.add_node(n["id"], n["label"], n.get("type", "CONCEPT"), n.get("properties", {}))
        for e in obj.get("edges", []):
            self._edges[e["id"]] = Edge(
                id=e["id"], source=e["source"], target=e["target"],
                relation=e["relation"], properties=e.get("properties", {}),
            )
            self._adj.setdefault(e["source"], []).append(e["id"])
            self._adj.setdefault(e["target"], []).append(e["id"])

    def stats(self) -> dict:
        return {"nodes": len(self._nodes), "edges": len(self._edges)}
