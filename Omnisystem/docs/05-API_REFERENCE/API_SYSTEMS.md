# Systems Framework API Reference

**Complete API reference for OS integration and system operations**

---

## Module Overview

The Systems Framework provides:
- **Process Management**: Spawn, monitor, control processes
- **File I/O**: Read, write, delete files and directories
- **Memory**: System and process memory information
- **Threading**: Thread pools and synchronization
- **Networking**: TCP and UDP sockets

---

## Core Types

### Process

**Manage and monitor system processes**

```rust
pub struct Process {
    pid: u32,
    command: String,
    args: Vec<String>,
}

impl Process {
    // Spawning
    pub fn spawn(command: &str, args: Vec<&str>) -> Result<Self>
    pub fn spawn_with_env(
        command: &str,
        args: Vec<&str>,
        env: HashMap<String, String>
    ) -> Result<Self>
    pub fn spawn_in(command: &str, args: Vec<&str>, cwd: &str) -> Result<Self>
    
    // Process control
    pub fn pid(&self) -> u32
    pub fn command(&self) -> &str
    pub fn is_running(&self) -> bool
    pub fn wait(&mut self) -> Result<ExitStatus>
    pub fn wait_timeout(&mut self, timeout: Duration) -> Result<Option<ExitStatus>>
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>>
    pub fn kill(&mut self) -> Result<()>
    pub fn kill_after(&mut self, timeout: Duration) -> Result<()>
    pub fn suspend(&mut self) -> Result<()>
    pub fn resume(&mut self) -> Result<()>
    
    // Information
    pub fn resource_usage(&self) -> Result<ResourceUsage>
    pub fn list_all() -> Result<Vec<ProcessInfo>>
    pub fn get_info(pid: u32) -> Result<ProcessInfo>
}

pub struct ExitStatus {
    code: i32,
    success: bool,
}

pub struct ResourceUsage {
    pub user_cpu_secs: f64,
    pub system_cpu_secs: f64,
    pub memory_mb: u64,
    pub max_memory_mb: u64,
}
```

**Example:**
```rust
let process = Process::spawn("echo", vec!["Hello"])?
let status = process.wait()?
println!("Exit code: {}", status.code)
```

---

### File Operations

**Read, write, and manage files**

```rust
pub struct File {
    path: String,
    handle: FileHandle,
}

impl File {
    // Read operations
    pub fn read(path: &str) -> Result<String>
    pub fn read_bytes(path: &str) -> Result<Vec<u8>>
    pub fn open(path: &str) -> Result<FileHandle>
    
    // Write operations
    pub fn write(path: &str, content: &str) -> Result<()>
    pub fn write_bytes(path: &str, data: &[u8]) -> Result<()>
    pub fn create(path: &str) -> Result<Self>
    pub fn append(path: &str, content: &str) -> Result<()>
    
    // File operations
    pub fn exists(path: &str) -> bool
    pub fn delete(path: &str) -> Result<()>
    pub fn rename(from: &str, to: &str) -> Result<()>
    pub fn copy(from: &str, to: &str) -> Result<()>
    pub fn size(path: &str) -> Result<u64>
    pub fn metadata(path: &str) -> Result<Metadata>
}

pub struct Metadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub permissions: u32,
    pub created_at: SystemTime,
    pub modified_at: SystemTime,
    pub accessed_at: SystemTime,
}
```

**Example:**
```rust
let content = File::read("data.txt")?
File::write("output.txt", &content)?
let size = File::size("data.txt")?
```

---

### Directory Operations

**Create, list, and remove directories**

```rust
pub struct Dir {
    path: String,
}

impl Dir {
    pub fn current() -> Result<String>
    pub fn change(path: &str) -> Result<()>
    pub fn create(path: &str) -> Result<()>
    pub fn create_recursive(path: &str) -> Result<()>
    pub fn remove(path: &str) -> Result<()>
    pub fn remove_recursive(path: &str) -> Result<()>
    pub fn read(path: &str) -> Result<Vec<DirEntry>>
    pub fn exists(path: &str) -> bool
}

pub struct DirEntry {
    name: String,
    path: String,
    is_file: bool,
    is_dir: bool,
    size: u64,
}

impl DirEntry {
    pub fn name(&self) -> &str
    pub fn path(&self) -> &str
    pub fn is_file(&self) -> bool
    pub fn is_dir(&self) -> bool
    pub fn size(&self) -> u64
}
```

**Example:**
```rust
Dir::create("data")?
for entry in Dir::read(".")? {
    println!("{}: {} bytes", entry.name(), entry.size())
}
Dir::remove_recursive("data")?
```

---

### Path Operations

**Manipulate file paths**

```rust
pub struct Path {
    path: String,
}

impl Path {
    pub fn new(path: &str) -> Self
    pub fn parent(&self) -> String
    pub fn file_name(&self) -> String
    pub fn file_stem(&self) -> String
    pub fn extension(&self) -> String
    pub fn join(&self, other: &str) -> String
    pub fn canonicalize(&self) -> Result<String>
    pub fn is_file(&self) -> bool
    pub fn is_dir(&self) -> bool
    pub fn is_symlink(&self) -> bool
    pub fn exists(&self) -> bool
}
```

**Example:**
```rust
let path = Path::new("/home/user/file.txt")
let parent = path.parent()
let name = path.file_name()
let stem = path.file_stem()
let ext = path.extension()
```

---

### Memory Information

**Query system and process memory**

```rust
pub struct Memory;

impl Memory {
    // System memory
    pub fn system_info() -> MemoryInfo
    pub fn current_process() -> ProcessMemory
    
    // Memory allocation
    pub fn allocate(size: usize) -> Result<MemoryHandle>
    pub fn allocate_aligned(size: usize, alignment: usize) -> Result<MemoryHandle>
    
    // Monitoring
    pub fn track_allocation<F>(callback: F) -> MemoryTracker where F: Fn(u64)
}

pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_percent: f32,
    pub total_mb: u64,
    pub available_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
}

pub struct ProcessMemory {
    pub rss_mb: u64,      // Resident set size
    pub vms_mb: u64,      // Virtual memory size
    pub private_mb: u64,  // Private memory
    pub shared_mb: u64,   // Shared memory
}
```

**Example:**
```rust
let info = Memory::system_info()
println!("Free: {} MB", info.free_mb)

let proc = Memory::current_process()
println!("RSS: {} MB", proc.rss_mb)
```

---

### Threading

**Multi-threaded execution with synchronization**

```rust
pub mod thread {
    pub fn spawn<F>(f: F) -> JoinHandle<T>
    where F: FnOnce() -> T + Send + 'static
    
    pub fn current_id() -> ThreadId
    pub fn current_name() -> Option<String>
    pub fn sleep(duration: Duration)
    pub fn yield_now()
}

pub struct JoinHandle<T> {
    // Handle to joined thread
}

impl<T> JoinHandle<T> {
    pub fn join(self) -> Result<T>
    pub fn is_finished(&self) -> bool
}
```

**Example:**
```rust
let handle = thread::spawn(|| {
    println!("Hello from thread!")
})
handle.join()?
```

---

### ThreadPool

**Managed thread pool for task execution**

```rust
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Task>,
}

impl ThreadPool {
    pub fn new(num_workers: usize) -> Self
    pub fn with_config(config: ThreadPoolConfig) -> Self
    pub fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static
    pub fn shutdown(self) -> Result<()>
    pub fn shutdown_timeout(self, timeout: Duration) -> Result<()>
}

pub struct ThreadPoolConfig {
    pub num_workers: usize,
    pub stack_size: usize,
    pub name: String,
}
```

**Example:**
```rust
let pool = ThreadPool::new(4)
for i in 0..100 {
    pool.execute(move || {
        println!("Task {}", i)
    })
}
pool.shutdown()?
```

---

### Synchronization Primitives

**Thread synchronization tools**

```rust
pub use std::sync::{
    Mutex,          // Mutual exclusion
    RwLock,         // Read-write lock
    Semaphore,      // Counter-based synchronization
    CondVar,        // Condition variable
    Barrier,        // Thread barrier
    Once,           // One-time execution
};

pub use std::sync::atomic::{
    AtomicBool,
    AtomicI32,
    AtomicU64,
    Ordering,
};
```

**Example:**
```rust
let counter = Arc::new(Mutex::new(0))
{
    let mut c = counter.lock()?
    *c += 1
}

let shared = Arc::new(RwLock::new(vec![]))
{
    let items = shared.read()?
    println!("{:?}", items)
}
```

---

### Network Sockets

**TCP and UDP networking**

```rust
pub struct TcpListener {
    // Socket listener
}

impl TcpListener {
    pub fn bind(addr: &str) -> Result<Self>
    pub fn incoming(&self) -> Incoming
}

pub struct TcpStream {
    // TCP connection
}

impl TcpStream {
    pub fn connect(addr: &str) -> Result<Self>
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize>
    pub fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize>
    pub fn write_all(&mut self, buf: &[u8]) -> Result<()>
    pub fn shutdown(&mut self) -> Result<()>
}

pub struct UdpSocket {
    // UDP socket
}

impl UdpSocket {
    pub fn bind(addr: &str) -> Result<Self>
    pub fn send_to(&self, buf: &[u8], addr: &str) -> Result<usize>
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, String)>
}
```

**Example:**
```rust
let listener = TcpListener::bind("127.0.0.1:8080")?
for stream in listener.incoming() {
    let mut stream = stream?
    stream.write_all(b"Hello")?
}
```

---

## Error Types

### SystemError

**System operation errors**

```rust
pub enum SystemError {
    ProcessError(String),
    FileNotFound(String),
    PermissionDenied(String),
    AlreadyExists(String),
    InvalidPath(String),
    IoError(String),
    Timeout,
    ThreadPanicked,
    SocketError(String),
}
```

---

## Usage Patterns

### Process Spawning

```rust
// Simple execution
let mut process = Process::spawn("ls", vec!["-la"])?
process.wait()?

// With environment
let mut env = HashMap::new()
env.insert("PATH".to_string(), "/usr/bin".to_string())
let process = Process::spawn_with_env("command", vec![], env)?

// With working directory
let process = Process::spawn_in("make", vec!["build"], "/project")?
```

### File Processing

```rust
// Read and process
let content = File::read("input.txt")?
let processed = process_content(&content)
File::write("output.txt", &processed)?

// Directory traversal
for entry in Dir::read(".")? {
    if entry.is_file() {
        let data = File::read(&entry.path())?
        println!("Processed {}", entry.name())
    }
}
```

### Concurrent Task Execution

```rust
let pool = ThreadPool::new(8)

for task in tasks {
    let data = task.data.clone()
    pool.execute(move || {
        let result = process(data)
        store_result(result)
    })
}

pool.shutdown()?
```

---

## Constants

### Process Signals

```rust
pub const SIGTERM: i32 = 15  // Terminate
pub const SIGKILL: i32 = 9   // Kill
pub const SIGUSR1: i32 = 10  // User 1
pub const SIGUSR2: i32 = 12  // User 2
```

### File Permissions

```rust
pub const PERM_OWNER_READ: u32 = 0o400
pub const PERM_OWNER_WRITE: u32 = 0o200
pub const PERM_OWNER_EXEC: u32 = 0o100
pub const PERM_GROUP_READ: u32 = 0o040
pub const PERM_GROUP_WRITE: u32 = 0o020
pub const PERM_GROUP_EXEC: u32 = 0o010
pub const PERM_OTHERS_READ: u32 = 0o004
pub const PERM_OTHERS_WRITE: u32 = 0o002
pub const PERM_OTHERS_EXEC: u32 = 0o001
```

---

## Examples

### System Monitoring

```rust
use omnisystem::system::*

let info = Memory::system_info()
println!("Memory Usage: {:.1}%", info.usage_percent)

let procs = Process::list_all()?
for proc in procs.iter().take(5) {
    println!("{}: {} MB", proc.pid(), proc.resource_usage()?.memory_mb)
}
```

### File Processing Pipeline

```rust
// Read input
let input = File::read("data.json")?

// Process
let output = process_json(&input)?

// Write result
File::write("result.json", &output)?

// Copy to backup
File::copy("result.json", "result.backup.json")?
```

### Thread Pool Worker

```rust
let pool = ThreadPool::new(4)

for item in items {
    pool.execute(move || {
        let result = expensive_computation(item)
        store_result(result)
    })
}

pool.shutdown()?
```

---

## Testing

### File Tests

```rust
#[test]
fn test_file_operations() {
    File::write("test.txt", "content").unwrap()
    assert!(File::exists("test.txt"))
    
    let content = File::read("test.txt").unwrap()
    assert_eq!(content, "content")
    
    File::delete("test.txt").unwrap()
}
```

### Process Tests

```rust
#[test]
fn test_process_spawn() {
    let mut p = Process::spawn("echo", vec!["test"]).unwrap()
    let status = p.wait().unwrap()
    assert!(status.success)
}
```

---

## Performance Notes

- Use **thread pools** instead of spawning threads per task
- **Buffer file I/O** for better throughput
- **Use RwLock** for read-heavy workloads
- Monitor **memory usage** to prevent leaks
- Batch **file operations** when possible

---

## See Also
- [SYSTEMS_FRAMEWORK_GUIDE.md](SYSTEMS_FRAMEWORK_GUIDE.md) - Framework tutorial
- [TUTORIAL_SYSTEMS.md](TUTORIAL_SYSTEMS.md) - Systems example
- [SYSTEMS_FRAMEWORK_SPECIFICATION.md](SYSTEMS_FRAMEWORK_SPECIFICATION.md) - Formal spec

---

**Last Updated**: 2026-06-15
