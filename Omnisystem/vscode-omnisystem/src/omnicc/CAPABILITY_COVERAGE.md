# Omni-Language Capability Coverage Specification

## What "integrate all 1000+ languages' capabilities into the 7 Omni-Languages" means concretely

It does **not** mean bundling 1000 language runtimes. It means: the **OmniCC
Universal Language IR (ULIR)** is rich enough to represent any construct any
language can express, and the 7 Omni-Languages are **complete targets** for
that IR — every capability category ULIR can carry has a faithful generator
into whichever Omni-Language owns that capability. When that holds, any of the
1000+ registered languages can be lowered to ULIR and re-expressed in the
Omni-Languages without silent capability loss. That is the real, bounded,
*verifiable* version of the goal.

The mechanism already exists and is genuine:
- `ULIR.ts` — paradigm-agnostic IR: functions/methods/lambdas/closures/
  generators/coroutines/macros; classes/structs/interfaces/traits/mixins/
  protocols; enums/unions/tagged-unions/records/tuples; type-aliases/newtypes/
  opaque-types; actors/channels/threads; theorems/proofs/axioms/invariants;
  tests/benchmarks; widgets (component/layout/style/event); data (table/view/
  schema/query). Plus full statement + expression + type + generics models.
- `LanguageRegistry.ts` — ~250 explicit language defs + family defaults for the
  remaining 750+, each tagged with paradigms / typing / memory / features.
- `ConversionEngine.ts` — source → family parser → ULIR → family generator →
  target. The 7 Omni-Languages are family `omni`, handled by
  `families/OmniLanguageHandler.ts`.

## Capability → owning Omni-Language (the routing contract)

| ULIR capability category | Owning Omni-Language | Rationale (see [[omni-language-domains]]) |
|---|---|---|
| functions, structs, enums, traits, generics, type-aliases, unions, records, namespaces, constants, variables, classes/actors, macros | **Titan** | systems/core — also the universal `default` fallback target, so it must be capability-complete |
| ML models, layers, training pipelines (tensor/autodiff constructs) | **Sylva** | Python/ML equivalent |
| tables, views, schemas, queries, actors as distributed/cloud state, channels | **Aether** | databases & cloud |
| widget-component, widget-event, UI render trees | **Vera** | UI |
| widget-layout, responsive/breakpoint/grid constraints | **Nexus** | layout |
| shaders, compute pipelines (vertex/fragment/compute) | **Helix** | GPU/compute-kernel DSL |
| theorem, proof, axiom-decl, invariant | **Axiom** | formal verification |

## Coverage status (as of this pass)

**Fixed this pass** — real capability-loss defects in `OmniLanguageHandler.ts`:
1. **Silent truncation removed.** Six generators (Vera, Nexus, Helix, Aether,
   Axiom, Sylva) capped output with `.slice(0, N)`, silently dropping every
   unit past 3–5 when converting any non-trivial module. All caps removed —
   conversions now emit *all* units, not an arbitrary prefix.
2. **Titan made capability-complete.** `generateTitan` (the systems/core
   catch-all *and* the `default` target for every unmatched language) handled
   only struct/enum/trait/class and mis-emitted everything else as a bare `fn`.
   It now faithfully lowers: type-alias/newtype/opaque-type → `type`;
   constant → `const` (value preserved from source, not fabricated);
   variable/field/property → `let [mut]`; interface/protocol/mixin → `trait`;
   union/tagged-union/record/tuple-type → `enum`/`struct`; namespace/module →
   `mod` (recursing so nested items aren't lost); macro → `#[macro] fn`;
   theorem/proof/invariant → checked `assert_*` fn (guarantee preserved when
   the target isn't Axiom).

**Verification:** `tsc --noEmit` across the whole extension = 0 errors.

## Remaining coverage work (verifiable, prioritized)

Each row is a discrete, testable gap. "Done" = a round-trip conversion of a
representative source file through that capability preserves it with
confidence ≥ `medium` and no dropped units.

1. **Body translation fidelity** (`BodyTranslator.ts`, 850 LOC) — statement/
   expression lowering is the deepest surface; audit ULIR `ULIRStatementKind`
   / `ULIRExpressionKind` coverage per Omni-Language target (e.g. `match`,
   `defer`, `yield`, `async-stmt`, pattern destructuring).
2. **Sylva/Helix/Nexus generators are template-shaped** — they emit a
   plausible skeleton (fixed architecture, fixed breakpoints, fixed shader I/O)
   rather than deriving structure from the actual IR units. Drive them from the
   real `model`/`layer`, layout, and shader units the parser extracts.
3. **Reverse direction (Omni-Language → 1000+ langs)** — `parseOmniLanguage`
   extracts units; confirm each Omni-Language construct (actor, theorem, model,
   shader, component, layout) round-trips *out* to at least the C/Python/
   functional families without loss.
4. **Data/query capability into Aether** — ULIR has `table`/`view`/`schema`/
   `query-unit`; wire these to Aether generation (currently Aether only emits
   actors), so SQL/GraphQL/Cypher sources land in the DB/cloud language.
5. **Coverage test harness** — a fixture-driven test that feeds one
   representative file per `LanguageFamily` through ULIR → each of the 7
   Omni-Languages and asserts unit-count preservation + confidence floor. This
   is what turns "perfectly functional for any situation" from an aspiration
   into a CI gate.

## Explicitly not claimed

- Not "perfect for 100 years" — that's unverifiable. What *is* delivered: the
  IR + routing contract above, the truncation/data-loss fixes, and a test-gated
  path to close the remaining rows.
- Semantic equivalence of executable behavior across paradigms (e.g. a Prolog
  logic program → Titan imperative) is best-effort by nature; ULIR preserves
  the original source per-unit (`originalSource`) as the fidelity fallback.
