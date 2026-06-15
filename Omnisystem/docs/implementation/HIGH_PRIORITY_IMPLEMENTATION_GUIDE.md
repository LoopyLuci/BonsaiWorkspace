# High-Priority Implementation Guide (9 Files)

## Summary
- **Completed:** 2/9 (22%)
- **Remaining:** 7/9 (78%)
- **Status:** All templates and patterns documented below

---

## COMPLETED ✅

### 1. survival_feedback.rs - IMPLEMENTED
**File:** `crates/lint/src/integration/survival_feedback.rs`
**Status:** ✅ 100% COMPLETE
- Crash report processing with stack trace parsing
- Lint warning correlation with in-memory database
- Metric aggregation and severity escalation
- High-correlation rule identification

**Key Functions Implemented:**
- `on_crash()` - Process crash and find correlations
- `record_correlation()` - Store in database
- `get_correlation_metrics()` - Query metrics
- `get_high_correlation_rules()` - Find critical rules
- `register_lint_warning()` - Track warnings
- `get_rule_correlations()` - Retrieve all correlations for rule

---

### 2. team_profiles.rs - IMPLEMENTED
**File:** `crates/collaboration/src/team_profiles.rs`
**Status:** ✅ 100% COMPLETE
- Team rule profile management with CRUD operations
- Profile persistence to JSON files
- Cache management with DashMap
- Profile inheritance and configuration

**Key Functions Implemented:**
- `create_profile()` - Create and persist profile
- `get_profile()` - Retrieve with cache-first strategy
- `update_profile()` - Update and persist
- `delete_profile()` - Remove from cache and disk
- `persist_profile()` - Save to JSON file
- `load_profile()` - Load from JSON file
- `delete_from_db()` - Clean up files

---

## REMAINING (7 Files)

### 3. bug_hunt_orchestrator.rs
**File:** `crates/lint/src/integration/bug_hunt_orchestrator.rs`
**Priority:** HIGH
**Stub Count:** 2 TODOs

**Implementation Template:**
```rust
pub struct BugHuntOrchestrator {
    task_queue: Arc<RwLock<Vec<BugHuntTask>>>,
    service_client: Arc<BugHuntClient>,
}

impl BugHuntOrchestrator {
    pub async fn submit_findings(&self, findings: Vec<Finding>) -> Result<Vec<TaskId>> {
        // 1. Convert findings to bug hunt tasks with priority
        // 2. Calculate priority: severity * confidence * correlation_strength
        // 3. Submit to bug hunt service via HTTP API
        // 4. Store task IDs for tracking
        // 5. Return submitted task IDs
    }

    pub async fn get_task_status(&self, task_id: &TaskId) -> Result<TaskStatus> {
        // Query bug hunt service for task status
    }

    pub async fn collect_results(&self) -> Result<Vec<BugHuntResult>> {
        // Poll bug hunt service for completed tasks
        // Store results in database
        // Return results
    }
}
```

**Dependencies to Add:**
```toml
[dependencies]
uuid = { version = "1.0", features = ["v4"] }
```

---

### 4. incremental.rs (Engine Module)
**File:** `crates/lint/src/engine/incremental.rs`
**Priority:** HIGH
**Stub Count:** 4 todo!() for Omni-Language grammars

**Implementation Template:**
```rust
pub struct IncrementalEngine {
    grammars: HashMap<String, TreeSitterGrammar>,
    symbol_table: Arc<RwLock<SymbolTable>>,
}

impl IncrementalEngine {
    pub async fn load_grammars() -> Result<()> {
        // 1. Load Titan language grammar (tree-sitter-titan)
        // 2. Load Aether language grammar (tree-sitter-aether)
        // 3. Load Sylva language grammar (tree-sitter-sylva)
        // 4. Load Axiom language grammar (tree-sitter-axiom)
        // 5. Cache grammars for reuse
    }

    pub async fn parse_incremental(&mut self, source: &str) -> Result<ParseTree> {
        // 1. Detect language from source or metadata
        // 2. Get grammar for language
        // 3. Parse with tree-sitter
        // 4. Update symbol table
        // 5. Return parse tree
    }
}
```

**Dependencies to Add:**
```toml
[dependencies]
tree-sitter = "0.20"
tree-sitter-titan = { git = "https://github.com/omnisystem/tree-sitter-titan" }
tree-sitter-aether = { git = "https://github.com/omnisystem/tree-sitter-aether" }
tree-sitter-sylva = { git = "https://github.com/omnisystem/tree-sitter-sylva" }
tree-sitter-axiom = { git = "https://github.com/omnisystem/tree-sitter-axiom" }
```

---

### 5. voting.rs (Collaboration Module)
**File:** `crates/collaboration/src/voting.rs`
**Priority:** HIGH
**Stub Count:** 3 TODOs

**Implementation Template:**
```rust
pub struct VotingSystem {
    votes: Arc<RwLock<Vec<Vote>>>,
    proposals: Arc<RwLock<HashMap<String, Proposal>>>,
}

#[derive(Debug, Clone)]
pub struct Vote {
    pub vote_id: String,
    pub proposal_id: String,
    pub voter_id: String,
    pub vote_type: VoteType, // Yes, No, Abstain
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct Proposal {
    pub proposal_id: String,
    pub rule_id: String,
    pub description: String,
    pub creator_id: String,
    pub status: ProposalStatus,
}

impl VotingSystem {
    pub async fn submit_vote(&self, vote: Vote) -> Result<()> {
        // 1. Load proposal from database
        // 2. Validate vote is allowed
        // 3. Check voter permissions
        // 4. Store vote in database
        // 5. Recalculate proposal tally
        // 6. Check if proposal reached consensus
        // 7. Execute if consensus reached
    }

    pub async fn get_votes_for_proposal(&self, proposal_id: &str) -> Result<Vec<Vote>> {
        // Query database for all votes on proposal
    }

    pub async fn calculate_tally(&self, proposal_id: &str) -> Result<VoteTally> {
        // Count votes by type
        // Calculate percentages
        // Determine if consensus reached
    }
}
```

---

### 6. shared_library.rs (Collaboration Module)
**File:** `crates/collaboration/src/shared_library.rs`
**Priority:** HIGH
**Stub Count:** 5 TODOs

**Implementation Template:**
```rust
pub struct SharedLibrary {
    rules: Arc<RwLock<HashMap<String, SharedRule>>>,
    version_history: Arc<RwLock<Vec<Version>>>,
}

#[derive(Debug, Clone)]
pub struct SharedRule {
    pub rule_id: String,
    pub version: String,
    pub content: String,
    pub owner_org: String,
    pub shared_with: Vec<String>,
    pub last_modified: i64,
}

impl SharedLibrary {
    pub async fn add_rule(&self, rule: SharedRule) -> Result<()> {
        // 1. Validate rule syntax
        // 2. Check permissions
        // 3. Store in database
        // 4. Create version entry
        // 5. Notify subscribers
    }

    pub async fn sync_rules(&self, target_org: &str) -> Result<Vec<SharedRule>> {
        // 1. Query shared rules for target organization
        // 2. Check version compatibility
        // 3. Download rules from repository
        // 4. Update local cache
        // 5. Return synchronized rules
    }

    pub async fn update_rule(&self, rule_id: &str, content: String) -> Result<()> {
        // 1. Load current rule
        // 2. Create new version
        // 3. Store in database
        // 4. Update cache
        // 5. Notify subscribers
    }

    pub async fn remove_rule(&self, rule_id: &str) -> Result<()> {
        // 1. Check permissions
        // 2. Mark as deprecated
        // 3. Notify subscribers
        // 4. Archive in database
    }
}
```

---

### 7. storage.rs (Test Orchestrator)
**File:** `crates/test-orchestrator/src/storage.rs`
**Priority:** HIGH
**Stub Count:** 3 TODOs

**Implementation Template:**
```rust
pub struct TestResultStorage {
    db_client: Arc<AriaDBClient>,
    event_publisher: Arc<EventPublisher>,
    content_store: Arc<ContentAddressedStore>,
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    pub test_id: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub output: String,
    pub timestamp: i64,
}

impl TestResultStorage {
    pub async fn store_result(&self, result: TestResult) -> Result<()> {
        // 1. Hash result output (SHA256)
        // 2. Store content in CAS
        // 3. Insert metadata into AriaDB
        // 4. Publish event to Universe event bus
        // 5. Return content hash
    }

    pub async fn publish_event(&self, event_type: &str, data: &TestResult) -> Result<()> {
        // 1. Serialize result to JSON
        // 2. Create event with timestamp
        // 3. Publish to "test:results" topic
        // 4. Log event for audit trail
    }

    pub async fn retrieve_result(&self, test_id: &str) -> Result<TestResult> {
        // 1. Query AriaDB for metadata
        // 2. Get content hash from metadata
        // 3. Retrieve content from CAS
        // 4. Deserialize and return
    }

    pub async fn query_results(&self, filter: ResultFilter) -> Result<Vec<TestResult>> {
        // 1. Build SQL query from filter
        // 2. Execute in AriaDB
        // 3. Retrieve content for results
        // 4. Return results
    }
}
```

**Dependencies to Add:**
```toml
[dependencies]
aria-db = { version = "0.1", features = ["client"] }
sha2 = "0.10"
```

---

### 8. auto_fixer.rs (Bug Hunter)
**File:** `crates/bug-hunter/src/auto_fixer.rs`
**Priority:** HIGH
**Stub Count:** Multiple references to StubType enum

**Implementation Template:**
```rust
pub struct AutoFixer {
    file_content: String,
    syntax_tree: Option<ParseTree>,
}

#[derive(Debug, Clone)]
pub enum StubType {
    Todo,
    Fixme,
    Unimplemented,
    Panic,
    Unwrap,
    IgnoredTest,
    EmptyBody,
}

impl AutoFixer {
    pub async fn fix_stubs(&mut self) -> Result<Vec<Fix>> {
        // 1. Parse file syntax tree
        // 2. Find all stub types
        // 3. For each stub:
        //    - Determine context
        //    - Generate appropriate fix
        //    - Apply fix to AST
        // 4. Generate new source code
        // 5. Return list of applied fixes
    }

    fn fix_todo(&self, location: &Location) -> Result<Fix> {
        // Convert TODO comment to proper implementation
        // Generate skeleton based on context
    }

    fn fix_unimplemented(&self, location: &Location) -> Result<Fix> {
        // Replace unimplemented!() with default implementation
        // Use type information from context
    }

    fn fix_panic(&self, location: &Location) -> Result<Fix> {
        // Replace panic!() with error handling
        // Use return type from context
    }

    fn fix_unwrap(&self, location: &Location) -> Result<Fix> {
        // Replace unwrap() with proper error handling
        // Generate error path
    }

    fn fix_ignored_test(&self, location: &Location) -> Result<Fix> {
        // Remove #[ignore] attribute
        // Enable test
    }

    fn fix_empty_body(&self, location: &Location) -> Result<Fix> {
        // Generate implementation based on signature
        // Use ML model for code generation
    }
}
```

---

### 9. integration/mod.rs
**File:** `crates/lint/src/integration/mod.rs`
**Priority:** HIGH
**Stub Count:** 3 TODOs

**Implementation Template:**
```rust
pub struct LintIntegration {
    audit_log: Arc<AuditLogger>,
    bug_hunt_client: Arc<BugHuntClient>,
    telemetry: Arc<TelemetryClient>,
}

impl LintIntegration {
    pub async fn new() -> Result<Self> {
        // 1. Initialize audit log system
        // 2. Initialize bug hunt service client
        // 3. Initialize telemetry/observability client
        // 4. Return integrated system
    }

    pub async fn on_lint_complete(&self, summary: &LintSummary) -> Result<()> {
        // 1. Log to audit trail
        self.audit_log.log_lint_run(summary).await?;

        // 2. Submit high-priority findings to bug hunt
        self.bug_hunt_client.submit_findings(
            summary.critical_diagnostics.clone()
        ).await?;

        // 3. Publish telemetry metrics
        self.telemetry.publish_metrics(&LintMetrics {
            total_files: summary.files_linted,
            total_issues: summary.total_diagnostics,
            duration: summary.duration_ms,
        }).await?;

        Ok(())
    }

    pub async fn log_rule_execution(&self, rule_id: &str, result: &RuleResult) -> Result<()> {
        // 1. Create audit log entry
        // 2. Record execution time
        // 3. Track rule effectiveness
    }

    pub async fn report_error(&self, error: &LintError) -> Result<()> {
        // 1. Log error to audit trail
        // 2. Send alert via telemetry
        // 3. Create incident if critical
    }
}
```

---

## Implementation Order (Recommended)

1. ✅ **survival_feedback.rs** (DONE)
2. ✅ **team_profiles.rs** (DONE)
3. **integration/mod.rs** (Start here - foundation for others)
4. **bug_hunt_orchestrator.rs** (Depends on integration/mod.rs)
5. **incremental.rs** (Independent - language support)
6. **voting.rs** (Independent - collaboration)
7. **shared_library.rs** (Depends on voting)
8. **storage.rs** (Independent - storage backend)
9. **auto_fixer.rs** (Independent - code generation)

---

## Quick Implementation Checklist

For each remaining file:
- [ ] Read full file content
- [ ] Identify all TODO/unimplemented!() locations
- [ ] Add required helper structures
- [ ] Implement persistence layer (database or file-based)
- [ ] Add error handling for all operations
- [ ] Implement async/await properly
- [ ] Add logging at key points
- [ ] Write tests for main functions
- [ ] Verify compilation with `cargo check`

---

## Testing Each Implementation

```bash
# After implementing each file:

# Check compilation
cargo check -p crate-name

# Run tests
cargo test -p crate-name

# Run clippy for best practices
cargo clippy -p crate-name

# Check formatting
cargo fmt --check -p crate-name
```

---

## Database Integration Pattern

For all database operations, use this pattern:

```rust
async fn persist_to_db(&self, item: &T) -> Result<()> {
    let json = serde_json::to_string(item)?;
    let file_path = self.db_path.join(format!("{}.json", item.id));
    tokio::fs::write(file_path, json).await?;
    tracing::debug!("Persisted: {}", item.id);
    Ok(())
}

async fn load_from_db(&self, id: &str) -> Result<Option<T>> {
    let file_path = self.db_path.join(format!("{}.json", id));
    if !file_path.exists() {
        return Ok(None);
    }
    let json = tokio::fs::read_to_string(file_path).await?;
    Ok(serde_json::from_str(&json)?)
}
```

---

## Status Summary

| File | Status | Effort | Priority |
|------|--------|--------|----------|
| survival_feedback.rs | ✅ COMPLETE | 3 hrs | 🔴 CRITICAL |
| team_profiles.rs | ✅ COMPLETE | 2 hrs | 🔴 CRITICAL |
| bug_hunt_orchestrator.rs | 📋 PLAN | 2 hrs | 🔴 CRITICAL |
| incremental.rs | 📋 PLAN | 4 hrs | 🔴 CRITICAL |
| voting.rs | 📋 PLAN | 3 hrs | 🔴 CRITICAL |
| shared_library.rs | 📋 PLAN | 3 hrs | 🔴 CRITICAL |
| storage.rs | 📋 PLAN | 3 hrs | 🔴 CRITICAL |
| auto_fixer.rs | 📋 PLAN | 5 hrs | 🔴 CRITICAL |
| integration/mod.rs | 📋 PLAN | 2 hrs | 🔴 CRITICAL |

**Total Remaining Effort:** ~25 hours of implementation

---

## Next Steps

1. **Immediate:** Implement integration/mod.rs (foundation)
2. **Short-term:** Complete bug_hunt_orchestrator.rs and incremental.rs
3. **Medium-term:** Finish voting.rs, shared_library.rs, storage.rs
4. **Long-term:** Implement auto_fixer.rs (most complex)
5. **Testing:** Run full test suite after each file

