# Trust Score & Verification System

## The Verification Continuum

The Omnisystem provides a continuous spectrum from rapid development to mathematical certainty:

| Level | Technique | Time Cost | Trust Gain | Use Case |
|-------|-----------|-----------|------------|----------|
| 0 | No verification | 0 | Baseline (74) | Prototyping |
| 1 | Unit tests pass | Minutes | Low | Development |
| 2 | Property tests (1000 cases) | Minutes | Medium | Staging |
| 3 | Fuzzing (1 hour, coverage-guided) | Hours | Medium-High | Pre-release |
| 4 | Symbolic execution (bounded) | Hours-days | High | Critical paths |
| 5 | Full Axiom proof | Days-weeks | Mathematical certainty | Production safety-critical |

## Trust Score Computation

The trust score (0-100) aggregates evidence across multiple dimensions:

```
trust_score = baseline + verification_level + capability_safety - violations
            = 74 + effect_level + proof_coverage - unsafe_blocks
```

### Baseline (74/100)
- Code is written in Omni languages (Titan, Aether, Sylva, Axiom)
- Compiles successfully
- No unsafe blocks in critical paths

### Verification Levels
- **Unit tests pass:** +5 points per 10% coverage
- **Property tests pass:** +8 points (1000+ cases required)
- **Fuzzing success:** +12 points (no crashes in 1+ hours)
- **Symbolic verification:** +15 points (bounded paths exhausted)
- **Full Axiom proof:** +20 points (machine-checked to axioms)

### Capability Safety
- **No unsafe blocks:** +0 (neutral)
- **All effects declared:** +3
- **All capabilities granted:** +2
- **Regions checked:** +1

### Violations
- **Unsafe block:** -10 per block
- **Capability escalation attempt:** -15 (caught at compile time)
- **Undefined behavior:** -25
- **Test failure:** -5 per failing test

### Examples
- New Titan module (baseline + units): 74 + 5 = **79/100**
- Titan + properties + fuzzing: 74 + 5 + 8 + 12 = **99/100**
- Axiom-proven code: 74 + 26 = **100/100**

## Error Message Standards

Every diagnostic cites a UniIR rule for clarity and searchability:

```
[E042] Type mismatch in application:
  Expected: (x : Int) -> Vec Int
  Found:    Int
  Rule: UniIR_T_App (UniIR v0.2, §3.4)
  Location: examples/main.ti:12:5-15
  Hint: The function expects a single Int argument.
  Suggestion: Change argument to `Vec::new()` or use int_to_vec(x)
```

## Deployment Gates

Different trust levels are required for different deployment contexts:

### Development Environment
- **Minimum trust:** 0/100
- **Restriction:** None (fail fast encouraged)
- **Rollback:** Automatic

### Staging Environment
- **Minimum trust:** 74/100 (baseline)
- **Restriction:** No unsafe blocks
- **Rollback:** Manual approval

### Production (Standard)
- **Minimum trust:** 95/100
- **Restriction:** ≥90% code path coverage
- **Rollback:** Automatic on panic

### Production (Safety-Critical)
- **Minimum trust:** 100/100
- **Restriction:** All critical paths have Axiom proofs
- **Rollback:** Manual approval only
- **Audit:** Full formal verification report required

## Proof Tokens

Every compile produces a proof token that serves as a certificate:

```json
{
  "module": "examples/calculator.ti",
  "hash": "blake3:abc123...",
  "trust_score": 99,
  "verification": {
    "level": 4,
    "unit_tests": 42,
    "property_tests": 1000,
    "fuzz_hours": 2.5,
    "coverage": 0.95
  },
  "proofs": [
    "UniIR_T_App:examples/main.ti:12",
    "Borrow_Check:examples/actor.ae:34"
  ],
  "signature": "ed25519:xyz789...",
  "timestamp": "2026-05-17T14:23:45Z"
}
```

Tokens can be:
- Embedded in binaries for runtime verification
- Signed and published to the package registry
- Used for automated deployment decisions

## Observability & Telemetry

The Omnisystem emits structured telemetry for every verification step:

```
TELEMETRY: CompileStart
  module: examples/main.ti
  timestamp: 2026-05-17T14:23:45Z

TELEMETRY: TypeCheck
  module: examples/main.ti
  status: OK
  time_ms: 234

TELEMETRY: BorrowCheck
  module: examples/main.ti
  violations: 0
  time_ms: 145

TELEMETRY: Codegen
  module: examples/main.ti
  instructions: 2341
  time_ms: 89

TELEMETRY: CompileEnd
  module: examples/main.ti
  status: OK
  total_time_ms: 468
  trust_score: 99
```

These events are content-addressed and indexed for replay and analysis.

## Continuous Verification

The `build observe` command provides live telemetry streams:

```bash
$ build observe --watch ./myproject

[main.ti] TypeCheck: OK (234ms)
[main.ti] BorrowCheck: OK (145ms), 0 violations
[calculator.ae] Compile: OK (89ms), trust: 95/100
[tests.sy] Run: 42/42 passed (156ms)

Project trust score: 94/100
Status: STAGING-READY
```

## Formal Verification with Axiom

For the highest level of assurance, use Axiom:

```axiom
theorem safe_add_correctness : ∀ (a b : Int),
    result_of(safe_add a b) = a + b ∨ error(safe_add a b)

proof
    intro a b;
    unfold safe_add;
    cases (a > MAX_INT - b);
    case true:  -- overflow
        simp [error_case];
    case false: -- no overflow
        simp [arithmetic];
        exact add_correctness;
end
```

When an Axiom proof is attached to a Titan module, the trust score becomes **100/100** and all downstream users inherit that certainty.

## Migration Path: From Testing to Proof

1. **Write code:** Any Omni language, no verification required
2. **Add unit tests:** Get +5 trust points, identify bugs early
3. **Add property tests:** Get +8 more, catch edge cases
4. **Fuzz the code:** Get +12 more, find rare crashes
5. **Prove critical paths:** Get +15 more with symbolic verification
6. **Prove everything:** Get +20 more with Axiom, reach 100/100

Each step is optional but recommended for production code. Small utility functions might stop at step 2; security-critical code should reach step 6.
