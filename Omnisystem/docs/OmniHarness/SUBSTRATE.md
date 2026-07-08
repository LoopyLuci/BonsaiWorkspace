# OmniHarness Substrate — Swarms, RAG, Ensembles, Governance & Evolution

The Substrate turns OmniHarness from a universal *harness* into a universal
*substrate* for safe, controllable, self-improving AI. It provides five capability
families, each implemented across the polyglot stack and wrapped in a governance
layer that enforces **absolute user control**.

| Capability | What it does |
|-----------|--------------|
| **Agent Swarm** | Coordinate many specialized agents: pipeline, parallel/map-reduce, orchestrator-workers, debate/consensus |
| **RAG** | Ingest → chunk → embed → retrieve → assemble grounded context for any model |
| **Ensemble / Mixture-of-Agents** | Blend up to dozens of models (concat, vote, judge synthesis, layered MoA) — the "hybrid of 20 models" at inference time |
| **Governance** | Resource budgets, capability policy, hash-chained audit log, live kill switch |
| **Evolution & self-learning** | Trajectory + preference capture, evolutionary agent-config optimization, and multi-teacher distillation datasets |

---

## Polyglot implementation

The substrate is implemented in the right language for each concern — matching
OmniHarness's polyglot architecture:

| Language | Module | Role |
|----------|--------|------|
| **Python** | `orchestrator/omniharness/substrate/` | Executable reference engines (swarm, rag, ensemble, evolution, governance) — **19/19 self-tests pass** |
| **Rust** | `kernel/src/substrate.rs` | Kernel-level governance/audit/metering trust anchor — **3/3 unit tests pass** |
| **Clojure** | `clj-orchestrator/src/omniharness/{swarm,ensemble,governance}.clj` | Data-oriented orchestration + hash-chained audit |
| **Titan** | `omni-integration/SubstrateCore.titan` | Systems coordinator: governs and dispatches runs |
| **Aether** | `omni-integration/SwarmActors.aether` | Swarm agents as concurrent actors over a blackboard |
| **Axiom** | `omni-integration/SubstrateGovernance.axiom` | Formal safety theorems (budget, capability, audit, kill switch, no-exfiltration, bounded autonomy) |
| **Sylva** | `omni-integration/DistillationEngine.sylva` | Distillation, preference learning, evolution as ML pipelines |
| **Helix** | `omni-integration/SubstrateCompute.helix` | GPU kernels: batch embed, cosine retrieve, ensemble softmax, distill KL |
| **Vera** | `omni-integration/SubstratePanel.vera` | Control UI: swarm designer, ensemble console, distillation studio, governance bar |
| **Nexus** | `omni-integration/SubstrateLayout.nexus` | Responsive substrate dashboard layout |
| **TypeScript** | `vscode-omnisystem/src/harness/OmniHarnessClient.ts` + commands | In-editor control surface |

Every engine takes an **injected `LLMFn`** `(model, messages, system) -> text`, so
the logic is provider-agnostic and testable without any provider SDK. The Python
FastAPI server wires the real `ModelRouter`; tests inject a fake generator.

---

## Governance — absolute control (the safety spine)

No autonomous run is unbounded. Every swarm/ensemble/evolution run executes inside
a **Governor** that enforces:

- **Budgets** — `max_model_calls`, `max_tokens`, `max_cost_usd`, `max_steps`,
  `max_wallclock_s`, `max_parallel`. Exceeding any raises `BudgetExceeded`.
- **Capability policy** — allow-lists for models and tools, deny-lists, network and
  filesystem-write gates, and a hard agent cap. Violations raise `PolicyViolation`.
- **Audit log** — an append-only, SHA-256 hash-chained record of every model call
  and decision. `verify()` proves it was not tampered with.
- **Kill switch** — a cooperative stop checked at every step; tripping it raises
  `Aborted` before the next model call.

The same guarantees are re-expressed as machine-checkable theorems in
`SubstrateGovernance.axiom` and enforced independently in the Rust kernel.

---

## REST API

```
POST /api/swarm/run       { topology, task, agents[], rounds, budget?, policy? }
POST /api/ensemble/run    { prompt, models[], strategy, judge_model?, budget?, policy? }
POST /api/rag/ingest      { doc_id, text, metadata? }
POST /api/rag/query       { query, k?, doc_id? }
POST /api/distill/build   { prompts[], teachers[], judge_model?, backend?, base_model? }
```

Each swarm/ensemble/distill response includes a `governance` report (usage, audit
validity, budget, kill state).

**From VS Code:** Command Palette → **“Omnisystem / Substrate: Run Agent Swarm”**
or **“Blend Models (Ensemble / Mixture-of-Agents)”**.

---

## Training from the ground up, or from a hybrid of many models

`DistillationDatasetBuilder` is the honest on-ramp to custom models:

1. **Generate data** — for each seed prompt, query every teacher model (a hybrid of
   up to N), then optionally have a judge model select or merge the best answer.
2. **Emit a dataset** — chat-format JSONL ready for supervised fine-tuning.
3. **Emit a training config** — a ready-to-run command for a real trainer:
   **Unsloth**, **Axolotl**, **llama.cpp**, or **MLX**.

```python
builder = DistillationDatasetBuilder(llm, teachers=["anthropic/claude-sonnet-4-6",
                                                     "gpt-4o", "local/qwen3.5-0.8b", ...],
                                     judge_model="anthropic/claude-opus-4-8")
await builder.build(prompts)
builder.write_jsonl("distill.jsonl")
cfg = builder.training_config("unsloth", base_model="qwen2.5-3b")  # -> command to run
```

The `StudentLM` architecture and distillation/DPO objectives are expressed in
`DistillationEngine.sylva` for the from-scratch path. **Actual weight training runs
on a real GPU trainer** — the substrate owns the data, feedback, and search that
make that training effective; it does not reimplement CUDA-level training in-process.

**Continual self-learning without retraining:** `TrajectoryStore` captures
successful agent runs (which become RAG exemplars), and `PreferenceStore` captures
chosen/rejected pairs (DPO/ORPO-ready), so the system improves in-context over time.

---

## Evolution

`EvolutionaryOptimizer` runs a genetic search over agent configurations — system
prompt, temperature, tools, model — against a pluggable async fitness function
(e.g. average task success on a benchmark). Mutation perturbs prompts/params,
crossover blends parents, elitism preserves the best. Returns the fittest agent
plus per-generation history.

---

## Status & honest boundaries

- **Verified now:** Python engines (19/19 self-tests), Rust governance (3/3 unit
  tests). Both run here with no network or provider keys.
- **Source-complete, not run here:** the 7 Omni-Language modules (the Omni compiler
  is still maturing) and the Clojure modules (no Clojure runtime in this
  environment; they use only libraries already in `project.clj`).
- **Delegated by design:** GPU weight-training is handed to Unsloth/Axolotl/
  llama.cpp/MLX via generated configs — the substrate does not claim to train
  neural networks in-process.

**See also:** [MCP.md](MCP.md) · [VSCODE_PANEL.md](VSCODE_PANEL.md) · [ARCHITECTURE.md](ARCHITECTURE.md) · [MODELS.md](MODELS.md)
