"""
Evolution & self-learning — the substrate's continual-improvement layer.

Four honest, working mechanisms toward "AI that learns and evolves":

  1. TrajectoryStore  — record what agents did and how well it went (experience).
  2. PreferenceStore  — capture chosen/rejected pairs (DPO-ready preference data).
  3. EvolutionaryOptimizer — genetic search over agent configs (system prompt,
     temperature, tools, model) against a pluggable fitness function.
  4. DistillationDatasetBuilder — generate a fine-tuning dataset from a HYBRID of
     many teacher models (optionally judged/merged), and emit a ready-to-run
     training config for a real trainer (Unsloth / Axolotl / llama.cpp / MLX).

Weight-level training itself is delegated to those external GPU trainers; this
layer owns the data, feedback, and search that make training effective.
"""
from __future__ import annotations

import asyncio
import json
import random
import time
from dataclasses import dataclass, field
from typing import Any, Awaitable, Callable, Dict, List, Optional

from .types import LLMFn, AgentSpec

# ── Experience capture ──────────────────────────────────────────────────────


@dataclass
class Trajectory:
    task: str
    agent_id: str
    steps: List[Dict[str, Any]]
    outcome: str
    reward: float
    ts: float = field(default_factory=time.time)


class TrajectoryStore:
    """Records agent runs. Successful ones become few-shot exemplars for RAG."""

    def __init__(self) -> None:
        self._items: List[Trajectory] = []

    def record(self, task: str, agent_id: str, steps: List[Dict[str, Any]], outcome: str, reward: float) -> None:
        self._items.append(Trajectory(task, agent_id, steps, outcome, reward))

    def best(self, k: int = 5, min_reward: float = 0.5) -> List[Trajectory]:
        good = [t for t in self._items if t.reward >= min_reward]
        good.sort(key=lambda t: t.reward, reverse=True)
        return good[:k]

    def as_exemplars(self, k: int = 3) -> str:
        lines = []
        for t in self.best(k):
            lines.append(f"Task: {t.task}\nApproach that scored {t.reward:.2f}:\n{t.outcome}\n")
        return "\n---\n".join(lines)

    def all(self) -> List[Trajectory]:
        return list(self._items)


@dataclass
class Preference:
    prompt: str
    chosen: str
    rejected: str
    ts: float = field(default_factory=time.time)


class PreferenceStore:
    """Chosen/rejected pairs — directly exportable as DPO/ORPO training data."""

    def __init__(self) -> None:
        self._items: List[Preference] = []

    def record(self, prompt: str, chosen: str, rejected: str) -> None:
        self._items.append(Preference(prompt, chosen, rejected))

    def to_jsonl(self) -> str:
        return "\n".join(json.dumps({"prompt": p.prompt, "chosen": p.chosen, "rejected": p.rejected}) for p in self._items)

    def all(self) -> List[Preference]:
        return list(self._items)


# ── Evolutionary optimization of agent configs ──────────────────────────────

FitnessFn = Callable[[AgentSpec], Awaitable[float]]

_PROMPT_MUTATIONS = [
    "Think step by step and verify each step before continuing.",
    "Be concise and precise; avoid unnecessary elaboration.",
    "Consider edge cases and failure modes explicitly.",
    "Prefer simple, robust solutions over clever ones.",
    "State your assumptions before answering.",
    "Double-check facts and cite reasoning.",
]


@dataclass
class EvolutionReport:
    best: AgentSpec
    best_fitness: float
    generations: List[Dict[str, Any]]


class EvolutionaryOptimizer:
    """
    Genetic search over AgentSpec variants. Fitness is an injected async function
    (e.g. average task success from a benchmark). Mutations perturb the system
    prompt, temperature, and tool set; crossover blends two parents.
    """

    def __init__(self, fitness: FitnessFn, population: int = 8, elitism: int = 2,
                 mutation_rate: float = 0.6, seed: Optional[int] = None) -> None:
        self.fitness = fitness
        self.population = population
        self.elitism = elitism
        self.mutation_rate = mutation_rate
        self.rng = random.Random(seed)

    def _mutate(self, spec: AgentSpec, gen: int, idx: int) -> AgentSpec:
        child = spec.clone(id=f"{spec.id}-g{gen}-{idx}")
        if self.rng.random() < self.mutation_rate:
            add = self.rng.choice(_PROMPT_MUTATIONS)
            if add not in child.system:
                child.system = child.system.rstrip() + "\n" + add
        if self.rng.random() < self.mutation_rate:
            child.temperature = round(min(1.5, max(0.0, child.temperature + self.rng.uniform(-0.3, 0.3))), 2)
        return child

    def _crossover(self, a: AgentSpec, b: AgentSpec, gen: int, idx: int) -> AgentSpec:
        # Blend: take a's prompt head + b's prompt tail, average temperature.
        a_lines = a.system.split("\n")
        b_lines = b.system.split("\n")
        cut_a = len(a_lines) // 2
        merged = "\n".join(a_lines[:cut_a] + b_lines[len(b_lines) // 2:])
        return a.clone(id=f"x-g{gen}-{idx}", system=merged,
                       temperature=round((a.temperature + b.temperature) / 2, 2))

    async def _evaluate(self, specs: List[AgentSpec]) -> List[tuple[AgentSpec, float]]:
        scores = await asyncio.gather(*(self.fitness(s) for s in specs))
        return sorted(zip(specs, scores), key=lambda p: p[1], reverse=True)

    async def evolve(self, seed_spec: AgentSpec, generations: int = 5) -> EvolutionReport:
        pop = [self._mutate(seed_spec, 0, i) for i in range(self.population)]
        pop[0] = seed_spec  # keep the seed in the pool
        history: List[Dict[str, Any]] = []
        ranked = await self._evaluate(pop)

        for gen in range(1, generations + 1):
            elites = [s for s, _ in ranked[: self.elitism]]
            children: List[AgentSpec] = list(elites)
            while len(children) < self.population:
                if self.rng.random() < 0.5 and len(ranked) >= 2:
                    a, b = self.rng.sample([s for s, _ in ranked[: max(2, self.population // 2)]], 2)
                    child = self._crossover(a, b, gen, len(children))
                else:
                    parent = self.rng.choice([s for s, _ in ranked[: max(2, self.population // 2)]])
                    child = self._mutate(parent, gen, len(children))
                children.append(child)
            ranked = await self._evaluate(children)
            history.append({"generation": gen, "best_fitness": ranked[0][1], "best_id": ranked[0][0].id})

        best, best_fit = ranked[0]
        return EvolutionReport(best=best, best_fitness=best_fit, generations=history)


# ── Multi-teacher distillation dataset generation ───────────────────────────

_TRAINER_TEMPLATES = {
    "unsloth": {
        "framework": "unsloth",
        "note": "pip install unsloth ; run the generated Python with a chat SFT dataset",
        "command": "python train_unsloth.py --dataset {dataset} --base {base_model} --out {out_dir}",
    },
    "axolotl": {
        "framework": "axolotl",
        "note": "axolotl reads a YAML config; convert dataset to sharegpt/alpaca first",
        "command": "accelerate launch -m axolotl.cli.train {config_yaml}",
    },
    "llama.cpp": {
        "framework": "llama.cpp",
        "note": "finetune a GGUF base with the LoRA finetune example",
        "command": "llama-finetune --model-base {base_model} --train-data {dataset} --lora-out {out_dir}/adapter.gguf",
    },
    "mlx": {
        "framework": "mlx-lm",
        "note": "Apple Silicon LoRA fine-tuning",
        "command": "mlx_lm.lora --model {base_model} --train --data {dataset} --adapter-path {out_dir}",
    },
}


class DistillationDatasetBuilder:
    """
    Build a supervised fine-tuning dataset by distilling a HYBRID of teacher models.

    For each seed prompt, query every teacher; optionally have a judge model pick or
    merge the best response. Emit chat-format JSONL and a ready-to-run trainer config.
    """

    def __init__(self, llm: LLMFn, teachers: List[str], judge_model: Optional[str] = None,
                 system: Optional[str] = None) -> None:
        self.llm = llm
        self.teachers = teachers
        self.judge_model = judge_model
        self.system = system
        self.records: List[Dict[str, Any]] = []

    async def _best_response(self, prompt: str) -> str:
        answers = await asyncio.gather(*(
            self.llm(t, [{"role": "user", "content": prompt}], self.system) for t in self.teachers
        ))
        pairs = list(zip(self.teachers, answers))
        if not self.judge_model:
            # No judge — take the longest well-formed answer as a simple heuristic.
            return max((a for _, a in pairs), key=len)
        proposals = "\n\n".join(f"[{m}]\n{a}" for m, a in pairs)
        judge_prompt = (
            f"User prompt:\n{prompt}\n\nCandidate answers from teacher models:\n{proposals}\n\n"
            "Return the single best, correct, complete answer (you may merge the best parts). "
            "Output only the answer, with no commentary."
        )
        return await self.llm(self.judge_model, [{"role": "user", "content": judge_prompt}], self.system)

    async def build(self, prompts: List[str], concurrency: int = 4) -> int:
        """Distill responses for each prompt. Returns the number of records built."""
        sem = asyncio.Semaphore(concurrency)

        async def one(p: str) -> None:
            async with sem:
                try:
                    answer = await self._best_response(p)
                except Exception as exc:  # noqa: BLE001
                    answer = f"[distillation error: {exc}]"
                msgs = []
                if self.system:
                    msgs.append({"role": "system", "content": self.system})
                msgs.append({"role": "user", "content": p})
                msgs.append({"role": "assistant", "content": answer})
                self.records.append({"messages": msgs})

        await asyncio.gather(*(one(p) for p in prompts))
        return len(self.records)

    def to_jsonl(self) -> str:
        return "\n".join(json.dumps(r, ensure_ascii=False) for r in self.records)

    def write_jsonl(self, path: str) -> str:
        with open(path, "w", encoding="utf-8") as f:
            f.write(self.to_jsonl())
        return path

    def training_config(self, backend: str, base_model: str, out_dir: str = "./distilled-model",
                        dataset_path: str = "./distill.jsonl") -> Dict[str, Any]:
        tpl = _TRAINER_TEMPLATES.get(backend)
        if not tpl:
            raise ValueError(f"unknown trainer backend '{backend}'. Options: {list(_TRAINER_TEMPLATES)}")
        cmd = tpl["command"].format(dataset=dataset_path, base_model=base_model, out_dir=out_dir,
                                    config_yaml=f"{out_dir}/config.yaml")
        return {
            "backend": tpl["framework"],
            "base_model": base_model,
            "teachers": self.teachers,
            "judge_model": self.judge_model,
            "dataset": dataset_path,
            "records": len(self.records),
            "output_dir": out_dir,
            "note": tpl["note"],
            "command": cmd,
        }
