# Omni-Language Capability Map

**Goal:** each of the 7 Omni-Languages must *natively* possess every language
capability of the world languages in its domain — so that anything expressible
in Rust/C/Zig is expressible in **Titan**, anything in Python/JS/TS/Ruby in
**Sylva**, etc. This is a **language-design** map (grammar + type system +
semantics + stdlib), **not** transpilation and **not** OmniCC (which is a
separate cross-converter). This document is the specification that drives
building those capabilities into the actual frontends
(`src/compiler/frontend/*`), backends, runtime, and standard libraries.

**Status legend:** ✓ present · ◐ partial/shallow · ✗ missing (build target)

**Method:** capabilities are grouped by category so each language's coverage is
*complete by construction* (every category enumerated), with exemplar source
languages named. The long tail of 1000+ languages collapses into these
categories — a new esoteric language introduces no capability category not
already covered by a mainstream exemplar.

**Correction (2026-07-03, later pass):** the baseline below was written from a
survey of `src/compiler/frontend/*.titan` — an inert, non-executing frontend.
The **actual runnable Titan is `Omnisystem/bootstrap-rs`** (a Rust-hosted
lexer/parser/tree-walking interpreter, `cargo build` clean, driven by
`omnicc-seed run|test <file|dir>`, regression suite at `Omnisystem/bootstrap/
tests/*.titan` with `//@ expect-stdout:` assertions). Probing it directly
showed it's **much further along** than the frontend survey implied: struct/
enum/trait/generics/closures/`?`/if-let/while-let/guards/custom iterator
trait impls/associated types (`type Item`, `Self::Item`)/`dyn` trait objects
all **already work** (verified, not assumed). All Titan work in this map
should target `bootstrap-rs` and be verified against real program output, not
against the `frontend/*.titan` files, which remain an inert parallel effort
covered by the separate compiler-completion plan.

The other six Omni-Languages are still narrow domain DSLs at the
frontend-survey level (Sylva=`model layer`, Aether=`actor message state`,
Vera=`component render state`, Helix=`shader vertex fragment compute`,
Axiom=`theorem`, Nexus=`layout grid breakpoint`) — **none has a runnable
substrate yet**, only a parser-level DSL grammar. Standing up an executable
core (interpreter or codegen) for each is a prerequisite before their capability
rows below can be built and verified the way Titan's now are.

**Sylva finding (checked directly, 2026-07-03):** `src/compiler/languages/
sylva_interpreter.py` is **not a real interpreter** — its own `interpret()`
method calls Python's `exec()` directly on the raw source with the comment
*"Simple Python-compatible interpretation... In a real implementation, this
would parse Sylva syntax."* There is no lexer, no parser, no Sylva grammar
recognition anywhere in that file — it's hollow scaffolding, not partial
infrastructure.

**Sylva real substrate built (2026-07-04):** `Omnisystem/bootstrap-sylva-rs` —
a genuine, from-scratch lexer + parser + tree-walking interpreter, **not**
a reskin of Titan's grammar. Deliberately unique per direct user instruction
("Sylva must be fully changed over... all seven Omni-Languages must be unique
to themselves"):
- **Significant whitespace** (real Indent/Dedent tokens), not braces — the
  defining structural difference from Titan.
- `def`/`class` (not `fn`/`struct`+`impl`), dynamic typing throughout (no
  type annotations required — hints are parsed and discarded, never enforced).
- Real exceptions: `try`/`except`/`finally`/`raise`, not `Result`/`?`. All
  runtime faults (division by zero, index errors, missing attributes) are
  **catchable** — this was a real bug caught and fixed mid-build (errors
  initially routed through an uncatchable `Flow::Error`, defeating the whole
  point of having exception handling).
- Classes with single inheritance + MRO, `__init__` constructor semantics,
  closures with real captured-by-reference mutable state (verified: a
  `make_counter()` closure test), f-string interpolation, list/dict
  comprehensions, tuple-unpacking `for` targets (`for k, v in d.items():`),
  lambda, and a real (if minimal) `Tensor` value type for the ML domain
  (zeros/ones/tensor/add/mul/sum/mean — the true minimal subset of
  `SYLVA_STANDARD_LIBRARY.sylva`'s ~40-method spec, not faked further).
- Top-to-bottom script execution (no `main()` requirement) — matches real
  Python semantics, another deliberate structural difference from Titan.
- **Not implemented, flagged not faked:** generators/`yield` (real lazy
  suspension needs a coroutine-capable evaluator — explicitly errors rather
  than pretending), `async`/`await` (parses, evaluates synchronously, no
  event loop), stepped slicing, multi-source `From`-style dispatch equivalent.
- Test suite: `Omnisystem/bootstrap-sylva/tests/*.sylva` with `#@
  expect-stdout:` assertions, run via `cargo run -- test <dir>`. **4/4
  passing**, verified against actual interpreter output (not hand-assumed).

**Verified progress log (bootstrap-rs):**
- Range patterns in `match` (`1..=9`, open-ended `..=69`, char ranges) — built + tested (`14_range_patterns.titan`).
- Loop labels (`'outer: for … break 'outer / continue 'outer`) — built + tested (`15_loop_labels.titan`).
- `?` + `From` auto-conversion across differing `Result<_, E>` error types — built + tested (`16_try_from_conversion.titan`).
- Confirmed already-working (no work needed): custom trait-based iterators, associated types, `dyn` trait objects, basic same-type `?` propagation.
- Suite status: 16/16 passing.

---

## 1. TITAN — systems / core

**Absorbs:** Rust, C, C++, Zig, Go, Ada, D, Carbon, Odin, Nim, V, Swift(sys),
Objective-C, assembly, WASM, Fortran(perf).

| Category | Capabilities (exemplars) | Status |
|---|---|---|
| **Memory & ownership** | manual alloc/free (C); RAII/destructors (C++); ownership + borrow checker + lifetimes (Rust); move semantics; arenas/regions + `defer`/`errdefer` (Zig); optional GC (Go/D); ARC (Swift); `Box`/smart pointers; pinning | ◐ `ref/mut` exist; borrow checker, lifetimes, RAII, arenas ✗ |
| **Type system** | sized primitives (i8…i128, f16…f128); structs; tagged-union enums w/ data (Rust); `union` (C); traits + bounds + associated types + trait objects `dyn` (Rust); generics w/ monomorphization; templates (C++); interfaces (Go); const generics; type inference; newtypes; opaque/existential types; `where` clauses | ◐ struct/enum/trait/generics/**associated types**/**`dyn` trait objects** ✓ (verified); const generics, full inference, newtypes, HKT ✗ |
| **Concurrency** | OS threads; async/await + futures; `select` + channels (Go); actors; atomics + memory orderings; `Send`/`Sync` markers; scoped threads; mutex/rwlock; work-stealing | ◐ async/await/actor keywords; channels, atomics, select, markers ✗ |
| **Metaprogramming** | declarative + procedural macros (Rust); `comptime` (Zig); templates (C++); `const fn`/const-eval; build-time reflection; derive | ✗ (no macro/comptime/const-eval) |
| **Unsafe / low-level** | raw pointers + arithmetic; inline asm; `extern "C"` FFI; SIMD intrinsics; `volatile`; bitfields + packed structs; alignment control; `#[repr]`; syscalls | ◐ `unsafe` keyword only; pointers/asm/FFI/SIMD/repr ✗ |
| **Control flow** | pattern matching + guards + exhaustiveness; `if let`/`while let`; **labeled break/continue**; `goto` (C); **ranges**; iterators + adaptors | ◐ `match/loop/while/for`/guards/if-let/while-let/**labels**✓/**range patterns**✓/**custom iterator trait impls**✓ (all verified); `goto`, full adaptor chain laziness ✗ |
| **Error handling** | `Result`/`Option` + `?` (Rust); `panic`/`recover` (Go); exceptions (C++); `errdefer` (Zig); error sets | ◐ `Result`/`Option`/`?`/**`?`+`From` auto-conversion**✓ (verified, single-`From`-impl case); multi-source `From` dispatch, `panic`/`recover`, error sets ✗ |
| **Modules & build** | modules + visibility; `cfg` conditional compilation; crates/packages; features; workspaces; incremental compilation | ◐ `mod/use/pub`; cfg, features, package system ✗ |

---

## 2. SYLVA — Python / ML / dynamic scripting

**Absorbs:** Python, JavaScript, TypeScript, Ruby, Lua, R, Julia, Perl, PHP,
Groovy, MATLAB/Octave (numeric), plus NumPy/PyTorch/pandas semantics.

| Category | Capabilities (exemplars) | Status |
|---|---|---|
| **Typing discipline** | dynamic typing; duck typing; gradual/optional type hints (TS, Python typing); structural types (TS); type inference; union/literal types (TS) | ✗ (Sylva currently only parses `model`/`layer`) |
| **Objects & dispatch** | everything-is-object; classes + inheritance + mixins; prototype inheritance (JS); metaclasses (Python); `__dunder__`/protocol methods; monkey-patching; dynamic attribute access; method_missing (Ruby) | ✗ |
| **Functions** | first-class fns + closures; higher-order; decorators (Python); default/keyword/variadic args; partial application; lambda; blocks (Ruby) | ✗ |
| **Async & iteration** | async/await + event loop; promises/futures; generators + `yield`; coroutines; async generators; iterators/`__iter__` | ✗ |
| **Expressive syntax** | list/dict/set comprehensions; destructuring; spread/rest; f-strings/template literals; slicing; operator overloading; `with`/context managers | ✗ |
| **ML / numeric** | N-D tensors + broadcasting; autodiff (fwd/rev); vectorized ops (NumPy/Julia); dataframes (R/pandas); GPU dispatch; einsum; layers/models/optimizers; JIT (Julia/Numba) | ◐ `Tensor` stdlib + `model`/`layer` grammar; autodiff, broadcasting, dataframes, GPU ✗ |
| **Reflection & meta** | runtime `eval`; introspection; dynamic import; REPL; hot reload; metaprogramming (Ruby `define_method`) | ✗ |
| **Strings & data** | regex; string formatting; JSON/serialization; Unicode; interpolation | ✗ |

---

## 3. AETHER — databases & cloud / distributed

**Absorbs:** SQL (ANSI/Postgres/T-SQL/PL-SQL), GraphQL, Cypher, SPARQL, PRQL,
Erlang, Elixir, Gleam, plus distributed-systems semantics.

**Real substrate built (2026-07-04):** `Omnisystem/bootstrap-aether-rs` —
genuinely unique from both Titan (braces) and Sylva (indentation): **`do`/
`end` keyword-delimited blocks** (the third distinct block convention),
atoms (`:ok`), the pipe operator (`|>`), `#{}` string interpolation, and
**multi-clause pattern-matched function definitions** (`def fact(0) do ... end`
/ `def fact(n) do ... end`, tried in order — genuine Erlang/Elixir semantics,
structurally different from Titan's single-body-with-internal-`match` and
Sylva's single dynamic body). Also real: cons-cell list patterns (`[h | t]`),
guards (`when`), `case`/`if` as real expressions (not statement-only), and a
cooperative (documented, not preemptive) actor model with `spawn`/`send`/
`receive`. Test suite `Omnisystem/bootstrap-aether/tests/*.aether`, 3/3
passing, verified against real interpreter output.

| Category | Capabilities (exemplars) | Status |
|---|---|---|
| **Relational query** | SELECT/JOIN/aggregation/GROUP BY; CTEs + recursive; window functions; subqueries; set ops; DDL/schema; indexes; prepared statements; views | ✗ |
| **Transactions** | ACID; isolation levels; MVCC; savepoints; 2-phase commit; optimistic/pessimistic locking | ✗ |
| **NoSQL & graph** | document store; key-value; graph traversal (Cypher/Gremlin); time-series; full-text; vector search | ✗ |
| **Actor / process model** | lightweight processes; mailboxes; supervision trees + let-it-crash (Erlang/OTP); hot code reload; **pattern-matched receive**; links/monitors | ◐ **multi-clause dispatch**✓, **cons patterns**✓, **spawn/send/receive**✓ (all verified, cooperative not preemptive); supervision trees, hot code reload, links/monitors ✗ |
| **Distribution** | clustering; consensus (Raft/Paxos); replication; sharding; eventual consistency + CRDTs; distributed transactions; node discovery | ✗ |
| **Messaging & streaming** | pub/sub; message queues; RPC; event sourcing; CQRS; backpressure; dataflow/stream processing | ✗ |
| **Schema & migration** | schema evolution; migrations; constraints; triggers; stored procedures | ✗ |

---

## 4. VERA — UI

**Absorbs:** React/JSX, Vue, Svelte, SolidJS, Angular, SwiftUI, Jetpack Compose,
Flutter/Dart, QML, HTML, Web Components.

**Real substrate built (2026-07-04):** `Omnisystem/bootstrap-vera-rs` —
genuinely unique structural choice: **markup embedded directly in the
language** (`<Tag attr={expr}>...</Tag>`), not a string template and not a
builder-pattern API — the real, defining differentiator, matching what
JSX/Vue/Svelte actually do syntactically. The classic JSX `<`-ambiguity
(tag-open vs. less-than) is avoided structurally, not by lexer heuristics:
the parser only ever calls its markup-node parser from `render { }`, a
tag's own children, or inside `{if}`/`{for}` — never from general expression
context — so `<` is unambiguous by construction. Every binding (props/
state/locals) is stored as a shared mutable cell, which is what makes
reactivity genuinely real rather than asserted: a method mutates the same
cell a render reads, verified by a real before/after tree-diff test (not a
visual claim — there's no browser/GPU here, so "real" means the render
output is a genuine, verifiably-correct data structure). Also real:
`computed` (refreshed each render pass — documented as not fine-grained/
incrementally memoized), component composition (a tag naming another
component mounts and recursively renders it in place), `{if}`/`{for}`
conditional and list rendering, event-handler closures (`fn(){...}`, `||
expr`, `|params| expr`). Test suite `Omnisystem/bootstrap-vera/tests/*.vera`,
2/2 passing.

| Category | Capabilities (exemplars) | Status |
|---|---|---|
| **Component model** | components + props + children/slots; lifecycle; composition; fragments | ◐ **components/props/composition**✓ (verified); slots, lifecycle hooks, named fragments ✗ |
| **Reactivity** | declarative render; virtual DOM + diffing; signals/fine-grained reactivity (Solid/Svelte); computed/derived; effects | ◐ **cell-based state**✓, **computed**✓ (verified, whole-pass not fine-grained); real diffing, effects ✗ |
| **Data binding** | one-way + two-way binding; controlled inputs; refs; context/provide-inject | ◐ one-way (props)✓; two-way binding, refs, context/provide-inject ✗ |
| **Events & hooks** | event handlers; hooks/composables; custom events; delegation; lifecycle hooks | ◐ **event-handler closures**✓ (verified via method calls mutating state); hooks/composables, custom events, lifecycle ✗ |
| **Rendering logic** | conditional render; list render + keys; portals; suspense/async boundaries; error boundaries | ◐ **{if}/{for}**✓ (verified); keys, portals, suspense, error boundaries ✗ |
| **Styling & motion** | scoped styles; CSS-in-JS; themes; transitions/animations; keyframes | ✗ |
| **Accessibility** | ARIA; focus management; semantic roles; keyboard nav | ✗ |

---

## 5. NEXUS — layout

**Absorbs:** CSS (Flexbox/Grid), Cassowary constraint layout, Apple Auto Layout,
Android ConstraintLayout, TeX/typesetting.

**Real substrate built (2026-07-04):** `Omnisystem/bootstrap-nexus-rs` —
genuinely different *in kind*, not just syntax: a **declarative
constraint/layout solver**, not a general-purpose imperative language at
all (no functions, no control flow, no statements — see the other four
languages for that). Box/layout properties are equations resolved lazily
and memoized (spreadsheet-cell style) with real dependency-cycle detection
(verified: a genuine `A.width` ← `B.width` ← `A.width` cycle reports a clear
error instead of infinite-looping). `layout` blocks run a real row/column
flow-layout algorithm — cumulative main-axis positioning + cross-axis
stretch — not just arbitrary equation declarations, which is the actual
structural reason `layout` exists as distinct from plain `box`. Top-level
`constrain` statements are genuinely **checked**, not just declared: a
violated constraint (verified: `A.width <= 100` when `A.width` solves to
500) reports `FAILED` and the whole run exits 1 — solving can really fail.
Test suite `Omnisystem/bootstrap-nexus/tests/*.nexus`, 3/3 passing, covering
the happy path *and both failure modes* (violated constraint, dependency
cycle) — not just success cases.

| Category | Capabilities (exemplars) | Status |
|---|---|---|
| **Flexbox** | direction, wrap, grow/shrink/basis, justify/align, gap, order | ◐ **direction (row/column) + real flow positioning + cross-axis stretch**✓ (verified); wrap, grow/shrink/basis, justify/align, gap, order ✗ |
| **Grid** | template rows/cols, areas, auto-placement, spanning, `minmax`/`fr`, subgrid | ✗ |
| **Constraints** | Cassowary solver; equalities/inequalities; priorities; intrinsic sizing | ◐ **equality/inequality checking + cycle-safe dependency resolution**✓ (verified, both success and failure); real Cassowary (priorities/soft constraints), intrinsic sizing ✗ |
| **Responsive** | breakpoints; media/container queries; fluid units (clamp/vw); aspect ratio | ✗ |
| **Box & position** | box model; margin/padding/border; absolute/relative/sticky/fixed; z-order; overflow | ◐ **x/y/width/height**✓ (verified); margin/padding/border, position modes, z-order, overflow ✗ |
| **Typography flow** | text flow; baseline alignment; columns; line-breaking (TeX) | ✗ |

---

## 6. HELIX — GPU / compute-kernel

**Absorbs:** GLSL, HLSL, WGSL, MSL (Metal), CUDA, OpenCL, SPIR-V, compute
shaders.

**Real substrate built (2026-07-04):** `Omnisystem/bootstrap-helix-rs` —
genuine uniqueness is the **type system and execution model**, not
punctuation: first-class swizzled vector types (`v.xyz`, `v.rgb` — verified:
`dot`/`cross`/`length`/`normalize` all numerically correct against known
values, e.g. `cross(x̂,ŷ)=ẑ`, `length(3,4,0)=5`), a data-parallel kernel
**dispatch** model (`dispatch(Kernel, buffer, n)` runs the kernel body once
per thread id 0..n against a shared mutable buffer — honestly documented as
sequential/single-threaded simulation, since there's no real GPU here, not
a claim of actual parallel execution), and vertex/fragment/compute shader
**stages** (`run_stage(Shader, buffer(...))` runs the stage once per input,
collecting outputs — verified with a real position-transform test, no
rasterization simulated, an honest scope boundary). Two restrictions are
**genuinely enforced**, not just implied by the domain: **no recursion**
(a call-stack check rejects a fn/kernel/shader calling itself — verified)
and **no dynamic loop bounds** (`for i in 0..N` requires `N` to be a literal
integer in the source, checked against the AST node itself, not evaluated —
verified: a `for i in 0..n` with `n` a variable is rejected with a clear
error, while `for i in 0..5` runs correctly). Test suite
`Omnisystem/bootstrap-helix/tests/*.helix`, 5/5 passing — the largest and
most failure-mode-complete suite of the six languages built so far
(deliberately tests both enforced restrictions as real rejections, not just
happy paths). Three real bugs caught and fixed: a `use Value::*` glob import
shadowed the `std::vec::Vec` type name (needed explicit `std::vec::Vec`
qualification), an NLL borrow-lifetime edge case in tail-expression
position, and — the third occurrence of a now-fully-recognized bug class —
`buffer`/`dispatch` reserved as keywords despite never getting dedicated
grammar, blocking their use as ordinary builtin-call names (same root cause
as Aether's `send` and Vera's `render`).

| Category | Capabilities (exemplars) | Status |
|---|---|---|
| **Shader stages** | vertex, fragment, geometry, tessellation (control/eval), compute, mesh/task | ◐ **vertex/fragment/compute stages via run_stage**✓ (verified); geometry/tessellation/mesh ✗ |
| **GPU types** | vec2-4, mat2-4, samplers, textures (1D/2D/3D/cube/array), images, buffers, atomics | ◐ **vec2/vec3/vec4 + swizzle**✓, **buffers**✓ (verified); mat2-4, samplers, textures, atomics ✗ |
| **Built-in functions** | dot/cross/normalize/reflect/refract; mix/clamp/step/smoothstep; texture sampling; derivatives (dFdx); pow/exp/log trig | ◐ **dot/cross/normalize/length/mix/clamp/min/max/abs/floor/fract/pow/sqrt**✓ (verified numerically correct); reflect/refract/step/smoothstep, texture sampling, derivatives ✗ |
| **Compute model** | workgroups + local size; shared/groupshared memory; barriers/sync; atomic ops; SIMD lanes | ◐ **kernel dispatch over a thread-id domain**✓ (verified, sequential simulation); workgroups/shared-memory/barriers/atomics ✗ |
| **GPU memory & binding** | uniforms; push/root constants; descriptor sets/bind groups; storage buffers; memory qualifiers (coherent/volatile) | ◐ **mutable shared buffers (SSBO idiom)**✓ (verified); uniforms, descriptor sets, memory qualifiers ✗ |
| **Kernel dispatch** | CUDA/OpenCL kernels; grid/block dims; host↔device transfer; streams | ◐ **dispatch(kernel, buffer, n)**✓ (verified); grid/block 2D/3D dims, streams ✗ |
| **Codegen targets** | SPIR-V emission; DXIL/DXBC; Metal AIR; PTX | ◐ SPIR-V header words only (per compiler plan); full ISel ✗ |

---

## 7. AXIOM — formal verification

**Absorbs:** Coq, Agda, Idris, Lean, TLA+, Dafny, F*, Isabelle/HOL, Alloy,
MiniZinc, Prolog/Datalog.

**Real substrate built (2026-07-04):** `Omnisystem/bootstrap-axiom-rs` — a
genuine **checker**, not a program interpreter: `theorem`/`invariant`
statements are actually verified, not example-tested. Core technique is
**bounded-exhaustive verification** (Alloy's real, legitimate finite-scope
model-finding approach): `theorem Name forall x in lo..hi, ...` checks every
combination of the quantified variables' explicit finite ranges — verified:
commutativity proven over 100 cases, `x*x >= 0` proven over 10 cases
(including negatives), and a genuinely false claim (`x < 3` for all
`0..5`) correctly **disproven** with the real, reproducible counterexample
`x=3`, not a guess. `axiom` declarations are assumed ground facts,
referenceable by name in theorems via the one genuinely domain-distinctive
operator, `=>` (material implication) — verified working. `invariant ...
over states (...)` checks a proposition across an explicit, enumerated
state space (TLA+ heritage, simplified to a fixed list rather than
reachability search from a transition relation — an honest scope boundary)
— verified: correctly flags a negative-count state as `VIOLATED` while
passing the non-negative ones. The restriction that makes this authentic
rather than silently wrong: **an unbounded free variable in a theorem is a
real, reportable error** (`undefined name 'x'`), not silently assumed
universal — verified via a dedicated rejection test. Test suite
`Omnisystem/bootstrap-axiom/tests/*.axiom`, 2/2 passing. One real bug
caught: negative range/state literals (`-5..5`) never actually lex as
single signed-int tokens (a prior lexer comment falsely promised a
"dedicated signed-int parser path" that was never implemented) — fixed by
having `expect_int_lit` consume an optional leading `-` operator itself.

| Category | Capabilities (exemplars) | Status |
|---|---|---|
| **Theorems & proofs** | theorem/lemma; tactic language (Coq/Lean); proof terms; `Qed`; goals/subgoals | ◐ **theorem + bounded-exhaustive proof/disproof with real counterexamples**✓ (verified); tactic language, proof terms, goals/subgoals ✗ |
| **Dependent & refinement types** | dependent types (Agda/Idris); refinement types (F*/Liquid); indexed types; `Prop`/`Type` universes | ✗ |
| **Contracts** | pre/postconditions; invariants; `requires`/`ensures` (Dafny); `assume`/`assert`; ghost/spec variables | ◐ **invariant-over-explicit-states checking**✓ (verified); requires/ensures contracts, ghost/spec variables ✗ |
| **Model checking** | TLA+ temporal logic (□/◇); state exploration; safety/liveness; fairness | ◐ **explicit-state-space invariant checking**✓ (verified); temporal logic, transition-relation reachability search, liveness/fairness ✗ |
| **Solvers** | SMT backends (Z3); constraint solving (MiniZinc); unification | ◐ **bounded-exhaustive decision procedure**✓ (verified, real for the stated finite domain); real SMT (Z3-class unbounded solving), unification ✗ |
| **Inductive & logic** | inductive datatypes; structural recursion + termination checking; Horn clauses + backtracking (Prolog); Datalog fixpoint | ✗ |

---

## Build sequencing (token-scoped, one capability-category per pass)

Ordered by leverage. Each row is a discrete, compilable milestone: extend the
frontend grammar + AST, add type-system/semantic support, add runtime/stdlib,
and add a fixture test proving the feature parses and lowers.

1. **Titan error handling + pattern matching depth** — `Result`/`Option`/`?`,
   exhaustive match + guards + if-let. Foundational; everything else leans on it.
2. **Titan type system** — associated types, trait objects, const generics,
   full inference. Unlocks generic stdlib.
3. **Titan memory model** — ownership/borrow checker, lifetimes, RAII, arenas.
   Highest design difficulty; do after the type system is solid.
4. **Titan metaprogramming + low-level** — macros/comptime, FFI, inline asm, SIMD.
5. **Sylva general-purpose core** — dynamic typing + gradual hints, objects/
   classes/closures/decorators, async/generators, comprehensions. (Sylva is
   currently the *least* complete vs. its target — biggest single gap.)
6. **Sylva numeric/ML depth** — autodiff, broadcasting, dataframes, GPU dispatch.
7. **Aether query + actor core** — SQL/DDL/transactions; OTP-style supervision.
8. **Aether distribution** — consensus, replication, CRDTs, pub/sub.
9. **Vera reactivity + component depth** — signals, effects, binding, hooks.
10. **Helix GPU types + built-ins + compute model** — then codegen targets.
11. **Nexus full flex/grid/constraint model.**
12. **Axiom dependent types + tactics + solver integration.**

## Verification gate per capability

A capability is "done" only when: (a) the frontend parses it (fixture in
`validation/`), (b) it type-checks / lowers to IR without falling back to raw
passthrough, (c) a runtime/stdlib test exercises it, and (d) it's documented in
that language's standard-library reference. Grammar-only support (a keyword the
parser accepts but does nothing with) does **not** count — that is the ◐ state
this map is designed to eliminate.

## Relationship to other plans

- `OmniHarness/docs/SUBSYSTEM_BUILD_PLAN.md` — Rust subsystem crates; separate.
- `vscode-omnisystem/src/omnicc/CAPABILITY_COVERAGE.md` — OmniCC cross-converter
  IR coverage; separate (converts *between* languages, does not add native
  capability to the Omni-Languages themselves — which is what *this* map is for).
- The existing compiler-completion plan (`polymorphic-purring-gizmo`) builds the
  parsers/backends; this map defines *what language features those parsers must
  eventually accept*.
