# Anti-Hallucination Framework (AHF) Implementation Plan

**Date**: 2026-06-07  
**Status**: Ready for Parallel Execution  
**Scope**: 6 phases, 8 parallel agents, ~8,000+ LOC expected  
**Integration**: Full Omnisystem (HDE, UMS, CAS, Axiom, Audit Log, SLM)  

---

## Overview

The Anti-Hallucination Framework prevents AI models (internal or external) from generating false, misleading, or biased outputs. It is **not** a filter — it is a **provably complete verification pipeline** that grounds every output in verified knowledge, validates formal correctness, and applies safety constraints.

### Key Principle
**When verification fails, refuse to answer rather than hallucinate.**

---

## Architecture

```
AI Model Output
    ↓
[Anti-Hallucination Gateway (AHG) - Aether Actor]
    ├─→ Parse & Validate Schema
    ├─→ Extract Factual Claims
    ↓
[Parallel Verification]
    ├─→ Knowledge Grounding Service (KGS)
    │   └─→ CAS lookup, UMS modules, system state
    └─→ Formal Verifier (Axiom)
        └─→ Schema validation, consistency check
    ↓
[Arbiter - HDE Subsystem]
    ├─→ Combine signals (grounding, verification, confidence, bias)
    ├─→ Apply safety envelopes
    ↓
    ├─→ ACCEPT → deliver to user
    ├─→ REJECT → safe fallback response
    └─→ ESCALATE → human review queue
```

All components are Aether actors, run in Sanctum vaults, and communicate via capability-mediated IPC.

---

## Crate Structure (New)

```
crates/
├── ahf-core/                 # Core types, error handling, interfaces
├── ahf-knowledge-grounding/  # KGS implementation
├── ahf-formal-verifier/      # Axiom integration, schema validation
├── ahf-arbiter/              # Decision logic, HDE integration
├── ahf-bias-detector/        # Pattern matching + optional ML
├── ahf-gateway/              # Main AHG actor, orchestration
├── ahf-integration-tests/    # 100+ comprehensive tests
└── ahf-config/               # Governance policies, thresholds
```

---

## Phase 1: Core Infrastructure & Knowledge Grounding

**Target**: 1,500+ LOC, 40+ tests  
**Deliverables**:
- `ahf-core`: Type definitions, traits, error handling
- `ahf-knowledge-grounding`: Triple extraction, CAS lookup, scoring

**Key Components**:
1. `Fact` struct: subject, predicate, object, confidence
2. `FactExtractor`: Deterministic triple parser
3. `KnowledgeBase` trait: CAS, UMS, and dynamic state backends
4. `GroundingService`: Queries knowledge base, returns evidence or NotFound/Contradicted
5. `GroundingResult`: score, evidence hashes, contradictions

**Integration Points**:
- CAS for immutable knowledge snapshots
- UMS for knowledge modules (`knowledge-facts-v1`)
- Service Lifecycle Manager for dynamic state queries

---

## Phase 2: Formal Verifier & Schema Validation

**Target**: 1,200+ LOC, 35+ tests  
**Deliverables**:
- `ahf-formal-verifier`: Axiom integration, schema validation
- Schema validation pipeline (JSON Schema, Protocol Buffers)
- Session consistency checker

**Key Components**:
1. `OutputParser`: Converts raw output → Omni term heap
2. `SchemaValidator`: JSON Schema / type checking
3. `SessionHistory`: Tracks accepted facts, checks contradictions
4. `AxiomVerifier`: Integrates Axiom proof checking (optional proofs)
5. `VerificationResult`: Valid/Invalid with proof details

**Integration Points**:
- Omni-ABI term heap for structured representation
- Axiom for proof verification
- Session management for history tracking

---

## Phase 3: Arbiter & HDE Integration

**Target**: 800+ LOC, 30+ tests  
**Deliverables**:
- `ahf-arbiter`: Decision engine, safety envelopes, threshold management

**Key Components**:
1. `ArbiterSignals`: Struct combining all input signals
2. `ArbiterDecision`: Accept/Reject/Escalate
3. `SafetyEnvelope`: Axiom-proven bounds on outputs
4. `PolicyRegistry`: Council-governed thresholds
5. `HdeArbiterActor`: Aether actor integration

**Decision Rules** (Formally Verified):
```rust
if verification_result == Invalid → REJECT
if grounding_score < THRESHOLD → REJECT
if model_confidence < MIN_CONFIDENCE → REJECT
if bias_score > BIAS_THRESHOLD → REJECT
if criticality == High && grounding_score < 1.0 → ESCALATE
else → ACCEPT
```

**Integration Points**:
- HDE for safety envelopes and shadow mode
- Governance council for policy updates
- Audit log for decision recording

---

## Phase 4: Bias Detection & Uncertainty Quantification

**Target**: 900+ LOC, 40+ tests  
**Deliverables**:
- `ahf-bias-detector`: Pattern matching + optional classifier
- Uncertainty quantification (model confidence, self-consistency sampling)

**Key Components**:
1. `BiasPattern`: Regex + metadata for known biased patterns
2. `BiasDetector`: Deterministic pattern matching
3. `BiasClassifier`: Optional ML-based detection (shadow mode)
4. `ConfidenceExtractor`: Gets confidence from model or samples
5. `CalibrationValidator`: Validates confidence scores

**Bias Patterns** (Examples):
- "all [group] are [stereotype]"
- Demographic disparities in outcomes
- Unfair treatment indicators

**Integration Points**:
- UMS for bias pattern modules
- Model confidence extraction (built-in or external)
- Validation Mesh for continuous testing

---

## Phase 5: Anti-Hallucination Gateway & Orchestration

**Target**: 1,500+ LOC, 50+ tests  
**Deliverables**:
- `ahf-gateway`: Main AHG Aether actor, orchestration logic
- `ahf-config`: Governance policies, configuration management
- Comprehensive integration tests

**Key Components**:
1. `AhgActor`: Main Aether actor (receives model output, orchestrates verification)
2. `AhfPipeline`: Chains KGS, Verifier, Arbiter
3. `PolicyConfig`: Thresholds, knowledge sources, bias patterns
4. `HallucinationTestSuite`: 1,000+ test cases for continuous validation
5. `AhfMetrics`: Tracking hallucination rate, false rejections, latency

**Gateway Pipeline**:
1. Parse output & extract schema type
2. Validate against schema
3. Extract factual claims
4. Spawn parallel KGS & Verifier tasks
5. Wait for both → Arbiter decision
6. Apply safety envelopes
7. Log to audit trail
8. Return result (output / fallback / escalation)

**Integration Points**:
- Omni-Bot for exposing AHF via REST API
- Service Lifecycle Manager for hot-reloading knowledge modules
- Audit log for immutable decision trail

---

## Phase 6: Formal Verification & Production Hardening

**Target**: 800+ LOC, 30+ tests  
**Deliverables**:
- Axiom proofs of arbiter correctness
- Continuous validation with UVM
- Performance optimization
- Hot-reload mechanism for knowledge modules

**Proofs**:
1. **Arbiter Soundness**: If all checks pass, output cannot be hallucination (given correct knowledge base)
2. **Bias Detector Completeness**: Proven false-negative rate bounds
3. **Safety Envelope Monotonicity**: Clamping never breaks invariants

**Continuous Validation**:
- UVM runs hallucination test suite 24/7
- Regression detection on any model
- Automatic capability revocation for violations

---

## Parallel Execution Strategy

**8 Agents** (all running simultaneously):

1. **AHF Core Infrastructure** (Phase 1)
   - ahf-core, ahf-knowledge-grounding types
   
2. **Knowledge Grounding Service** (Phase 1)
   - Fact extraction, CAS/UMS lookup, scoring
   
3. **Formal Verifier** (Phase 2)
   - Schema validation, Axiom integration, consistency checking
   
4. **Arbiter & HDE Integration** (Phase 3)
   - Decision logic, safety envelopes, policy management
   
5. **Bias Detector & Uncertainty** (Phase 4)
   - Pattern matching, confidence extraction, calibration
   
6. **Gateway Orchestration** (Phase 5)
   - AHG actor, pipeline coordination, config management
   
7. **Integration Tests & Validation** (Phases 5-6)
   - 100+ integration tests, hallucination test suite, UVM harness
   
8. **Formal Verification & Hardening** (Phase 6)
   - Axiom proofs, performance tuning, hot-reload

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Hallucination Rate (test suite) | < 0.1% |
| False Rejection Rate | < 0.5% |
| Bias Blocking Rate | > 99% |
| AHF Latency (end-to-end) | < 50 ms |
| Knowledge Base Coverage | > 90% |
| Code Coverage | > 95% |
| Axiom Proof Coverage | > 80% of critical paths |

---

## Integration Checklist

- [ ] CAS knowledge base snapshots loaded
- [ ] UMS knowledge modules integrated
- [ ] Service Lifecycle Manager integration
- [ ] Axiom proof system ready
- [ ] Audit log recording enabled
- [ ] Omni-Bot REST API wired
- [ ] Validation Mesh continuous testing
- [ ] Governance council policy framework
- [ ] Human review queue service
- [ ] TransferDaemon knowledge base distribution

---

## Timeline (No Hard Deadlines)

Phase 1-2: Foundation (KGS, Verifier)  
Phase 3-4: Decision & Safety (Arbiter, Bias)  
Phase 5: Integration & Testing  
Phase 6: Formal Verification & Production  

All phases run in parallel; dependencies managed via Aether message passing.

---

**Status**: Ready for parallel execution  
**Next**: Spawn 8 agents for simultaneous development
