# Omnisystem Standard Library - Complete Reference

**Comprehensive standard library for TITAN, SYLVA, AETHER, and AXIOM**

---

## Library Overview

The Omnisystem Standard Library provides:
- **Core Collections** - Vec, Map, Set, Deque, Heap
- **String Operations** - Manipulation, parsing, formatting
- **File I/O** - Files, directories, paths
- **Concurrency** - Threads, mutexes, channels, atomics
- **Math & Algorithms** - Linear algebra, sorting, searching
- **Time & Date** - System time, duration, scheduling
- **Cryptography** - Hashing, encryption, signatures
- **JSON/OMNI** - Serialization, deserialization
- **Testing** - Unit tests, assertions, benchmarks
- **Logging** - Structured logging, levels, outputs

---

## Core Collections

### Vec<T> - Dynamic Array

```titan
// Creation
let mut v: Vec<i32> = vec![]
let v = Vec::new()
let v = vec![1, 2, 3]
let v = vec![0; 10]  // 10 zeros

// Operations
v.push(4)              // Add element
v.pop()                // Remove last
v.insert(0, 0)         // Insert at index
v.remove(0)            // Remove at index
v.clear()              // Remove all
v.len()                // Length
v.capacity()           // Capacity
v.is_empty()           // Check if empty
v.contains(&5)         // Check if contains

// Iteration
for item in &v {
    println!("{}", item)
}

for (i, item) in v.iter().enumerate() {
    println!("{}: {}", i, item)
}

// Functional
let doubled = v.map(fun(x) { x * 2 }).collect()
let evens = v.filter(fun(x) { x % 2 == 0 }).collect()
let sum = v.fold(0, fun(acc, x) { acc + x })
```

### Map<K, V> - Hash Map

```titan
// Creation
let mut map: Map<string, i32> = map! {}
let map = Map::new()

// Insertion
map.insert("one", 1)
map.insert("two", 2)

// Access
match map.get("one") {
    Some(value) => println!("{}", value),
    None => println!("Not found"),
}

let value = map.get_or_else("three", || 0)

// Iteration
for (key, value) in map.iter() {
    println!("{}: {}", key, value)
}

// Removal
map.remove("one")
map.clear()
```

### Set<T> - Hash Set

```titan
let mut set: Set<string> = set! {}
set.insert("apple")
set.insert("banana")

// Operations
set.contains("apple")      // true
set.remove("apple")
set.len()
set.is_empty()

// Set operations
let intersection = set1.intersection(&set2)
let union = set1.union(&set2)
let difference = set1.difference(&set2)
```

### LinkedList<T> - Doubly Linked List

```titan
let mut list: LinkedList<i32> = LinkedList::new()
list.push_front(1)
list.push_back(2)
list.pop_front()
```

### Deque<T> - Double-Ended Queue

```titan
let mut deque: Deque<i32> = Deque::new()
deque.push_front(1)
deque.push_back(2)
deque.pop_front()
deque.pop_back()
```

---

## String Operations

### String Manipulation

```titan
// Creation
let s = "hello"
let s = String::from("world")
let s = String::new()

// Length & Access
s.len()                    // Byte length
s.chars().count()          // Character count
s.chars().nth(0)           // Get character
s.bytes().nth(0)           // Get byte

// Substring
s.substring(0, 5)
s.chars().take(3).collect::<String>()

// Case conversion
s.to_lowercase()
s.to_uppercase()
s.to_title_case()

// Trimming
s.trim()
s.trim_start()
s.trim_end()

// Splitting
let parts = s.split(" ")
let lines = s.lines()
let chars = s.chars()

// Searching
s.contains("ll")
s.starts_with("he")
s.ends_with("lo")
s.find("ll")
s.rfind("l")

// Replacement
s.replace("l", "L")
s.replacen("l", "L", 1)

// Concatenation
let combined = s1 + &s2
let combined = format!("{} {}", s1, s2)
```

### String Formatting

```titan
// Basic formatting
format!("Hello {}", name)
format!("{} + {} = {}", 1, 2, 3)

// Debug formatting
format!("{:?}", value)

// Hex/binary
format!("0x{:x}", 255)      // 0xff
format!("0b{:b}", 5)        // 0b101

// Padding
format!("{:10}", "hello")   // "hello     " (right-align)
format!("{:<10}", "hello")  // "hello     " (left-align)
format!("{:^10}", "hello")  // "  hello   " (center)

// Precision
format!("{:.2}", 3.14159)   // "3.14"
format!("{:.5}", "hello")   // "hello"
```

---

## File I/O & Paths

### File Operations

```titan
// Open file
let file = File::open("data.txt")?
let file = File::create("output.txt")?

// Reading
let contents = file.read_to_string()?
let line = file.read_line()?
let bytes = file.read(buffer)?

// Writing
file.write_all(b"hello")?
file.write_fmt(format_args!("Value: {}\n", x))?

// Seeking
file.seek(SeekFrom::Start(0))?
file.seek(SeekFrom::Current(10))?
file.seek(SeekFrom::End(-5))?

// Metadata
file.metadata()?.len()
file.metadata()?.modified()?
```

### Directory Operations

```titan
// Iterate directory
for entry in Dir::read_dir(".")? {
    let entry = entry?
    let path = entry.path()
    let filename = entry.file_name()
}

// Create directory
Dir::create("new_dir")?
Dir::create_all("a/b/c")?

// Remove directory
Dir::remove("empty_dir")?
Dir::remove_all("tree")?
```

### Path Operations

```titan
// Path manipulation
let path = Path::new("data/file.txt")
let parent = path.parent()
let filename = path.file_name()
let extension = path.extension()
let stem = path.file_stem()

// Path building
let path = PathBuf::from("data")
path.push("subdir")
path.push("file.txt")

// Path joining
let path = Path::new("data").join("file.txt")

// Canonicalize
let absolute = path.canonicalize()?
```

---

## Concurrency

### Threads

```titan
// Spawn thread
let handle = std::thread::spawn(fun || {
    println!("Running in thread")
})

// Join thread
handle.join()?

// Named threads
let handle = std::thread::Builder::new()
    .name("worker".to_string())
    .spawn(fun || {
        // Thread logic
    })?

// Thread local storage
thread_local! {
    static BUFFER: Vec<u8> = Vec::new()
}

BUFFER.with(fun |buf| {
    buf.push(42)
})
```

### Synchronization Primitives

```titan
// Mutex
let mutex = Mutex::new(0)

{
    let mut value = mutex.lock()?
    *value = 42
}  // Lock released here

// RwLock
let lock = RwLock::new(vec![])

{
    let readable = lock.read()?
    println!("{:?}", *readable)
}

{
    let mut writable = lock.write()?
    writable.push(1)
}

// Atomic
let atomic = Atomic::new(0)
atomic.store(42, Ordering::SeqCst)
let value = atomic.load(Ordering::SeqCst)
atomic.fetch_add(1, Ordering::SeqCst)
```

### Channels

```titan
// Create channel
let (sender, receiver) = Channel::new()

// Send message
sender.send(42)?
sender.send(Message::Data(vec![1, 2, 3]))?

// Receive message
match receiver.recv() {
    Ok(msg) => println!("{:?}", msg),
    Err(_) => println!("Channel closed"),
}

// Non-blocking receive
match receiver.try_recv() {
    Ok(msg) => println!("{:?}", msg),
    Err(TryRecvError::Empty) => println!("No message yet"),
    Err(TryRecvError::Disconnected) => println!("Channel closed"),
}
```

---

## Math & Algorithms

### Vector Math

```titan
// Vector operations
let v1 = vec3(1.0, 2.0, 3.0)
let v2 = vec3(4.0, 5.0, 6.0)

let sum = v1 + v2
let diff = v1 - v2
let scaled = v1 * 2.0

// Dot product
let dot = v1.dot(v2)

// Cross product
let cross = v1.cross(v2)

// Length
let len = v1.length()
let squared = v1.length_squared()

// Normalization
let normalized = v1.normalize()

// Distance
let distance = v1.distance_to(v2)
```

### Matrix Operations

```titan
// Matrix creation
let m = Matrix4::identity()
let m = Matrix4::translation(vec3(1.0, 2.0, 3.0))
let m = Matrix4::rotation(angle, axis)
let m = Matrix4::scale(vec3(2.0, 2.0, 2.0))

// Matrix operations
let result = m1 * m2
let result = m * v
let determinant = m.determinant()
let inverse = m.inverse()?
let transposed = m.transpose()
```

### Sorting & Searching

```titan
// Sorting
vec.sort()
vec.sort_by(fun(a, b) { a.cmp(b) })
vec.sort_by_key(fun(x) { x.value })

// Searching
vec.binary_search(&5)
vec.contains(&5)
vec.position(fun(x) { x > 10 })

// Unique
let unique = vec.unique()
```

---

## Time & Date

### System Time

```titan
// Current time
let now = SystemTime::now()

// Duration
let duration = Duration::from_secs(60)
let duration = Duration::from_millis(5000)
let duration = Duration::from_nanos(1000)

// Elapsed
let elapsed = now.elapsed()?
println!("{}ms", elapsed.as_millis())

// Sleep
std::thread::sleep(Duration::from_secs(1))
```

---

## Cryptography

### Hashing

```titan
// SHA256
let hash = sha256("hello")?
let hex = hash.to_hex()

// Blake3
let hash = blake3("data")?

// HMAC
let hmac = hmac_sha256(key, data)?
```

### Encryption

```titan
// AES-256
let cipher = AES256::new(key)?
let ciphertext = cipher.encrypt(plaintext, nonce)?
let plaintext = cipher.decrypt(&ciphertext, nonce)?

// ChaCha20
let cipher = ChaCha20::new(key)?
let ciphertext = cipher.encrypt(plaintext, nonce)?
```

### Signatures

```titan
// Generate keypair
let (public, private) = generate_keypair()?

// Sign
let signature = private.sign(message)?

// Verify
public.verify(message, &signature)?
```

---

## Testing Framework

### Unit Tests

```titan
#[test]
fun test_addition() {
    assert_eq!(2 + 2, 4)
}

#[test]
fun test_string_contains() {
    let s = "hello world"
    assert!(s.contains("world"))
}

#[test]
#[should_panic]
fun test_panic() {
    panic!("expected panic")
}

#[test]
#[ignore]
fun test_slow_operation() {
    // Slow test, only run when explicitly requested
}
```

### Assertions

```titan
assert!(condition)
assert_eq!(a, b)
assert_ne!(a, b)
debug_assert!(condition)
debug_assert_eq!(a, b)
```

### Test Macros

```titan
#[test_case(1, 2, 3)]  // Parameterized test
#[test_case(0, 0, 0)]
fun test_add(a: i32, b: i32, expected: i32) {
    assert_eq!(a + b, expected)
}
```

---

## Logging

### Structured Logging

```titan
// Initialize logger
Logger::init()
    .with_level(LogLevel::Info)
    .with_output(LogOutput::File("app.log"))

// Log macros
debug!("Debug message")
info!("Info: {}", value)
warn!("Warning: operation took {}ms", elapsed)
error!("Error: {}", error)

// Structured logging
info!("User login", {
    "user_id": user.id,
    "timestamp": now(),
    "ip": client_ip,
})
```

---

## Feature Flags

### Conditional Compilation

```titan
#[cfg(feature = "networking")]
mod networking {
    // Networking code
}

#[cfg(all(target_os = "windows", feature = "gui"))]
fun windows_gui_setup() {
    // Windows GUI setup
}

#[cfg(not(debug_assertions))]
fun production_only() {
    // Release-only code
}
```

---

## Library Statistics

| Category | Functions | Modules | Lines |
|----------|-----------|---------|-------|
| Collections | 150+ | core | 5,000+ |
| String | 100+ | string | 3,000+ |
| IO | 80+ | io | 2,500+ |
| Concurrency | 60+ | thread | 2,000+ |
| Math | 200+ | math | 4,000+ |
| Crypto | 40+ | crypto | 1,500+ |
| Time | 30+ | time | 1,000+ |
| Testing | 50+ | test | 2,000+ |
| **TOTAL** | **710+** | **8** | **21,000+** |

---

## Import Examples

```titan
// Import everything from module
use std::collections::*

// Import specific items
use std::fs::{File, Dir}
use std::io::Result

// Import with alias
use std::collections::HashMap as Map

// Re-export
pub use std::vec::Vec
```

---

## Next Steps

- [BUILD_SYSTEM_GUIDE.md](BUILD_SYSTEM_GUIDE.md) - Build and compilation
- [PACKAGE_MANAGER.md](PACKAGE_MANAGER.md) - Dependency management
- [IDE_INTEGRATION.md](IDE_INTEGRATION.md) - Editor setup

---

**Standard Library** - Comprehensive tools for all Omnisystem languages!
