# OmniHarness Subsystem Build Plan

Status as of the Cargo build-out pass: `omnisystem-core` (Universal Module
System) compiles clean and is genuinely implemented. `ModuleSystemBridge.titan`
bridges its one REST-exposed capability (`/api/v1/plugins/*`) into Titan.
Everything below is confirmed **template-scaffold stub**, not real logic —
each crate currently exports a single generic `Core { data: DashMap<...> }`
placeholder and none of the types the Workspace Rust code actually calls
(`ActorSystem`, `ChessPosition`, `AxiomKernel`, etc.) exist in source yet.

**Ground rule (matches [[omni-language-domains]]):** an Omni-Language file is
a thin, self-contained bridge — usually a REST client — to a Rust/Python
process. It is not a place to hand-transcribe algorithmic logic line-by-line.
So "build subsystem X in Omni-Languages" below means: **the real engine
lives in Rust** (in `OmniHarness/crates/<name>`, replacing the stub), and
the Omni-Language file is the thin control/query surface the rest of the
polyglot system talks to, matching `ModelLoaderBridge.titan` /
`TrainingLoopBridge.sylva` precedent. Writing the actual MCTS search, the
actual proof kernel, etc. as raw Titan/Sylva source would both violate this
project's own established convention and produce slower, unverified code
for zero benefit — Rust is correct here per [[omni-language-domains]]
(Titan = "systems/core... REST-client bridges", not the implementation
language for CPU-bound engines).

## Priority order (dependency-driven, cheapest-context-first)

Ordered so each subsystem either has zero dependencies on another stub, or
depends only on subsystems already built earlier in this list — this
minimizes how much of the plan needs to be held in context at once, and
lets each subsystem be built, compiled, and verified in isolation before
starting the next.

| # | Subsystem | Crate | Omni-Language bridge | Depends on | Real work required |
|---|---|---|---|---|---|
| 1 | CAS (content-addressed store) | `cas` | (internal, no bridge yet — pure library) | none | `CasKey`, `CasStore`: blake3-keyed blob store, put/get/gc |
| 2 | CRDTs | `crdt` | none (library) | none | `GCounter`, `PNCounter`, `LwwRegister`, `OrSet`, `VClock` — standard CRDT algebra, well-specified, no design risk |
| 3 | Mailbox | `mailbox` | none (library) | none | `AgentMailbox`, `MailEnvelope` — bounded async channel wrapper |
| 4 | Package format | `package` | none (library) | `cas` | `PackageReader`/`PackageWriter`, manifest schema |
| 5 | Capability registry | `capability-registry` | none (library) | none | `UniversalCapabilityRegistry`, `CapabilityEntry`, `CapabilitySource`, `EffectRow` — mirrors `omnisystem-core::CapabilityManager` shape, keep consistent with it |
| 6 | Distributed actor system | `actors` | `ActorSystemBridge.titan` (Titan=systems/core) | `mailbox` | `Actor`, `ActorRef`, `ActorContext`, `ActorSystem`, `SupervisionDirective` — **do not duplicate `omnisystem-core::advanced_runtime`'s existing real Actor/ActorRef/ActorSystem**; either re-export those or give this crate a distinct purpose (distributed/remote actors vs. omnisystem-core's in-process ones) before writing new code |
| 7 | Coordinator | `coordinator` | none yet | `actors` | `Coordinator`, `CoordinatorConfig`, `CoordinatorTask` |
| 8 | Fabric | `fabric` | none yet | `actors`, `coordinator` | `CoordinatorActor`, `fabric::catalog`, `fabric::types` |
| 9 | Swarm | `swarm` | `SwarmBridge.titan` (real Tauri commands already call this) | `actors`, `coordinator` | `swarm::hierarchy`, `swarm::role`, `swarm::registry`, `swarm::orchestrator`, `swarm::ledger`, `swarm::assistant`, `TemplateRegistry` |
| 10 | Sandbox | `sns` (sandbox-and-supervision) | `SandboxBridge.titan` | `actors` | `SandboxSupervisor`, `SandboxInfo`, `start_supervisor`, `CapabilityViolation` |
| 11 | P2P core + crypto | `p2p-core`, `p2p-crypto` | `P2PBridge.titan` | `crdt`, `mailbox` | node discovery, encrypted transport — reuse `actors::transport.rs`'s existing `GossipRegistry`/`TransportLayer` groundwork (that file is more complete than `core.rs`, worth checking before writing fresh) |
| 12 | Transfer store | `transfer-store` | none (library) | `cas`, `p2p-crypto` | `EncryptedStore` |
| 13 | Marketplace | `marketplace` | `MarketplaceBridge.titan` | `credits`, `capability-registry` | `MarketplaceState`, listing/registry/reservation/free_tier modules |
| 14 | Credits/billing | `credits` | none (library) | none | ledger, meter, estimator, urv, community modules |
| 15 | Skill compiler | `skill-compiler` | `SkillCompilerBridge.sylva` (compiles/distills — Sylva=ML/training domain) | `capability-registry` | `compile_skill`, `distill`, `CompiledSkill`, integrity verification |
| 16 | Extensions | `extensions` | none (library) | `skill-compiler` | registry, installer, manifest, scanner, `SecurityScanner` |
| 17 | Knowledge graph | `knowledge` | `KnowledgeGraphBridge.titan` (already has real REST routes: `/api/v2/knowledge/*`) | `cas` | `KnowledgeGraph`, `Belief`, `BeliefId`, `Entity`, `Evidence`, `Predicate`, `RelationTarget` — this one has the most REST surface already wired (see `management_api.rs` `/api/v2/knowledge/*`, `/api/v2/reason*`, `/api/v2/beliefs/check`), so bridging pays off immediately once the crate is real |
| 18 | Formal verification kernel | `verify` | `VerificationBridge.axiom` (Axiom=formal verification domain) | none | `AxiomKernel`, `Term`, `Context`, `definitionally_equal` — a real dependently-typed or SMT-backed kernel; highest design risk in this list, do last or with a design spike first |
| 19 | Chess engine | `chess` | `GameEngineBridge.titan`, shared with #20 | none | `ChessPosition`, `ChessColor`, `ChessGameSession`, `Player`/`PlayerKind`, `MaterialEvaluator`, MCTS search, network eval |
| 20 | Go engine | `go` | shares `GameEngineBridge.titan` | none | `GoGameSession`, `Stone`, `GoColor`, `GoMctsConfig`, MCTS search, network eval — structurally near-identical to #19, build both from one shared MCTS core (`go-nn` mirrors `chess`'s `TrainingExample`/evaluator pattern too) to avoid duplicating the search algorithm twice |
| 21 | Failure-finder (fuzzing/chaos) | `failure-finder` | `FailureFinderBridge.titan` | `sns` | `F3Orchestrator`, `CampaignSpec`/`CampaignState`, `FailureReport`, `FuzzStrategy`, `ResourceBudget` |
| 22 | kdb (vector/knowledge db) | `kdb` | none yet | `cas` | `KdbStore`, `KdbRetriever` |
| 23 | hnsw (approx-NN index) | `hnsw` | none (library, used by `kdb`) | none | standard HNSW graph index — well-specified, low design risk |
| 24 | IR (intermediate repr, shared with compiler) | `ir` | n/a — consumed by `Omnisystem/src/compiler/*`, not this Workspace | none | out of OmniHarness scope; belongs to the separate compiler-ecosystem plan already on file |
| 25 | iot-control, remote-desktop, mcp-server, profiler | (already user-named, real-feature crates) | each gets its own bridge once real | varies | these were the *original* 7 you asked to absorb — same "stub scaffold" problem applies; build after the above since Workspace's own compile errors don't currently block on these paths as hard as actors/chess/go/verify/knowledge do |

## Token-efficient build sequencing rules

1. **One crate, one turn.** Each subsystem above is scoped so a single
   focused pass (read stub + call sites → design types → implement → `cargo
   check -p <crate>`) fits without needing the rest of this table in
   context. Re-open only this table's one row, not the whole plan, when
   resuming a specific subsystem.
2. **Compile-scoped, not whole-workspace.** Use `cargo check -p <crate>`
   against just the crate being built, not the full `workspace` binary,
   until the crate itself is clean — the full-workspace error list is huge
   and mostly noise from crates not yet started.
3. **Reuse before writing.** Rows 6, 9, 11, 19/20 explicitly call out
   existing, more-complete code elsewhere in the tree
   (`omnisystem-core::advanced_runtime`, `actors::transport.rs`,
   `go`/`go-nn` symmetry) — check those first; re-exporting or lightly
   adapting existing real code is strictly cheaper than writing fresh code
   that will need to be reconciled with it later.
4. **Bridge only what's REST-reachable.** Don't write an Omni-Language file
   for a crate until it has an actual `management_api.rs` route calling it
   — a bridge for a route that doesn't exist is fabricated, not real (this
   is why `SmartRouterBridge` was skipped this pass: no REST route yet).
5. **Verification gate per subsystem:** a subsystem is "done" only when (a)
   `cargo check -p <crate>` is clean, (b) the `workspace` binary's error
   count for that crate's symbols drops to zero, and (c) if a bridge file
   was written, it points at a route that actually exists and returns real
   data in `management_api.rs`.

## Explicitly out of scope for this document

- `Omnisystem/src/compiler/*` (the 7-language compiler ecosystem) — tracked
  separately in the existing compiler-ecosystem completion plan.
- The `~/.bonsai/*` local-data-directory de-brand (flagged in an earlier
  pass, not started).
