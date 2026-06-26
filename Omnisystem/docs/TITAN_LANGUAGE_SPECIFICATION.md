# TITAN Language Specification v1.0
## The Omnisystem Systems Programming Language

---

## 1. OVERVIEW

**TITAN** is a next-generation systems programming language designed to replace C, C++, Rust, and Go. It combines:
- Zero-cost abstractions
- Memory safety without garbage collection
- Compile-time verification
- Native performance
- Elegant syntax
- 100-year stability guarantee

### Design Principles
1. **Zero Runtime Overhead** - All safety checks at compile time
2. **Explicit is Better** - No hidden allocations or type conversions
3. **Fail Fast** - Errors caught at compile time, never at runtime
4. **Composability** - Small, orthogonal features combine elegantly
5. **Predictability** - Performance characteristics always obvious

---

## 2. SYNTAX

### 2.1 Basic Structure

```titan
// Single-line comment
/* Multi-line
   comment */

// Module declaration
mod kernel::memory

// Import statement
use std::collections::{Map, Set}
use crypto::{sha256, aes256}

// Function declaration
fn add(x: i64, y: i64) -> i64 {
    return x + y
}

// Constants
const PI: f64 = 3.14159265359
const MAX_BUFFER: usize = 1024 * 1024

// Mutable variable
let mut counter: i32 = 0
counter = counter + 1

// Immutable variable
let message: string = "Hello, TITAN"

// Block expression (returns value)
let result: i32 = {
    let x = 5
    let y = 10
    x + y  // No semicolon = return value
}
```

### 2.2 Type System

```titan
// Primitive types
bool              // true, false
i8, i16, i32, i64, i128  // Signed integers
u8, u16, u32, u64, u128  // Unsigned integers
f32, f64          // Floating point
usize, isize      // Platform-dependent sizes
string            // UTF-8 string (immutable)
bytes             // Byte array (immutable)

// Composite types
struct Point {
    x: f64
    y: f64
    z: f64
}

tuple (i32, string, bool)

enum Color {
    Red,
    Green,
    Blue,
    Custom(u8, u8, u8)
}

// Type aliases
type UserId = u64
type Timestamp = i64
```

### 2.3 Memory Model

```titan
// Ownership - exactly ONE owner
let mut buffer: [u8] = allocate(1024)  // buffer owns 1024 bytes
// buffer is automatically freed at end of scope (RAII)

// Borrowing - temporary access
fn process(data: &[u8]) -> u32 {  // Immutable borrow
    return data.len()
}

fn mutate(data: &mut [u8]) {  // Mutable borrow
    data[0] = 42
}

let mut my_data: [u8] = [1, 2, 3, 4, 5]
process(my_data)          // Immutable borrow
mutate(my_data)           // Mutable borrow

// Lifetime tracking (explicit)
fn longest<'a>(a: &'a string, b: &'a string) -> &'a string {
    if a.len() > b.len() { return a }
    return b
}
```

### 2.4 Functions

```titan
// Simple function
fn greet(name: string) -> string {
    return "Hello, " + name
}

// Function with multiple return values
fn divide(a: i64, b: i64) -> (i64, i64, bool) {
    if b == 0 {
        return (0, 0, false)
    }
    return (a / b, a % b, true)
}

// Using result
let (quotient, remainder, success) = divide(17, 5)

// Function that returns Result<T, E>
fn safe_divide(a: i64, b: i64) -> Result<i64, string> {
    if b == 0 {
        return Error("Division by zero")
    }
    return Ok(a / b)
}

// Error handling with try (?)
fn compute() -> Result<i64, string> {
    let x = safe_divide(10, 2)?  // Propagate error if present
    let y = safe_divide(20, 4)?
    return Ok(x + y)
}

// Generic functions
fn max<T: Comparable>(a: T, b: T) -> T {
    if a > b { return a }
    return b
}
```

### 2.5 Control Flow

```titan
// If expression (returns value)
let status: string = if x > 0 {
    "positive"
} else if x < 0 {
    "negative"
} else {
    "zero"
}

// While loop
let mut i: i32 = 0
while i < 10 {
    print(i)
    i = i + 1
}

// For loop with range
for i in 0..10 {
    print(i)
}

// For loop with iterator
for item in collection {
    process(item)
}

// Match (exhaustive pattern matching)
match color {
    Color::Red => print("Red"),
    Color::Green => print("Green"),
    Color::Blue => print("Blue"),
    Color::Custom(r, g, b) => print("RGB({}, {}, {})", r, g, b),
}

// Loop (infinite loop)
loop {
    if should_break { break }
    continue
}
```

### 2.6 Data Structures

```titan
// Struct
struct Person {
    name: string
    age: u32
    email: string
}

// Create instance
let alice: Person = Person {
    name: "Alice",
    age: 30,
    email: "alice@example.com"
}

// Access fields
let name: string = alice.name

// Update (immutable update)
let alice2: Person = alice {
    age: 31
}

// Enum (tagged union)
enum Result<T, E> {
    Ok(T),
    Error(E)
}

// Array (fixed size, stack allocated)
let nums: [i32; 5] = [1, 2, 3, 4, 5]

// Slice (dynamically sized, reference to array)
let slice: &[i32] = nums[1..4]

// Vector (dynamic array, heap allocated)
let mut vec: Vec<i32> = Vec::new()
vec.push(1)
vec.push(2)
vec.push(3)

// Map (hash table)
let mut scores: Map<string, i32> = Map::new()
scores["Alice"] = 95
scores["Bob"] = 87
```

### 2.7 Object-Oriented Features

```titan
// Traits (interfaces)
trait Drawable {
    fn draw() -> void
    fn get_bounds() -> Rectangle
}

// Struct implementing trait
struct Circle {
    x: f64
    y: f64
    radius: f64
}

impl Drawable for Circle {
    fn draw() -> void {
        // Draw circle
    }
    
    fn get_bounds() -> Rectangle {
        return Rectangle {
            x: x - radius,
            y: y - radius,
            width: radius * 2,
            height: radius * 2
        }
    }
}

// Method syntax
impl Circle {
    fn area() -> f64 {
        return 3.14159 * radius * radius
    }
    
    fn translate(dx: f64, dy: f64) -> void {
        x = x + dx
        y = y + dy
    }
}

// Call method
let c: Circle = Circle { x: 0, y: 0, radius: 5 }
let area: f64 = c.area()
c.translate(10, 10)
```

### 2.8 Advanced Features

```titan
// Generics with constraints
fn process<T: Serializable + Copyable>(item: T) -> string {
    return item.serialize()
}

// Higher-order functions
fn apply_twice(f: fn(i32) -> i32, x: i32) -> i32 {
    return f(f(x))
}

let double = fn(x: i32) -> i32 { return x * 2 }
let result: i32 = apply_twice(double, 5)  // Returns 20

// Closures (capture environment)
let multiplier: i32 = 3
let multiply = fn(x: i32) -> i32 { return x * multiplier }

// Compile-time computation
const BUFFER_SIZE: usize = {
    const PAGE_SIZE: usize = 4096
    const PAGES: usize = 256
    return PAGE_SIZE * PAGES
}

// Inline assembly (for systems programming)
fn read_rax() -> u64 {
    return asm {
        mov rax, [rsp]
        ret
    }
}

// Attributes (compiler directives)
@inline
fn small_function() -> i32 {
    return 42
}

@no_mangle
fn ffi_function() -> void {
    // Can be called from C
}

@test
fn test_addition() -> void {
    assert(2 + 2 == 4)
}
```

---

## 3. TYPE SYSTEM

### 3.1 Type Inference

```titan
// Types are inferred when possible
let x = 5                 // i32
let y = 3.14              // f64
let name = "Alice"        // string
let items = [1, 2, 3]     // [i32]

// Explicit types when needed
let x: i64 = 5
let y: f32 = 3.14
```

### 3.2 Trait-Based Polymorphism

```titan
trait Comparable {
    fn compare(other: Self) -> i32
    fn equals(other: Self) -> bool
}

trait Serializable {
    fn serialize() -> string
    fn deserialize(data: string) -> Self
}

trait Hashable {
    fn hash() -> u64
}
```

### 3.3 Type Bounds

```titan
fn max<T: Comparable>(a: T, b: T) -> T {
    if a.compare(b) > 0 { return a }
    return b
}

fn hash_map<K: Hashable + Comparable, V: Clone>(
    keys: [K],
    values: [V]
) -> Map<K, V> {
    // Implementation
}
```

---

## 4. ERROR HANDLING

### 4.1 Result Type

```titan
type Result<T, E> = union {
    Ok(T),
    Error(E)
}

fn safe_parse(input: string) -> Result<i64, string> {
    // Try to parse
    if valid {
        return Ok(parsed_value)
    }
    return Error("Invalid input")
}

// Using Result
let result: Result<i64, string> = safe_parse("42")

match result {
    Ok(value) => print("Parsed: {}", value),
    Error(msg) => print("Error: {}", msg),
}
```

### 4.2 Try Operator (?)

```titan
fn compute() -> Result<i64, string> {
    let x = safe_parse("10")?   // Returns Error if failed
    let y = safe_parse("20")?
    return Ok(x + y)
}
```

### 4.3 Panic (Unrecoverable Errors)

```titan
fn unwrap_unsafe(result: Result<i64, string>) -> i64 {
    match result {
        Ok(value) => return value,
        Error(msg) => panic("Failed: {}", msg),
    }
}

// Assertions
assert(x > 0, "x must be positive")
debug_assert(condition, "Debug check")
```

---

## 5. MEMORY SAFETY

### 5.1 Ownership Rules

1. Each value has exactly one owner
2. Owner can lend the value (borrowing)
3. Value is freed when owner goes out of scope
4. Compiler enforces these rules

```titan
let s: string = "hello"        // s owns the string
let t: string = s              // ERROR: s moved to t, s no longer owns it

let s: string = "hello"
let t: &string = &s            // t borrows s (immutable)
let u: &string = &s            // Multiple immutable borrows OK
// s still owns the string
```

### 5.2 Lifetimes

```titan
// Lifetime 'a means: this reference is valid for duration 'a
fn borrow<'a>(s: &'a string) -> &'a string {
    return s
}

// Multiple lifetimes
fn join<'a, 'b>(a: &'a string, b: &'b string) -> string {
    return a + b
}
```

### 5.3 Stack vs Heap

```titan
// Stack allocation (automatic, fast)
let mut point: Point = Point { x: 0, y: 0, z: 0 }

// Heap allocation (explicit)
let mut buffer: &[u8] = allocate(4096)
// Use buffer
deallocate(buffer)  // Explicit free (or automatic with RAII)

// Smart pointers
let owned: Box<Data> = Box::new(data)  // Unique ownership
let shared: Rc<Data> = Rc::new(data)   // Shared ownership (reference counted)
```

---

## 6. CONCURRENCY & PARALLELISM

### 6.1 Threads

```titan
fn spawn_thread(name: string) -> Thread {
    let t: Thread = thread::spawn(fn() {
        print("Running in thread: {}", name)
    })
    return t
}

let t1 = spawn_thread("Worker-1")
let t2 = spawn_thread("Worker-2")

t1.join()  // Wait for thread to finish
t2.join()
```

### 6.2 Channels (Message Passing)

```titan
let (sender, receiver): (Sender<i32>, Receiver<i32>) = channel()

thread::spawn(fn() {
    sender.send(42)
})

let value: i32 = receiver.receive()  // Blocks until message arrives
```

### 6.3 Mutexes & Locks

```titan
let counter: Mutex<i32> = Mutex::new(0)

thread::spawn(fn() {
    let mut guard = counter.lock()
    guard.value = guard.value + 1
    // Automatically unlocked when guard goes out of scope
})
```

---

## 7. STANDARD LIBRARY

### 7.1 String Operations

```titan
fn main() -> void {
    let s: string = "Hello"
    
    print(s.len())                    // 5
    print(s.to_upper())               // "HELLO"
    print(s.to_lower())               // "hello"
    print(s.starts_with("He"))        // true
    print(s.ends_with("lo"))          // true
    print(s.contains("ll"))           // true
    print(s.substring(1, 4))          // "ell"
    print(s.replace("ll", "xx"))      // "Hexxo"
    print(s.split(" "))               // ["Hello"]
    print(s.trim())                   // "Hello" (no leading/trailing whitespace)
}
```

### 7.2 Collections

```titan
// Vector
let mut v: Vec<i32> = Vec::new()
v.push(1)
v.push(2)
v.push(3)
print(v.len())                        // 3
print(v[0])                           // 1
v.pop()                               // Removes 3

// Map
let mut map: Map<string, i32> = Map::new()
map["Alice"] = 95
map["Bob"] = 87
print(map["Alice"])                   // 95
print(map.contains_key("Alice"))      // true
print(map.keys())                     // ["Alice", "Bob"]
print(map.values())                   // [95, 87]

// Set
let mut set: Set<i32> = Set::new()
set.insert(1)
set.insert(2)
set.insert(3)
print(set.contains(2))                // true
print(set.len())                      // 3
```

### 7.3 I/O Operations

```titan
// Console I/O
print("Hello, World!")
println("With newline")
printf("Formatted: {}, {}", x, y)

let input: string = input()           // Read from stdin

// File I/O
let file = File::open("data.txt")
let content: string = file.read_all()
file.close()

let out = File::create("output.txt")
out.write("Hello")
out.write_line("World")
out.close()
```

### 7.4 Math Operations

```titan
print(max(5, 10))                     // 10
print(min(5, 10))                     // 5
print(abs(-42))                       // 42
print(floor(3.7))                     // 3
print(ceil(3.2))                      // 4
print(round(3.5))                     // 4
print(sqrt(16))                       // 4.0
print(pow(2, 8))                      // 256
print(sin(3.14159 / 2))               // ~1.0
print(cos(0))                         // 1.0
```

### 7.5 Time & Dates

```titan
let now: Timestamp = time::now()
let duration: Duration = time::Duration::new(seconds: 60)

let start = time::now()
// ... do work ...
let elapsed = time::now() - start
print("Elapsed: {} ms", elapsed.to_milliseconds())
```

### 7.6 Hashing & Cryptography

```titan
let hash: u64 = hash("hello")
let sha256: [u8] = crypto::sha256("password")
let encrypted: [u8] = crypto::aes256_encrypt(data, key)
let decrypted: [u8] = crypto::aes256_decrypt(encrypted, key)
```

---

## 8. COMPILATION & PERFORMANCE

### 8.1 Compilation Stages

```
Source Code (.titan)
    ↓
Lexer (Tokenization)
    ↓
Parser (AST Construction)
    ↓
Type Checker (Type Inference & Checking)
    ↓
Optimizer (Constant Folding, Inlining, etc.)
    ↓
Code Generator (LLVM IR / Native Code)
    ↓
Linker (Final Executable)
    ↓
Native Binary
```

### 8.2 Compile-Time Guarantees

- **Memory Safety**: No buffer overflows, use-after-free, null pointers
- **Thread Safety**: Data races detected at compile time
- **Type Safety**: No invalid type conversions
- **Bounds Checking**: Array access bounds verified
- **Null Safety**: Nulls explicitly represented in type system

### 8.3 Runtime Performance

- **Zero-Cost Abstractions**: High-level features compile to same code as low-level
- **No Garbage Collection**: RAII (Resource Acquisition Is Initialization) for automatic cleanup
- **Inlining**: Small functions automatically inlined
- **Vectorization**: SIMD operations automatically generated
- **Native Code**: Direct compilation to machine code, no VM

---

## 9. INTEROPERABILITY

### 9.1 C FFI (Foreign Function Interface)

```titan
@extern
fn sqrt(x: f64) -> f64

@extern
fn malloc(size: usize) -> &void

@no_mangle
fn titan_function() -> i32 {
    return 42
}

// Can be called from C:
// extern int titan_function();
```

### 9.2 LLVM IR Backend

```
TITAN Source Code
    ↓
TITAN AST
    ↓
LLVM IR
    ↓
Native Code (x86, ARM, RISC-V, etc.)
```

---

## 10. MODULE SYSTEM

### 10.1 Modules

```titan
// file: kernel/memory.titan
mod kernel::memory

pub struct MemoryPool {
    buffer: &[u8]
    offset: usize
}

pub fn allocate(size: usize) -> &[u8] {
    // Implementation
}

private fn internal_helper() -> void {
    // Not visible outside module
}
```

### 10.2 Imports

```titan
// file: main.titan
use kernel::memory::{MemoryPool, allocate}
use std::{
    collections::{Map, Set},
    io::{print, File}
}

fn main() -> void {
    let buffer = allocate(4096)
}
```

---

## 11. EXAMPLE PROGRAMS

### 11.1 Hello World

```titan
fn main() -> void {
    println("Hello, World!")
}
```

### 11.2 Fibonacci

```titan
fn fibonacci(n: u32) -> u64 {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() -> void {
    for i in 0..10 {
        println("fib({}) = {}", i, fibonacci(i))
    }
}
```

### 11.3 Safe Division

```titan
fn safe_divide(a: i64, b: i64) -> Result<i64, string> {
    if b == 0 {
        return Error("Division by zero")
    }
    return Ok(a / b)
}

fn compute() -> Result<i64, string> {
    let x = safe_divide(10, 2)?
    let y = safe_divide(20, 4)?
    let z = safe_divide(30, 5)?
    return Ok(x + y + z)
}

fn main() -> void {
    match compute() {
        Ok(result) => println("Result: {}", result),
        Error(msg) => println("Error: {}", msg),
    }
}
```

### 11.4 Concurrent Processing

```titan
fn process_item(id: i32, item: i32) -> i32 {
    return item * 2
}

fn main() -> void {
    let items: [i32] = [1, 2, 3, 4, 5]
    let mut threads: [Thread] = []
    
    for i in 0..items.len() {
        let t = thread::spawn(fn() {
            println("Worker {}: {}", i, process_item(i, items[i]))
        })
        threads.push(t)
    }
    
    for t in threads {
        t.join()
    }
}
```

---

## 12. DESIGN PHILOSOPHY

### Principles That Last 100 Years

1. **Simplicity**: Fewer features, well-designed, prevent mistakes
2. **Consistency**: Similar problems solved similarly across language
3. **Explicitness**: No magic, no hidden costs, intent clear
4. **Composability**: Small pieces combine elegantly
5. **Backwards Compatibility**: Code written today works in 100 years
6. **Performance**: Fast enough for any use case
7. **Safety**: Mistakes caught at compile time, not runtime
8. **Maintainability**: Code is readable and understandable
9. **Evolution**: Language can improve without breaking existing code
10. **Timelessness**: Not trendy, fundamentally sound

---

## 13. GRAMMAR (BNF)

```
Program ::= (Import | Declaration)*

Import ::= "use" Path

Declaration ::= 
    | "fn" Identifier Parameters ReturnType Block
    | "struct" Identifier "{" Fields "}"
    | "enum" Identifier "{" Variants "}"
    | "trait" Identifier "{" TraitMethods "}"
    | "impl" TraitName "for" TypeName "{" Methods "}"
    | "const" Identifier ":" Type "=" Expression

Statement ::=
    | "let" Identifier (":" Type)? "=" Expression
    | "let" "mut" Identifier (":" Type)? "=" Expression
    | Expression

Expression ::=
    | Literal
    | Identifier
    | BinaryOp
    | UnaryOp
    | FunctionCall
    | IfExpression
    | MatchExpression
    | LoopExpression
    | BlockExpression

Type ::=
    | "bool" | "i32" | "i64" | "f32" | "f64" | "string" | "bytes"
    | Identifier
    | "[" Type "]"
    | "&" Type
    | "&mut" Type
    | "(" (Type ",")* ")"
    | Type "?" // Optional (Result-like)
```

---

This specification provides the foundation for TITAN. All code compiles to LLVM IR and native binaries with zero runtime overhead.
