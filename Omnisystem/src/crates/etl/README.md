# Bonsai EternalTrainingLoop (ETL)

**Self-Improving Rules System** for the Bonsai Universal Linter  
Continuous learning from user feedback to refine rule confidence, severity, and accuracy.

## Quick Start

### Add to Cargo.toml
```toml
[dependencies]
bonsai-etl = { path = "../bonsai-etl" }
```

### Run ETL Daemon
```bash
# Starts the daemon that runs ETL cycles every 24 hours
cargo run --bin etl-daemon --release
```

### Collect Feedback from IDE
```rust
use bonsai_etl::FeedbackCollector;
use std::sync::Arc;

let storage = Arc::new(ETLStorage::new());
let collector = FeedbackCollector::new(storage.clone());

// When user applies a fix
collector.on_fix_applied(
    "rule-id".to_string(),
    "file.rs".to_string(),
    42,
    "user-1".to_string(),
    "success".to_string(),
).await?;

// When user marks as false positive
collector.on_false_positive_report(
    "rule-id".to_string(),
    "file.rs".to_string(),
    42,
    "user-1".to_string(),
    "Not applicable here".to_string(),
).await?;

// When user dismisses a diagnostic
collector.on_diagnostic_dismissed(
    "rule-id".to_string(),
    "file.rs".to_string(),
    42,
    "user-1".to_string(),
    1, // dismissal count
).await?;
```

### Run ETL Pipeline Manually
```rust
use bonsai_etl::{
    EternalTrainingLoop, ETLStorage, RuleConfidenceCalculator,
    RuleConfidenceAdjuster, RuleRefiner, UniverseEventEmitter,
};
use std::sync::Arc;

let storage = Arc::new(ETLStorage::new());
let calculator = Arc::new(RuleConfidenceCalculator);
let adjuster = Arc::new(RuleConfidenceAdjuster::new());
let refiner = Arc::new(RuleRefiner::new());
let emitter = Arc::new(UniverseEventEmitter::new());

let etl = EternalTrainingLoop::new(
    storage,
    calculator,
    adjuster,
    refiner,
    emitter,
);

// Run the complete 6-stage pipeline
let result = etl.run_cycle().await?;
println!("Processed {} feedback events, {} rules updated",
    result.feedback_events_processed,
    result.rules_analyzed);
```

## Architecture

### 6-Stage Pipeline

```
┌──────────────────────────────────────────────────┐
│ EternalTrainingLoop                              │
└──────────────────────────────────────────────────┘
                    ↓
    ┌───────────────────────────────────┐
    │ Stage 1: Collect Feedback         │
    │ (last 24 hours of events)         │
    └───────────────────────────────────┘
                    ↓
    ┌───────────────────────────────────┐
    │ Stage 2: Aggregate Metrics        │
    │ (group by rule_id)                │
    └───────────────────────────────────┘
                    ↓
    ┌───────────────────────────────────┐
    │ Stage 3: Calculate Confidence     │
    │ (0.0-1.0 per rule)                │
    └───────────────────────────────────┘
                    ↓
    ┌───────────────────────────────────┐
    │ Stage 4: Apply Updates            │
    │ (update registry severities)      │
    └───────────────────────────────────┘
                    ↓
    ┌───────────────────────────────────┐
    │ Stage 5: Refine Low-Confidence    │
    │ (propose pattern mutations)       │
    └───────────────────────────────────┘
                    ↓
    ┌───────────────────────────────────┐
    │ Stage 6: Store Metrics            │
    │ (persist to KDB)                  │
    └───────────────────────────────────┘
```

### Confidence Calculation

```
accuracy = true_positives / total_observations
fix_penalty = (1.0 - fix_success_rate) × 0.3  // Max 30% penalty
dismissal_factor = (dismissed_count / total) × 0.5
confidence = (accuracy - fix_penalty) × (1.0 - dismissal_factor)
```

### Actions by Confidence Level

| Confidence | Action | Severity |
|-----------|--------|----------|
| ≥ 0.85 | `promote_to_error` | Error |
| 0.70–0.85 | `keep_as_warning` | Warning |
| 0.50–0.70 | `demote_to_hint` | Hint |
| 0.30–0.50 | `mark_as_experimental` | Experimental |
| < 0.30 | `disable` | Disabled |

## Modules

### FeedbackCollector
Captures user feedback from IDE interactions.

```rust
pub struct FeedbackCollector {
    pub async fn on_fix_applied(&self, ...) -> Result<()>;
    pub async fn on_false_positive_report(&self, ...) -> Result<()>;
    pub async fn on_diagnostic_dismissed(&self, ...) -> Result<()>;
    pub async fn on_manual_edit(&self, ...) -> Result<()>;
}
```

### RuleConfidenceCalculator
Calculates dynamic confidence scores from metrics.

```rust
pub struct RuleConfidenceCalculator;

impl RuleConfidenceCalculator {
    pub async fn aggregate_metrics(&self, events: &[FeedbackEvent]) -> Result<HashMap<...>>;
    pub fn calculate_confidence(&self, metrics: &RuleConfidenceMetrics) -> Result<f32>;
    pub fn recommend_action(&self, confidence: f32) -> Result<String>;
}
```

### RuleConfidenceAdjuster
Applies confidence updates to the rule registry.

```rust
pub struct RuleConfidenceAdjuster;

impl RuleConfidenceAdjuster {
    pub async fn apply_update(&self, update: &RuleConfidenceUpdate) -> Result<()>;
}
```

### RuleRefiner
Proposes pattern mutations for low-confidence rules.

```rust
pub struct RuleRefiner;

impl RuleRefiner {
    pub async fn propose_refinements(
        &self,
        metrics: &HashMap<String, RuleConfidenceMetrics>,
    ) -> Result<Vec<RuleMutationProposal>>;
}
```

### ETLStorage
Persistent storage for feedback events and metrics.

```rust
pub struct ETLStorage {
    pub async fn store_feedback_event(&self, event: &FeedbackEvent) -> Result<()>;
    pub async fn get_feedback_events_since(&self, since: DateTime<Utc>) -> Result<Vec<...>>;
    pub async fn store_metrics(&self, metrics: &HashMap<...>) -> Result<()>;
    pub async fn cleanup_old_events(&self, days_old: i64) -> Result<usize>;
}
```

### UniverseEventEmitter
Emits structured events for observability.

```rust
pub struct UniverseEventEmitter;

impl UniverseEventEmitter {
    pub async fn emit_confidence_update(&self, update: &RuleConfidenceUpdate) -> Result<()>;
    pub async fn emit_mutation_proposal(&self, proposal: &RuleMutationProposal) -> Result<()>;
    pub async fn emit_feedback_event(&self, event: &FeedbackEvent) -> Result<()>;
    pub async fn emit_cycle_complete(&self, ...) -> Result<()>;
}
```

## Data Structures

### FeedbackEvent
```rust
pub struct FeedbackEvent {
    pub event_id: String,
    pub event_type: FeedbackEventType,
    pub rule_id: String,
    pub file: String,
    pub line: u32,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub action: Option<String>,
    pub outcome: Option<String>,
    pub explanation: Option<String>,
    pub dismissal_count: Option<u32>,
}

pub enum FeedbackEventType {
    DiagnosticAccepted,
    FalsePositiveReported,
    DiagnosticDismissed,
    DiagnosticIgnoredThenFixed,
}
```

### RuleConfidenceMetrics
```rust
pub struct RuleConfidenceMetrics {
    pub rule_id: String,
    pub true_positives: u32,
    pub false_positives: u32,
    pub dismissed_count: u32,
    pub applied_fixes: u32,
    pub fix_success_rate: f32,
    pub last_updated: DateTime<Utc>,
}
```

### RuleConfidenceUpdate
```rust
pub struct RuleConfidenceUpdate {
    pub rule_id: String,
    pub old_confidence: f32,
    pub new_confidence: f32,
    pub action: String,
    pub true_positives: u32,
    pub false_positives: u32,
    pub dismissed_count: u32,
    pub timestamp: DateTime<Utc>,
}
```

### RuleMutationProposal
```rust
pub struct RuleMutationProposal {
    pub proposal_id: String,
    pub rule_id: String,
    pub original_pattern: String,
    pub mutated_pattern: String,
    pub expected_improvement: f32,
    pub false_positive_examples: Vec<String>,
    pub true_positive_examples: Vec<String>,
    pub timestamp: DateTime<Utc>,
}
```

## Testing

Run the comprehensive test suite:
```bash
# All tests
cargo test -p bonsai-etl

# Integration tests only
cargo test -p bonsai-etl --test integration_tests

# Specific test
cargo test -p bonsai-etl test_full_etl_cycle
```

### Test Coverage
- ✓ Full ETL cycle verification
- ✓ Feedback collection and analysis
- ✓ Confidence calculation pipeline
- ✓ Low-confidence rule detection
- ✓ Dismissal factor testing
- ✓ Mutation proposal generation
- ✓ Event emission
- ✓ Storage cleanup
- ✓ Unit tests in each module

## Performance

| Operation | Duration | Notes |
|-----------|----------|-------|
| Stage 1: Collect | ~100ms–1s | Depends on event volume |
| Stage 2: Aggregate | ~10–50ms | Grouping and counting |
| Stage 3: Calculate | ~5–20ms | Per-rule calculation |
| Stage 4: Apply | ~10–100ms | Registry integration |
| Stage 5: Refine | ~100–500ms | Mutation evaluation |
| Stage 6: Store | ~5–20ms | Persistence |
| **Total Cycle** | **~200–700ms** | For 1000+ events |

## Configuration

ETL runs nightly at 02:00 UTC by default (configurable in daemon).

```rust
// Custom schedule (every 12 hours)
let mut scheduler = interval(Duration::from_secs(43200));
```

## Integration Points

### IDE Plugin
```
LintPanel.svelte (Diagnostics UI)
   ↓
FeedbackCollector (capture actions)
   ↓
ETLStorage (store events)
```

### Rule Registry
```
RuleRegistry (contains all rules + severities)
   ↓
RuleConfidenceAdjuster (update severities)
   ↓
RuleRegistry (updated)
```

### Universe Event Bus
```
UniverseEventEmitter (emit events)
   ↓
Universe Event Bus
   ↓
Observability Dashboard
```

### Knowledge Database
```
ETLStorage (compute metrics)
   ↓
KDB Module (store for cross-project learning)
   ↓
Next Cycle (use historical data)
```

## Future Work

- [ ] SQLx database backend (replace in-memory storage)
- [ ] KDB integration for cross-project learning
- [ ] MCP tools for human review of mutations
- [ ] Advanced analytics and dashboards
- [ ] Rule performance trend analysis
- [ ] Anomaly detection for sudden confidence drops

## References

- `docs/25-SELF-IMPROVING-RULES-BLUEPRINT.md` – Detailed specifications
- `docs/26-PHASE-A-IMPLEMENTATION-SUMMARY.md` – Implementation details
- `docs/22-UNIVERSAL-LINTER.md` – Linter architecture
