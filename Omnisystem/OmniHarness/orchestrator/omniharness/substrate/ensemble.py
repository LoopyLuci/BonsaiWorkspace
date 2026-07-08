"""
Ensemble / Mixture-of-Agents — query many models at once and combine them.

This is the "hybrid of N models" capability at inference time: fan a prompt across
up to dozens of models (local + API), then aggregate by concatenation, majority
vote, a judge model's synthesis, or layered Mixture-of-Agents (proposers → aggregator).
"""
from __future__ import annotations

import asyncio
import re
from collections import Counter
from typing import Dict, List, Optional

from .types import LLMFn
from .governance import Governor


def _tokens_estimate(text: str) -> int:
    return max(1, len(text) // 4)


async def _ask(llm: LLMFn, gov: Optional[Governor], model: str, prompt: str, system: Optional[str]) -> str:
    if gov:
        gov.check_model(model)
    messages = [{"role": "user", "content": prompt}]
    text = await llm(model, messages, system)
    if gov:
        gov.record_call(model, _tokens_estimate(prompt) + _tokens_estimate(text))
    return text


async def run_ensemble(
    llm: LLMFn,
    prompt: str,
    models: List[str],
    system: Optional[str] = None,
    strategy: str = "judge",           # concat | vote | judge | moa
    judge_model: Optional[str] = None,
    governor: Optional[Governor] = None,
) -> Dict[str, object]:
    """
    Run `prompt` across `models` concurrently and aggregate with `strategy`.
    Returns { answers: {model: text}, final: str, strategy, usage }.
    """
    gov = governor
    if gov:
        gov.checkpoint("ensemble")
    fanout = gov.parallelism(len(models)) if gov else len(models)
    sem = asyncio.Semaphore(max(1, fanout))

    async def one(m: str) -> tuple[str, str]:
        async with sem:
            try:
                return m, await _ask(llm, gov, m, prompt, system)
            except Exception as exc:  # noqa: BLE001 — a failed model shouldn't sink the ensemble
                return m, f"[error: {exc}]"

    pairs = await asyncio.gather(*(one(m) for m in models))
    answers = {m: t for m, t in pairs}
    valid = {m: t for m, t in answers.items() if not t.startswith("[error:")}

    final = await _aggregate(llm, gov, prompt, system, valid, strategy, judge_model)
    return {
        "answers": answers,
        "final": final,
        "strategy": strategy,
        "usage": gov.usage.snapshot() if gov else None,
    }


async def _aggregate(
    llm: LLMFn,
    gov: Optional[Governor],
    prompt: str,
    system: Optional[str],
    answers: Dict[str, str],
    strategy: str,
    judge_model: Optional[str],
) -> str:
    if not answers:
        return "[no successful model responses]"
    texts = list(answers.values())

    if strategy == "concat":
        return "\n\n".join(f"### {m}\n{t}" for m, t in answers.items())

    if strategy == "vote":
        # Majority vote over a normalized fingerprint of each answer.
        def norm(s: str) -> str:
            return re.sub(r"\s+", " ", s.strip().lower())[:400]
        counts = Counter(norm(t) for t in texts)
        winner_norm, _ = counts.most_common(1)[0]
        for t in texts:
            if norm(t) == winner_norm:
                return t
        return texts[0]

    if strategy == "moa":
        return await layered_moa(llm, gov, prompt, system, list(answers.keys()),
                                 judge_model or list(answers.keys())[0], layers=1, seeded=answers)

    # default: judge synthesis
    judge = judge_model or next(iter(answers.keys()))
    proposals = "\n\n".join(f"[Candidate {i+1} — {m}]\n{t}" for i, (m, t) in enumerate(answers.items()))
    synth_prompt = (
        f"You are an expert judge. The user asked:\n\n{prompt}\n\n"
        f"Here are candidate answers from several models:\n\n{proposals}\n\n"
        "Synthesize the single best, correct, and complete answer. Reconcile "
        "disagreements, keep what is well-supported, and drop anything wrong."
    )
    if gov:
        gov.check_model(judge)
    result = await llm(judge, [{"role": "user", "content": synth_prompt}], system)
    if gov:
        gov.record_call(judge, _tokens_estimate(synth_prompt) + _tokens_estimate(result))
    return result


async def layered_moa(
    llm: LLMFn,
    gov: Optional[Governor],
    prompt: str,
    system: Optional[str],
    proposer_models: List[str],
    aggregator_model: str,
    layers: int = 2,
    seeded: Optional[Dict[str, str]] = None,
) -> str:
    """
    Layered Mixture-of-Agents: each layer, all proposers answer using the previous
    layer's aggregated context; a final aggregator produces the answer.
    """
    context = ""
    proposals: Dict[str, str] = seeded or {}

    for layer in range(layers):
        if gov:
            gov.checkpoint(f"moa-layer-{layer}")
        if proposals and layer == 0 and seeded:
            pass  # reuse seeded proposals for the first layer
        else:
            layer_prompt = prompt if not context else (
                f"{prompt}\n\nPrevious aggregated responses to improve upon:\n{context}"
            )
            fanout = gov.parallelism(len(proposer_models)) if gov else len(proposer_models)
            sem = asyncio.Semaphore(max(1, fanout))

            async def one(m: str) -> tuple[str, str]:
                async with sem:
                    return m, await _ask(llm, gov, m, layer_prompt, system)

            proposals = dict(await asyncio.gather(*(one(m) for m in proposer_models)))
        context = "\n\n".join(f"- {t}" for t in proposals.values())

    agg_prompt = (
        f"Original request:\n{prompt}\n\nProposed responses from a panel of models:\n"
        f"{context}\n\nProduce the definitive, synthesized final answer."
    )
    if gov:
        gov.check_model(aggregator_model)
    final = await llm(aggregator_model, [{"role": "user", "content": agg_prompt}], system)
    if gov:
        gov.record_call(aggregator_model, _tokens_estimate(agg_prompt) + _tokens_estimate(final))
    return final
