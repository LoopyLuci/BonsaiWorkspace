# Worker Implementation Template

## 5-Minute Pattern for Adding New Workers

### Step 1: Create Module File
File: `crates/{category}/src/{worker_name_snake}.rs`

```rust
/// {WorkerName} - {description}

use omnisystem_workers_core::prelude::*;
use async_trait::async_trait;
use std::time::Duration;

pub struct {WorkerName} {
    // Configuration fields
}

pub struct {InputType} {
    // Request structure
}

pub enum {OutputType} {
    // Response variants
}

#[async_trait]
impl Worker for {WorkerName} {
    type Input = {InputType};
    type Output = {OutputType};

    async fn execute(&self, input: Self::Input) -> WorkerResult<Self::Output> {
        // Implementation
        Ok({OutputType}::Success)
    }

    fn name(&self) -> &str {
        "{WorkerName}"
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(X)
    }

    fn priority(&self) -> Priority {
        Priority::Normal
    }
}

impl {WorkerName} {
    pub fn new() -> Self {
        {WorkerName} {
            // Initialize fields
        }
    }
}
```

### Step 2: Export in lib.rs
```rust
pub mod {worker_name_snake};
pub use {worker_name_snake}::{WorkerName};
```

### Step 3: Run Tests
```bash
cargo test -p omnisystem-workers-{category}
```

### Step 4: Commit
Each worker = 1 commit with clear message

---

## Time Budget per Worker
- Setup (copy template): 30 seconds
- Implementation: 3-5 minutes
- Testing: 1 minute
- **Total: 5-10 minutes per worker**

## Recommended Batching
- Batch 3-5 workers per commit
- Test each batch
- Total: 17 workers = ~90 minutes

---

## Key Patterns

### Request/Response Separation
```rust
pub struct WorkerRequest {
    pub input: String,
    pub options: HashMap<String, String>,
}

pub enum WorkerResult {
    Success(OutputData),
    Partial(PartialData),
    Error(String),
}
```

### Timeout Guidelines
- I/O workers: 30s
- Network workers: 60s
- Compute workers: 15s
- Device workers: 10s
- Process workers: 20s
- Database workers: 45s

### Priority Assignment
- I/O Bound: `Normal`
- Network: `High` (time-sensitive)
- Compute: `Normal`
- Device: `High` (responsive)
- Process: `High` (system)
- Database: `High` (consistency)

---

## Testing Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_success() {
        let worker = {WorkerName}::new();
        let input = {InputType} { /* ... */ };
        let result = worker.execute(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_worker_error() {
        let worker = {WorkerName}::new();
        let input = {InputType} { /* invalid */ };
        let result = worker.execute(input).await;
        assert!(result.is_err());
    }
}
```

---

## Category Guidelines

### I/O Workers (File System)
- Focus on file operations (monitor, cache, buffer)
- Use `tokio::fs` for async file I/O
- Return file data or status
- Timeout: 30s

### Network Workers (Communication)
- Focus on protocols (FTP, proxy, DNS server)
- Use `tokio::net` for networking
- Handle connections asynchronously
- Timeout: 60s

### Compute Workers (Data Processing)
- Focus on algorithms (decompression, parsing)
- Use existing crates (flate2, serde_xml, serde_yaml)
- Process data efficiently
- Timeout: 15s

### Device Workers (Hardware)
- Focus on device communication (Bluetooth, USB)
- Use device-specific libraries
- Handle async operations
- Timeout: 10s

### Process Workers (System)
- Focus on process lifecycle
- Use `std::process` or async wrappers
- Monitor and manage processes
- Timeout: 20s

### Database Workers (Data Storage)
- Focus on database operations (transactions, indices)
- Use sqlx or equivalent
- Handle transactions safely
- Timeout: 45s

---

## Dependencies to Add

### I/O
```toml
notify = "6.1"        # File monitoring
lru = "0.12"          # Cache implementation
```

### Network
```toml
reqwest = "0.11"      # HTTP client
tonic = "0.11"        # gRPC
```

### Compute
```toml
flate2 = "1.0"        # Decompression
serde_xml_rs = "0.5"  # XML parsing
serde_yaml = "0.9"    # YAML parsing
```

### Device
```toml
rusb = "0.9"          # USB access
bluez = "0.9"         # Bluetooth
```

### Process
```toml
nix = "0.27"          # Unix process APIs
```

### Database
```toml
sqlx = { version = "0.7", features = ["runtime-tokio"] }
```

---

## Quick Checklist

- [ ] Module file created
- [ ] Struct defined (Worker + Input/Output types)
- [ ] Worker trait implemented
- [ ] name() returns correct string
- [ ] timeout() returns appropriate Duration
- [ ] priority() set for category
- [ ] Implementation is async and uses appropriate I/O
- [ ] Error handling in place
- [ ] Tests written
- [ ] Export added to lib.rs
- [ ] Cargo.toml has required dependencies
- [ ] `cargo test` passes
- [ ] Commit message clear

---

## Performance Targets

- **Implementation time**: 5-10 minutes/worker
- **Compilation time**: <10 seconds per worker
- **Test pass rate**: 100%
- **Code coverage**: >80%

---

## Quality Gates

Must pass before commit:
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] No clippy warnings
- [ ] Async/await used correctly
- [ ] Error handling proper
- [ ] Documentation present
