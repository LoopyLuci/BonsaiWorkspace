"""
Agent Swarm — coordinate many specialized agents toward one goal.

Topologies:
  • pipeline              — agents in sequence, each refines the previous output
  • parallel (map-reduce) — all agents attack the task; a reducer merges results
  • orchestrator-workers  — a lead agent decomposes the task, workers execute in
                            parallel, the lead synthesizes
  • debate                — agents critique each other over rounds toward consensus

Everything runs inside a Governor (budgets, policy, audit, kill switch) and shares
a Blackboard so agents can post and read intermediate findings.
"""
from __future__ import annotations

import asyncio
import json
import re
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from .types import LLMFn, AgentSpec
from .governance import Governor


class Blackboard:
    """A shared, append-logged workspace for swarm agents."""

    def __init__(self) -> None:
        self._data: Dict[str, Any] = {}
        self._log: List[Dict[str, Any]] = []

    def post(self, agent: str, key: str, value: Any) -> None:
        self._data[key] = value
        self._log.append({"agent": agent, "key": key, "value": value})

    def read(self, key: str, default: Any = None) -> Any:
        return self._data.get(key, default)

    def snapshot(self) -> Dict[str, Any]:
        return dict(self._data)

    def log(self) -> List[Dict[str, Any]]:
        return list(self._log)


@dataclass
class SwarmResult:
    output: str
    topology: str
    steps: List[Dict[str, Any]] = field(default_factory=list)
    blackboard: Dict[str, Any] = field(default_factory=dict)
    usage: Optional[Dict[str, Any]] = None


def _tokens(text: str) -> int:
    return max(1, len(text) // 4)


class SwarmCoordinator:
    def __init__(self, llm: LLMFn, governor: Optional[Governor] = None) -> None:
        self.llm = llm
        self.gov = governor or Governor()
        self.board = Blackboard()

    async def _run_agent(self, agent: AgentSpec, prompt: str) -> str:
        self.gov.checkpoint(f"agent:{agent.id}")
        self.gov.check_model(agent.model)
        system = agent.system
        text = await self.llm(agent.model, [{"role": "user", "content": prompt}], system)
        self.gov.record_call(agent.model, _tokens(prompt) + _tokens(text))
        self.board.post(agent.id, f"{agent.id}:last", text)
        return text

    async def run(self, topology: str, agents: List[AgentSpec], task: str, **kw) -> SwarmResult:
        self.gov.audit.append("swarm_start", {"topology": topology, "agents": [a.id for a in agents], "task": task[:200]})
        if topology == "pipeline":
            res = await self._pipeline(agents, task)
        elif topology == "parallel":
            res = await self._parallel(agents, task, kw.get("reducer"))
        elif topology == "orchestrator":
            res = await self._orchestrator_workers(agents, task)
        elif topology == "debate":
            res = await self._debate(agents, task, rounds=int(kw.get("rounds", 2)))
        else:
            raise ValueError(f"unknown topology: {topology}")
        res.blackboard = self.board.snapshot()
        res.usage = self.gov.usage.snapshot()
        self.gov.audit.append("swarm_end", {"topology": topology, "usage": res.usage})
        return res

    # ── Topologies ───────────────────────────────────────────────────────────

    async def _pipeline(self, agents: List[AgentSpec], task: str) -> SwarmResult:
        steps: List[Dict[str, Any]] = []
        current = task
        for agent in agents:
            prompt = f"Task: {task}\n\nWorking material from the previous stage:\n{current}\n\nApply your expertise and produce the improved result."
            out = await self._run_agent(agent, prompt if steps else task)
            steps.append({"agent": agent.id, "output": out})
            current = out
        return SwarmResult(output=current, topology="pipeline", steps=steps)

    async def _parallel(self, agents: List[AgentSpec], task: str, reducer: Optional[AgentSpec]) -> SwarmResult:
        fanout = self.gov.parallelism(len(agents))
        sem = asyncio.Semaphore(fanout)

        async def one(a: AgentSpec):
            async with sem:
                return {"agent": a.id, "output": await self._run_agent(a, task)}

        steps = list(await asyncio.gather(*(one(a) for a in agents)))
        combined = "\n\n".join(f"### {s['agent']}\n{s['output']}" for s in steps)
        if reducer:
            merged = await self._run_agent(
                reducer,
                f"Task: {task}\n\nIndependent results from the swarm:\n{combined}\n\nMerge into one coherent, de-duplicated result.",
            )
            return SwarmResult(output=merged, topology="parallel", steps=steps)
        return SwarmResult(output=combined, topology="parallel", steps=steps)

    async def _orchestrator_workers(self, agents: List[AgentSpec], task: str) -> SwarmResult:
        lead = next((a for a in agents if a.role == "orchestrator"), agents[0])
        workers = [a for a in agents if a is not lead] or agents

        plan_prompt = (
            f"You are the orchestrator. Break this task into 2-5 independent subtasks a team can do in parallel.\n\n"
            f"Task: {task}\n\nReturn ONLY a JSON array of subtask strings, e.g. [\"...\", \"...\"]."
        )
        plan_text = await self._run_agent(lead, plan_prompt)
        subtasks = _parse_json_list(plan_text) or [task]
        self.board.post(lead.id, "plan", subtasks)

        fanout = self.gov.parallelism(len(subtasks))
        sem = asyncio.Semaphore(fanout)

        async def do(i: int, sub: str):
            worker = workers[i % len(workers)]
            async with sem:
                out = await self._run_agent(worker, f"Subtask: {sub}\n\nOverall goal: {task}")
                return {"agent": worker.id, "subtask": sub, "output": out}

        results = list(await asyncio.gather(*(do(i, s) for i, s in enumerate(subtasks))))
        findings = "\n\n".join(f"[{r['subtask']}] → {r['output']}" for r in results)
        synthesis = await self._run_agent(
            lead, f"Task: {task}\n\nWorker results:\n{findings}\n\nSynthesize the final deliverable."
        )
        steps = [{"agent": lead.id, "output": plan_text, "subtasks": subtasks}] + results + [{"agent": lead.id, "output": synthesis}]
        return SwarmResult(output=synthesis, topology="orchestrator", steps=steps)

    async def _debate(self, agents: List[AgentSpec], task: str, rounds: int) -> SwarmResult:
        steps: List[Dict[str, Any]] = []
        positions: Dict[str, str] = {}
        # Opening statements.
        for a in agents:
            positions[a.id] = await self._run_agent(a, f"Question: {task}\n\nGive your best answer with reasoning.")
            steps.append({"round": 0, "agent": a.id, "output": positions[a.id]})
        # Critique rounds.
        for r in range(1, rounds + 1):
            others_view = lambda me: "\n\n".join(f"{aid}: {txt}" for aid, txt in positions.items() if aid != me)
            new_positions: Dict[str, str] = {}
            for a in agents:
                prompt = (
                    f"Question: {task}\n\nOther agents' current answers:\n{others_view(a.id)}\n\n"
                    f"Your previous answer:\n{positions[a.id]}\n\n"
                    "Critique the others, correct yourself if warranted, and give your refined answer."
                )
                new_positions[a.id] = await self._run_agent(a, prompt)
                steps.append({"round": r, "agent": a.id, "output": new_positions[a.id]})
            positions = new_positions
        # Consensus synthesis by the first agent acting as chair.
        chair = agents[0]
        summary = await self._run_agent(
            chair,
            f"Question: {task}\n\nFinal positions:\n" + "\n\n".join(f"{k}: {v}" for k, v in positions.items()) +
            "\n\nState the consensus answer (or the best-supported answer if no consensus).",
        )
        return SwarmResult(output=summary, topology="debate", steps=steps)


def _parse_json_list(text: str) -> Optional[List[str]]:
    m = re.search(r"\[.*\]", text, re.DOTALL)
    if not m:
        return None
    try:
        val = json.loads(m.group(0))
        if isinstance(val, list):
            return [str(x) for x in val]
    except json.JSONDecodeError:
        return None
    return None
