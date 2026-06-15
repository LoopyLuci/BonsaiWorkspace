# Systems Framework Guide - OS Integration

**Control processes, files, memory, and threads with low-level system access**

---

## Table of Contents
1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Process Management](#process-management)
4. [File Operations](#file-operations)
5. [Memory Management](#memory-management)
6. [Threading](#threading)
7. [Networking](#networking)
8. [System Monitoring](#system-monitoring)
9. [Advanced Topics](#advanced-topics)

---

## Introduction

The Systems Framework provides:
- **Process Control**: Spawn, monitor, kill processes
- **File I/O**: Read, write, delete files and directories
- **Memory**: Query memory, manage allocation
- **Threading**: Thread pools, synchronization primitives
- **Networking**: Low-level socket operations

### Quick Facts
- **Language**: TITAN
- **API Style**: Procedural/Imperative
- **Error Handling**: Result types with context
- **Performance**: Zero-copy where possible
- **Portability**: Cross-platform abstraction

---

## Getting Started

### Simple Process Example

```titan
use omnisystem::system::*

fun main() -> Result<(), str> {
    // Spawn child process
    let child = Process::spawn("echo", vec!["Hello, System!"])?
    
    // Wait for completion
    let status = child.wait()?
    
    println!("Exit code: {}", status.code())
    
    Ok(())
}
```

### Run It

```bash
omnisystem run process.ti
# Hello, System!
# Exit code: 0
```

---

## Process Management

### Spawning Processes

```titan
use omnisystem::process::*
use std::time::Duration

// Simple spawn
let process = Process::spawn("ls", vec!["-la"])?

// With environment variables
let env = HashMap::from([
    ("PATH", "/usr/bin:/bin"),
    ("HOME", "/home/user"),
])
let process = Process::spawn_with_env("command", vec![], env)?

// With working directory
let process = Process::spawn_in("ls", vec!["-la"], "/tmp")?

// With timeout
let process = Process::spawn("sleep", vec!["1000"])?
process.kill_after(Duration::from_secs(5))?
```

### Process Monitoring

```titan
// Check if running
if process.is_running() {
    println!("Process is alive")
}

// Get PID
let pid = process.pid()
println!("Process ID: {}", pid)

// Get exit status (non-blocking)
match process.try_wait() {
    Ok(Some(status)) => println!("Exited: {}", status.code()),
    Ok(None) => println!("Still running"),
    Err(e) => println!("Error: {:?}", e),
}

// Wait with timeout
let status = process.wait_timeout(Duration::from_secs(30))?
```

### Process Operations

```titan
// Send signal to process
process.kill()?

// Suspend process
process.suspend()?

// Resume process
process.resume()?

// Get resource usage
let usage = process.resource_usage()?
println!("CPU time: {}s", usage.user_cpu_secs)
println!("Memory: {}MB", usage.memory_mb)

// List all processes
for proc in Process::list_all()? {
    println!("{}: {}", proc.pid(), proc.command())
}
```

---

## File Operations

### Reading Files

```titan
use omnisystem::fs::*

// Read entire file
let contents = File::read("data.txt")?
println!("{}", contents)

// Read with error handling
match File::read("missing.txt") {
    Ok(data) => println!("{}", data),
    Err(FileError::NotFound) => println!("File not found"),
    Err(e) => println!("Error: {:?}", e),
}

// Read in chunks
let file = File::open("large.bin")?
let mut buffer = vec![0; 1024]
while let Ok(n) = file.read(&mut buffer)? {
    if n == 0 { break }
    process_chunk(&buffer[..n])
}
```

### Writing Files

```titan
// Write entire file
File::write("output.txt", "Hello, World!")?

// Append to file
File::append("log.txt", "New log entry\n")?

// Write with options
let file = File::create("data.json")?
    .with_permissions(0o644)
    .truncate()

file.write_all(b"{"data": []}")?

// Atomic write (write to temp, then rename)
let temp = File::create("data.json.tmp")?
temp.write_all(b"new data")?
temp.rename("data.json")?
```

### Directory Operations

```titan
use omnisystem::fs::*

// Create directory
Dir::create("new_folder")?

// Create with parents
Dir::create_recursive("path/to/deep/folder")?

// List contents
for entry in Dir::read(".")? {
    println!("{}: {} bytes", 
        entry.name(),
        entry.size()
    )
}

// Remove directory (empty only)
Dir::remove("empty_folder")?

// Remove recursively
Dir::remove_recursive("folder_tree")?

// Get current directory
let cwd = Dir::current()?
println!("CWD: {}", cwd)

// Change directory
Dir::change("/tmp")?
```

### File Information

```titan
// Check existence
if File::exists("config.toml") {
    println!("Config found")
}

// Get file size
let size = File::size("data.bin")?
println!("Size: {} bytes", size)

// Get metadata
let meta = File::metadata("file.txt")?
println!("Modified: {:?}", meta.modified_at)
println!("Permissions: {:o}", meta.permissions)

// Copy file
File::copy("original.txt", "backup.txt")?

// Move/rename file
File::rename("old_name.txt", "new_name.txt")?

// Delete file
File::delete("temporary.txt")?
```

### Path Operations

```titan
use omnisystem::path::*

// Path manipulation
let path = Path::new("/home/user/documents/file.txt")

// Get components
let parent = path.parent()    // "/home/user/documents"
let name = path.file_name()   // "file.txt"
let stem = path.file_stem()   // "file"
let ext = path.extension()    // "txt"

// Join paths
let joined = path.parent().join("other.txt")

// Resolve relative paths
let abs = Path::new(".").canonicalize()?

// Check path type
if path.is_file() { println!("File") }
if path.is_dir() { println!("Directory") }
if path.is_symlink() { println!("Symlink") }
```

---

## Memory Management

### Memory Information

```titan
use omnisystem::memory::*

// Get system memory info
let info = Memory::system_info()
println!("Total: {} MB", info.total_mb)
println!("Available: {} MB", info.available_mb)
println!("Used: {} MB", info.used_mb)
println!("Free: {} MB", info.free_mb)
println!("Usage: {}%", info.usage_percent)

// Get process memory
let proc_mem = Memory::current_process()
println!("Process memory: {} MB", proc_mem.rss_mb)
println!("Virtual: {} MB", proc_mem.vms_mb)

// Monitor memory over time
for _ in 0..10 {
    let info = Memory::system_info()
    println!("Free: {} MB", info.free_mb)
    std::thread::sleep(Duration::from_secs(1))
}
```

### Memory Allocation

```titan
// Allocate memory
let mut buffer = vec![0u8; 1024 * 1024]  // 1MB

// Use custom allocator
let allocated = Memory::allocate(4096)?

// Free memory
allocated.deallocate()?

// Track allocation
let handle = Memory::track_allocation(|alloc_bytes| {
    println!("Allocated: {} bytes", alloc_bytes)
})
```

---

## Threading

### Basic Threading

```titan
use omnisystem::thread::*
use std::sync::{Arc, Mutex}

// Spawn thread
let handle = thread::spawn(|| {
    println!("Hello from thread!")
})

// Wait for thread
handle.join()?

// Get thread ID
let tid = thread::current_id()
println!("Thread ID: {}", tid)
```

### Thread Synchronization

```titan
use std::sync::Mutex

// Shared mutable data
let counter = Arc::new(Mutex::new(0))

// Spawn multiple threads
let mut handles = vec![]
for i in 0..10 {
    let counter = Arc::clone(&counter)
    let handle = thread::spawn(move || {
        let mut count = counter.lock().unwrap()
        *count += 1
    })
    handles.push(handle)
}

// Wait for all
for handle in handles {
    handle.join()?
}

// Get result
println!("Count: {}", *counter.lock().unwrap())
```

### Thread Pools

```titan
use omnisystem::thread::ThreadPool

// Create thread pool
let pool = ThreadPool::new(4)  // 4 worker threads

// Execute tasks
for i in 0..100 {
    pool.execute(move || {
        println!("Task {}", i)
    })
}

// Wait for completion
pool.shutdown()?

// Or with custom executor
let pool = ThreadPool::with_config(ThreadPoolConfig {
    num_workers: 8,
    stack_size: 2 * 1024 * 1024,
    name: "worker-pool",
})
```

### Synchronization Primitives

```titan
use std::sync::{Mutex, RwLock, Semaphore, CondVar}

// Mutex for mutual exclusion
let data = Mutex::new(vec![])
{
    let mut locked = data.lock()?
    locked.push(42)
}

// RwLock for read-write access
let shared = RwLock::new(HashMap::new())

// Multiple readers
{
    let reader = shared.read()?
    println!("{:?}", reader)
}

// Single writer
{
    let mut writer = shared.write()?
    writer.insert("key", "value")
}

// Condition variable
let cv = CondVar::new()
let lock = Mutex::new(false)

// Wait for signal
let mut flag = lock.lock()?
while !*flag {
    flag = cv.wait(flag)?
}

// Notify waiting threads
cv.notify_one()
```

---

## Networking

### TCP Sockets

```titan
use omnisystem::net::*

// Create server socket
let listener = TcpListener::bind("127.0.0.1:8080")?
println!("Listening on :8080")

for stream in listener.incoming() {
    let stream = stream?
    
    // Handle connection
    let mut buf = vec![0; 1024]
    let n = stream.read(&mut buf)?
    println!("Received: {} bytes", n)
    
    stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n")?
}

// Create client socket
let mut stream = TcpStream::connect("127.0.0.1:8080")?
stream.write_all(b"GET / HTTP/1.1\r\n\r\n")?

let mut response = vec![]
stream.read_to_end(&mut response)?
```

### UDP Sockets

```titan
use omnisystem::net::*

// UDP server
let socket = UdpSocket::bind("127.0.0.1:8080")?

let mut buf = vec![0; 1024]
let (n, addr) = socket.recv_from(&mut buf)?
println!("Received from {}: {:?}", addr, &buf[..n])

socket.send_to(b"Response", addr)?

// UDP client
let socket = UdpSocket::bind("127.0.0.1:0")?
socket.send_to(b"Hello", "127.0.0.1:8080")?
```

---

## System Monitoring

### System Information

```titan
use omnisystem::system::*

// Get system info
let info = System::info()
println!("OS: {}", info.os_name)
println!("Arch: {}", info.arch)
println!("Cores: {}", info.cpu_count)
println!("Hostname: {}", info.hostname)

// CPU information
let cpu = System::cpu_info()
println!("Model: {}", cpu.model_name)
println!("Speed: {} GHz", cpu.speed_ghz)
println!("Cache: {} KB", cpu.cache_kb)

// Uptime
let uptime = System::uptime()?
println!("Uptime: {} seconds", uptime.as_secs())
```

### System Monitoring Loop

```titan
fun monitor_system() -> Result<()> {
    loop {
        let mem = Memory::system_info()
        let procs = Process::list_all()?
        
        println!("\n=== System Monitor ===");
        println!("Memory: {:.1}% used", mem.usage_percent);
        println!("Processes: {}", procs.len());
        
        // Top 3 by memory
        let mut sorted = procs
        sorted.sort_by(|a, b| {
            b.memory_mb().cmp(&a.memory_mb())
        })
        
        for proc in sorted.iter().take(3) {
            println!("  {}: {}MB", proc.pid(), proc.memory_mb())
        }
        
        std::thread::sleep(Duration::from_secs(1))
    }
}
```

---

## Advanced Topics

### Process Groups

```titan
// Create process group
let group = ProcessGroup::new()?

// Add processes
group.add(process1)?
group.add(process2)?
group.add(process3)?

// Wait for all
group.wait_all()?

// Kill all
group.kill_all()?
```

### Pipes and Redirection

```titan
// Create pipe
let (reader, writer) = Pipe::create()?

// Redirect output
let process = Process::spawn("echo", vec!["test"])?
    .with_stdout(writer)?

// Read from pipe
let output = reader.read_all()?
println!("Output: {}", output)
```

### Environment Management

```titan
// Get environment variable
if let Some(path) = Env::get("PATH") {
    println!("PATH: {}", path)
}

// Set environment variable
Env::set("MY_VAR", "my_value")?

// Get all variables
for (key, value) in Env::all() {
    println!("{}={}", key, value)
}

// Temporary environment
{
    let _guard = Env::temp_set("DEBUG", "1")?
    println!("DEBUG is set")
}  // DEBUG is unset
```

---

## Complete Example: System Monitor

```titan
use omnisystem::system::*
use omnisystem::process::*
use omnisystem::memory::*
use std::time::Duration

fun format_bytes(bytes: u64) -> String {
    match bytes {
        0..=999 => format!("{}B", bytes),
        1000..=999_999 => format!("{:.1}KB", bytes as f64 / 1000.0),
        1_000_000..=999_999_999 => format!("{:.1}MB", bytes as f64 / 1_000_000.0),
        _ => format!("{:.1}GB", bytes as f64 / 1_000_000_000.0),
    }
}

fun monitor_loop() -> Result<(), str> {
    let mut prev_info = Memory::system_info()
    
    loop {
        std::thread::sleep(Duration::from_secs(2))
        
        let info = Memory::system_info()
        let procs = Process::list_all()?
        
        println!("\n╔════════════════════════════════╗");
        println!("║      System Monitor             ║");
        println!("╚════════════════════════════════╝");
        
        println!("\nMemory:");
        println!("  Total:  {}", format_bytes(info.total_bytes));
        println!("  Used:   {} ({:.1}%)", 
            format_bytes(info.used_bytes),
            info.usage_percent
        );
        println!("  Free:   {}", format_bytes(info.free_bytes));
        
        println!("\nTop 5 Processes:");
        let mut sorted = procs
        sorted.sort_by(|a, b| {
            b.memory_bytes().cmp(&a.memory_bytes())
        })
        
        for (i, proc) in sorted.iter().take(5).enumerate() {
            println!("  {}. {} (PID: {}): {}",
                i + 1,
                proc.command(),
                proc.pid(),
                format_bytes(proc.memory_bytes())
            )
        }
        
        println!("\nTotal Processes: {}", procs.len());
    }
}

fun main() -> Result<(), str> {
    println!("Starting system monitor...");
    println!("Press Ctrl+C to exit\n");
    
    monitor_loop()
}
```

---

## Best Practices

✅ **DO**
- Always handle process errors
- Use thread pools for concurrent work
- Monitor memory in loops
- Close file handles explicitly
- Use synchronization primitives correctly
- Handle signals gracefully
- Log system operations

❌ **DON'T**
- Spawn unbounded processes
- Ignore thread panics
- Leave files open
- Busy-wait in loops
- Mix different sync primitives
- Assume path existence
- Hardcode system paths

---

## Performance Tips

1. **Use thread pools** instead of spawning threads
2. **Buffer I/O operations** for better throughput
3. **Monitor memory** to prevent leaks
4. **Use RwLock** for read-heavy workloads
5. **Batch file operations** when possible

---

## Debugging

### System Trace

```titan
// Enable system call tracing
System::enable_tracing()?

// Run operation
let result = expensive_operation()

// Get trace
let trace = System::get_trace()?
for call in trace {
    println!("{}: {}μs", call.name, call.duration_us)
}
```

---

## See Also
- [API_SYSTEMS.md](API_SYSTEMS.md) - Complete API reference
- [TUTORIAL_SYSTEMS.md](TUTORIAL_SYSTEMS.md) - Systems example
- [SYSTEMS_FRAMEWORK_SPECIFICATION.md](SYSTEMS_FRAMEWORK_SPECIFICATION.md) - Formal spec

---

**Next**: [TUTORIAL_SYSTEMS.md](TUTORIAL_SYSTEMS.md) - Build system tools
