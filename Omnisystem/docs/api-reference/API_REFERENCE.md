# OMNISYSTEM ENTERPRISE TIER 1 - API REFERENCE

## 🔌 Complete API Documentation

---

## IDE API

### Editor Management

#### `IDEEditor::new(file_path: &str) -> IDEEditor`
Initialize a new editor instance for a file.
```vera
let mut editor = IDEEditor::new("example.titan");
// editor.current_file.path = "example.titan"
// editor.current_file.language = "titan"
```

#### `insert_text(&mut self, text: &str)`
Insert text at current cursor position. Marks file as dirty.
```vera
editor.insert_text("pub fn main() {");
```

#### `update_syntax_tree(&mut self)`
Parse editor content and generate syntax tree with tokens. Called automatically after modifications.
```vera
editor.update_syntax_tree();
// Generates SyntaxNode tree for code navigation
```

#### `get_suggestions(&mut self, position: usize) -> Vec<Suggestion>`
Get autocomplete suggestions at cursor position.
```vera
let suggestions = editor.get_suggestions(42);
// Returns: [Suggestion { label: "function", kind: "keyword", ... }]
```

#### `save_file(&self) -> Result<(), String>`
Write editor content to disk.
```vera
match editor.save_file() {
    Ok(_) => println!("File saved"),
    Err(e) => println!("Error: {}", e),
}
```

#### `format_code(&mut self) -> Result<(), String>`
Format code according to language-specific rules.
```vera
editor.format_code()?;
// Aligns indentation, spacing, etc.
```

### Compilation

#### `CompilerInteraction::compile_file(&self, file_path: &str) -> Result<Vec<CompilerMessage>, String>`
Invoke compiler on file and get diagnostics.
```vera
let messages = compiler.compile_file("main.titan")?;
// Returns errors, warnings, hints with file:line:col
```

#### `get_diagnostics(&self) -> Vec<CompilerMessage>`
Get current diagnostic messages from last compilation.
```vera
let diagnostics = compiler.get_diagnostics();
for msg in diagnostics {
    println!("{}:{} - {}", msg.file, msg.line, msg.message);
}
```

### Debugging

#### `DebuggerInterface::start_debugging(&self, executable: &str) -> Result<(), String>`
Attach debugger to executable.
```vera
debugger.start_debugging("./program")?;
```

#### `set_breakpoint(&mut self, file: &str, line: u32) -> Result<(), String>`
Set conditional breakpoint at file:line.
```vera
debugger.set_breakpoint("main.titan", 42)?;
```

#### `step_over(&self) -> Result<(), String>`
Execute current line and pause at next line.
```vera
debugger.step_over()?;
```

#### `step_into(&self) -> Result<(), String>`
Enter function call at cursor.
```vera
debugger.step_into()?;
```

#### `continue_execution(&self) -> Result<(), String>`
Resume execution until next breakpoint.
```vera
debugger.continue_execution()?;
```

### Profiling

#### `ProfilerInterface::start_profiling(&self) -> Result<(), String>`
Begin performance profiling of running program.
```vera
profiler.start_profiling()?;
```

#### `stop_profiling(&self) -> Result<ProfilerData, String>`
End profiling and return collected samples.
```vera
let data = profiler.stop_profiling()?;
println!("Total time: {} ms", data.total_time_ms);
```

### Git Integration

#### `GitInterface::new(repo_path: &str) -> GitInterface`
Initialize git interface for repository.
```vera
let git = GitInterface::new(".");
```

#### `get_status(&self) -> Result<GitStatus, String>`
Query current git status.
```vera
let status = git.get_status()?;
println!("Branch: {}", status.current_branch);
println!("Modified: {:?}", status.modified_files);
```

#### `stage_file(&self, file: &str) -> Result<(), String>`
Stage file for commit.
```vera
git.stage_file("src/main.titan")?;
```

#### `commit(&self, message: &str) -> Result<(), String>`
Create commit with message.
```vera
git.commit("feat: Add new feature")?;
```

#### `push(&self) -> Result<(), String>`
Push commits to remote.
```vera
git.push()?;
```

### Package Management

#### `PackageManager::new() -> PackageManager`
Initialize package manager.
```vera
let mut pkg_mgr = PackageManager::new();
```

#### `search(&self, query: &str) -> Vec<Package>`
Search package registry.
```vera
let results = pkg_mgr.search("logging");
for pkg in results {
    println!("{}: {}", pkg.name, pkg.version);
}
```

#### `install(&mut self, name: &str, version: &str) -> Result<(), String>`
Install package with version.
```vera
pkg_mgr.install("omnisystem-stdlib", "1.0.0")?;
```

#### `update_all(&mut self) -> Result<(), String>`
Update all dependencies to latest versions.
```vera
pkg_mgr.update_all()?;
```

---

## Database API

### Database Management

#### `DistributedDatabase::new(name: &str) -> DistributedDatabase`
Create new distributed database instance.
```titan
let mut db = DistributedDatabase::new("myapp_db");
```

#### `add_node(&mut self, node: DatabaseNode)`
Add node to cluster.
```titan
let node = DatabaseNode {
    node_id: 1,
    host: "localhost".to_string(),
    port: 5432,
    is_master: true,
    storage_engine: StorageEngine::new(),
    replica_set: vec![2, 3],
    status: NodeStatus::Healthy,
};
db.add_node(node);
```

### Schema Operations

#### `create_table(&mut self, table_name: &str, schema: Schema) -> Result<(), String>`
Create new table with schema.
```titan
let schema = Schema {
    columns: vec![
        ColumnDef {
            name: "id".to_string(),
            data_type: DataType::UUID,
            nullable: false,
            default_value: None,
        },
        ColumnDef {
            name: "email".to_string(),
            data_type: DataType::String,
            nullable: false,
            default_value: None,
        },
    ],
    primary_key: "id".to_string(),
    indexes: vec!["email".to_string()],
};
db.create_table("users", schema)?;
```

### Data Manipulation

#### `insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), String>`
Insert key-value pair. Appends to transaction log.
```titan
db.insert(b"user:123".to_vec(), b"alice@example.com".to_vec())?;
```

#### `query(&self, sql: &str) -> Result<Vec<Vec<u8>>, String>`
Execute SQL query and return results.
```titan
let results = db.query("SELECT * FROM users WHERE age > 18")?;
for row in results {
    println!("{:?}", String::from_utf8(row));
}
```

### Transactions

#### `begin_transaction(&mut self) -> Result<u64, String>`
Begin new MVCC transaction. Returns transaction ID.
```titan
let xid = db.begin_transaction()?;
```

#### `commit_transaction(&mut self, xid: u64) -> Result<(), String>`
Commit transaction atomically.
```titan
db.commit_transaction(xid)?;
// All changes made in xid now visible
```

#### `rollback_transaction(&mut self, xid: u64) -> Result<(), String>`
Abort transaction and discard changes.
```titan
db.rollback_transaction(xid)?;
// Reverts all changes made in xid
```

### Backup & Recovery

#### `create_backup(&mut self) -> Result<Backup, String>`
Create incremental backup.
```titan
let backup = db.create_backup()?;
println!("Backup ID: {}", backup.backup_id);
```

#### `restore_from_backup(&mut self, backup_id: &str) -> Result<(), String>`
Restore database from backup point.
```titan
db.restore_from_backup("backup_1234567890")?;
```

### Partitioning

#### `get_partition(&self, key: &[u8]) -> Result<u32, String>`
Get partition ID for key (for partition-aware routing).
```titan
let partition_id = db.get_partition(b"user:123")?;
// Use partition_id to route to correct node
```

### Storage Engine

#### `StorageEngine::new() -> StorageEngine`
Initialize LSM-tree storage engine.
```titan
let mut engine = StorageEngine::new();
```

#### `get(&self, key: &[u8]) -> Option<Vec<u8>>`
Fast memtable lookup.
```titan
if let Some(value) = engine.get(b"key") {
    println!("Value: {:?}", value);
}
```

#### `put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), String>`
Insert into memtable.
```titan
engine.put(b"key".to_vec(), b"value".to_vec())?;
```

#### `delete(&mut self, key: &[u8]) -> Result<(), String>`
Delete key from storage.
```titan
engine.delete(b"key")?;
```

#### `compact(&mut self) -> Result<(), String>`
Trigger compaction (memtable → SSTable).
```titan
engine.compact()?;
```

### Bloom Filter

#### `BloomFilter::new(size: usize, hash_functions: u32) -> BloomFilter`
Create bloom filter for set membership testing.
```titan
let mut filter = BloomFilter::new(10000, 4);
```

#### `insert(&mut self, key: &[u8])`
Add key to bloom filter.
```titan
filter.insert(b"user:123");
```

#### `contains(&self, key: &[u8]) -> bool`
Test if key likely in set (false positive possible, false negative impossible).
```titan
if filter.contains(b"user:123") {
    // Key definitely or probably in set
}
```

---

## Monitoring API

### System Initialization

#### `MonitoringSystem::new(service_name: &str) -> MonitoringSystem`
Create monitoring system for service.
```aether
let mut monitoring = MonitoringSystem::new("api-service");
```

### Metrics Collection

#### `record_metric(&mut self, metric: Metric) -> Result<(), String>`
Record single metric.
```aether
let metric = Metric {
    name: "http_request_duration_ms".to_string(),
    metric_type: MetricType::Histogram,
    value: 125.5,
    timestamp: current_timestamp(),
    tags: HashMap::new(),
    unit: "ms".to_string(),
};
monitoring.record_metric(metric)?;
```

#### `aggregate_metrics(&mut self) -> Result<(), String>`
Calculate aggregates and percentiles across all collected metrics.
```aether
monitoring.aggregate_metrics()?;
// Populates: min, max, mean, p50, p95, p99, p999
```

### Distributed Tracing

#### `start_span(&mut self, operation_name: &str) -> String`
Begin trace span. Returns span ID.
```aether
let span_id = monitoring.start_span("process_request");
```

#### `end_span(&mut self, span_id: &str) -> Result<(), String>`
End trace span and record duration.
```aether
monitoring.end_span(&span_id)?;
// Duration = end_time - start_time
```

### Alerting

#### `add_alert_rule(&mut self, rule: AlertRule)`
Register alert rule.
```aether
let rule = AlertRule {
    rule_id: "high_latency".to_string(),
    name: "http_request_duration_ms".to_string(),
    condition: "mean > threshold".to_string(),
    threshold: 1000.0,
    duration_seconds: 60,
    severity: AlertSeverity::Critical,
    annotations: HashMap::new(),
};
monitoring.add_alert_rule(rule);
```

#### `evaluate_alert_rules(&mut self) -> Result<(), String>`
Check if alert conditions are met. Creates Alert structs if fired.
```aether
monitoring.evaluate_alert_rules()?;
```

#### `add_notification_channel(&mut self, channel: NotificationChannel)`
Register notification channel (Slack, Email, etc).
```aether
let channel = NotificationChannel {
    channel_id: "slack_on_call".to_string(),
    channel_type: NotificationType::Slack,
    config: {
        let mut m = HashMap::new();
        m.insert("webhook_url".to_string(), "https://hooks.slack.com/...".to_string());
        m
    },
    enabled: true,
};
monitoring.add_notification_channel(channel);
```

### Health Checking

#### `HealthChecker::new() -> HealthChecker`
Create health checker system.
```aether
let mut checker = HealthChecker::new();
```

#### `add_check(&mut self, check: HealthCheck)`
Register health check.
```aether
let check = HealthCheck {
    check_name: "database_alive".to_string(),
    check_type: CheckType::TCP,
    interval_seconds: 30,
    timeout_seconds: 5,
    last_result: HealthCheckResult {
        status: HealthStatus::Healthy,
        message: "OK".to_string(),
        response_time_ms: 50,
    },
    last_check_time: current_timestamp(),
};
checker.add_check(check);
```

#### `run_checks(&mut self) -> Result<(), String>`
Execute all health checks.
```aether
checker.run_checks()?;
// Updates last_result and last_check_time for each check
```

### Anomaly Detection

#### `AnomalyDetector::new(detector_id: &str, sensitivity: f64) -> AnomalyDetector`
Create anomaly detector with sensitivity (0.5-2.0, lower=more sensitive).
```aether
let mut detector = AnomalyDetector::new("perf_anomalies", 1.5);
```

#### `detect_anomalies(&mut self) -> Result<(), String>`
Score current metrics against baselines.
```aether
detector.detect_anomalies()?;
// Compares: (value - baseline.mean) / baseline.std_dev
// Sigma > sensitivity → anomaly
```

### Service Map

#### `ServiceMap::new() -> ServiceMap`
Create service dependency graph.
```aether
let mut svc_map = ServiceMap::new();
```

#### `add_service(&mut self, service: ServiceNode)`
Register service in map.
```aether
let svc = ServiceNode {
    service_name: "api".to_string(),
    instances: vec!["api-1".to_string(), "api-2".to_string()],
    error_rate: 0.001,
    latency_ms: 45.0,
    throughput_rps: 1000.0,
    dependencies: vec!["database".to_string()],
};
svc_map.add_service(svc);
```

#### `add_dependency(&mut self, dependency: ServiceDependency)`
Record service-to-service call metrics.
```aether
let dep = ServiceDependency {
    source_service: "api".to_string(),
    target_service: "database".to_string(),
    call_count: 1000000,
    error_count: 1000,
    latency_p50: 10.0,
    latency_p99: 100.0,
};
svc_map.add_dependency(dep);
```

#### `get_service_dependencies(&self, service_name: &str) -> Vec<&ServiceDependency>`
Get outbound dependencies for service.
```aether
let deps = svc_map.get_service_dependencies("api");
for dep in deps {
    println!("{} -> {} (error_rate: {})", 
        dep.source_service, 
        dep.target_service,
        dep.error_count as f64 / dep.call_count as f64);
}
```

---

## Type Definitions

### Metric
```aether
pub struct Metric {
    pub name: String,              // e.g., "http_request_duration_ms"
    pub metric_type: MetricType,   // Counter|Gauge|Histogram|Summary
    pub value: f64,                // Numeric value
    pub timestamp: u64,            // Unix seconds
    pub tags: HashMap<String, String>, // Dimensions: {"method": "GET", "path": "/api/users"}
    pub unit: String,              // e.g., "ms", "bytes", "rps"
}
```

### Span
```aether
pub struct Span {
    pub span_id: String,              // Unique span ID
    pub trace_id: String,             // Trace this span belongs to
    pub parent_span_id: Option<String>, // Parent span (if child)
    pub operation_name: String,       // e.g., "http.request"
    pub service_name: String,         // e.g., "api"
    pub start_time: u64,              // Unix nanos
    pub end_time: u64,                // Unix nanos
    pub duration_ms: u64,             // Calculated duration
    pub status: SpanStatus,           // Ok|Error|Cancelled
    pub tags: HashMap<String, String>, // Custom attributes
    pub logs: Vec<SpanLog>,           // Events during span
}
```

### CompilerMessage
```vera
pub struct CompilerMessage {
    pub severity: MessageSeverity,  // Error|Warning|Info|Hint
    pub file: String,              // Source file path
    pub line: u32,                 // Line number (1-based)
    pub column: u32,               // Column number (1-based)
    pub message: String,           // Error message
    pub suggestion: Option<String>, // Fix suggestion
}
```

---

## Error Handling

All fallible operations return `Result<T, String>`:

```vera
match operation() {
    Ok(value) => println!("Success: {:?}", value),
    Err(e) => eprintln!("Error: {}", e),
}
```

Common error messages:
- `"No nodes available"` - Database has no nodes
- `"Connection not found"` - TCP connection doesn't exist
- `"Partition not found"` - Key outside partition range
- `"No free blocks available"` - Storage full
- `"Span not found"` - Invalid span ID

---

## Concurrency

- **Thread-Safe Access:** `Arc<Mutex<T>>` for shared mutable state
- **Read-Only Access:** `Arc<RwLock<T>>` for readers + occasional writers
- **Single-Threaded:** Safe for use in single-threaded contexts

---

## Performance Characteristics

### IDE
- Syntax highlighting: O(n) per edit
- Autocompletion: O(log n) with trie
- Compilation: Depends on file size

### Database
- Memtable lookup: O(1) hash table
- Bloom filter: O(k) where k = hash functions
- Query: O(log n) for indexed columns
- Transaction: O(1) begin/commit

### Monitoring
- Metric recording: O(1) append
- Aggregation: O(n) scan + O(log n) percentile
- Trace: O(1) append per span

---

## Examples

### Complete IDE Session
```vera
let mut editor = IDEEditor::new("main.titan");
editor.insert_text("pub fn add(a: i32, b: i32) -> i32 { a + b }");
editor.update_syntax_tree();

let compiler = CompilerInteraction::new();
match compiler.compile_file("main.titan") {
    Ok(messages) => {
        for msg in messages {
            println!("{}:{}:{} - {}", msg.file, msg.line, msg.column, msg.message);
        }
    }
    Err(e) => println!("Compilation error: {}", e),
}

let git = GitInterface::new(".");
git.stage_file("main.titan")?;
git.commit("feat: Add addition function")?;
git.push()?;
```

### Complete Database Transaction
```titan
let mut db = DistributedDatabase::new("myapp");
let xid = db.begin_transaction()?;

db.insert(b"user:1".to_vec(), b"alice".to_vec())?;
db.insert(b"user:2".to_vec(), b"bob".to_vec())?;

db.commit_transaction(xid)?;
```

### Complete Monitoring Setup
```aether
let mut monitoring = MonitoringSystem::new("api");

// Alert when response time > 500ms for 60s
let rule = AlertRule {
    name: "http_response_time_ms".to_string(),
    threshold: 500.0,
    duration_seconds: 60,
    severity: AlertSeverity::Warning,
    // ...
};
monitoring.add_alert_rule(rule);

// Notify via Slack
let channel = NotificationChannel {
    channel_type: NotificationType::Slack,
    enabled: true,
    // ...
};
monitoring.add_notification_channel(channel);
```

---

## Standards Compliance

- **Monitoring:** OpenTelemetry semantic conventions
- **Metrics:** Prometheus format compatible
- **Tracing:** W3C Trace Context headers
- **Logging:** JSON structured logging

