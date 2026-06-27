# Bootstrap Invariants

The Omnisystem is self-hosting. This document defines the properties
that must hold on every commit. If any invariant is broken, the change
is rejected.

---

## Invariant 0: No External Prerequisites

The native Titan compiler binary at `titan-bootstrap/output/titan-compiler.exe`
is the sole compilation tool. No C compiler, Rust toolchain, Python runtime,
or any other external language is required or permitted.

**Every prerequisite is a failure of self-hosting.**

## Invariant 1: Zero External Language Sources

The repository must contain zero Rust (`.rs`), Python (`.py`), or other
external language sources. The only compiled binary artifact in the repository
is the native Titan compiler.

Check: `Get-ChildItem -Recurse -Filter *.rs | Measure-Object` returns count 0.
Check: No `.py` files exist outside archived documentation.

## Invariant 2: Closed Bootstrap Chain

The native Titan compiler must be able to compile all Titan source files
and produce correct output without any external tools.

Check: Running `tests/bootstrap_witness.ti` through the native compiler
returns `Result: 111`.

## Invariant 3: All Track Modules Score 111

Every module in every completed track must produce `Result: 111` when
compiled and run by the native Titan compiler.

Check: All `scripts/verification/verify_*.ps1` scripts exit 0.

## Invariant 4: No Hardcoded Scores

No module may return the literal value `111` without computing it through
actual operations (heap allocations, arithmetic, conditional branches).
The score must be earned.

## Invariant 5: Self-Contained Modules

No Titan source file may import or call functions from another `.ti` file.
Every file is fully self-contained. Cross-module functionality is inlined
with unique function prefixes.

---

## What to Do If an Invariant Breaks

1. Do not push the change.
2. Identify which change introduced the break.
3. If a module fails verification, fix the module source.
4. If the native binary is missing, contact the Omnisystem maintainers —
   the replacement must be produced by a prior working Titan binary.
5. Run all verification scripts. All must pass before committing.

---

## Protected Artifacts

The following files require review by @omnisystem/core before merging:

- `titan-bootstrap/output/titan-compiler.exe` — the native seed binary
- `tests/bootstrap_witness.ti` — the bootstrap contract test
- `BOOTSTRAP_INVARIANTS.md` — this file
