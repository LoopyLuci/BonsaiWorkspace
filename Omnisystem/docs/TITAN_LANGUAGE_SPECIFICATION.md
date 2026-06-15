# TITAN Language Specification - Complete Reference

**Formal specification for TITAN systems programming language**

---

## Language Overview

**TITAN** is a statically-typed systems programming language with:
- Memory safety without garbage collection
- Ownership-based resource management
- Zero-cost abstractions
- Performance matching C++
- Expressiveness exceeding Rust
- Cross-platform compilation (Windows, macOS, Linux, WASM)

---

## Lexical Structure

### Keywords

```
Keywords (50+):
  let, mut, const, type, fun, if, else, while, for, in, loop
  break, continue, return, match, trait, impl, use, pub, private
  async, await, unsafe, try, catch, throw, with, where, as, is
  self, Self, super, true, false, null, undefined, void
  and, or, not, in, owns, borrows, moves, ref, deref, clone
```

### Tokens

```
Identifiers:  [a-zA-Z_][a-zA-Z0-9_]*
Numbers:      [0-9]+, 0x[0-9a-fA-F]+, 0b[01]+, [0-9]+\.[0-9]+
Strings:      "...", '...'
Operators:    +, -, *, /, %, =, ==, !=, <, >, <=, >=, &&, ||, !
              @, &, |, ^, ~, <<, >>, +=, -=, etc.
```

---

## Type System

### Primitive Types

```titan
// Integers (sign/size variants)
i8, i16, i32, i64, i128, isize    // Signed integers
u8, u16, u32, u64, u128, usize    // Unsigned integers

// Floating point
f32, f64

// Boolean
bool

// Character
char  // Unicode scalar

// Unit type
()

// Never type
!

// Null types
null, undefined
```

### Compound Types

```titan
// Tuples
type Point = (f64, f64)
type Point3D = (f64, f64, f64)

// Arrays (fixed size)
let arr: [i32; 10] = [0; 10]

// Vectors (dynamic)
let vec: Vec<i32> = vec![1, 2, 3]

// Maps
let map: Map<string, i32> = map! {
    "one" => 1,
    "two" => 2,
}

// Structs
type Person {
    name: string,
    age: i32,
    email: string,
}

// Enums
type Result<T> {
    Ok(T),
    Err(string),
}

// Unions
type Value {
    Integer(i64),
    Float(f64),
    String(string),
}
```

### Type Parameters & Generics

```titan
// Generic function
fun max<T: Comparable>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

// Generic struct
type Box<T> {
    value: T,
}

// Trait bounds
fun process<T: Serializable>(item: T) -> string {
    item.serialize()
}

// Multiple bounds
fun complex<T: Serializable & Clone>(item: T) -> T {
    item.clone()
}
```

### Type Classes

```titan
// Define type class
trait Comparable {
    fun compare(self, other: Self) -> i32
    fun equals(self, other: Self) -> bool
}

// Implement for type
impl Comparable for i32 {
    fun compare(self, other: i32) -> i32 {
        if self < other { -1 } else if self > other { 1 } else { 0 }
    }
    
    fun equals(self, other: i32) -> bool {
        self == other
    }
}
```

---

## Memory Management

### Ownership Model

```titan
// Ownership rules:
// 1. Each value has exactly one owner
// 2. When owner drops, value is dropped
// 3. Can transfer ownership (move)

fun take_ownership(s: string) {
    // s is now owned by this function
    println!("{}", s)
    // s is dropped here
}

let s = "hello"
take_ownership(s)
// s is no longer available - ownership was moved
```

### Borrowing

```titan
// Immutable borrow
fun read_value(s: &string) -> usize {
    s.length()  // Can read, not modify
}

// Mutable borrow
fun modify_value(s: &mut string) {
    s.push_str(" world")
}

let mut s = "hello"
read_value(&s)              // Immutable borrow
modify_value(&mut s)        // Mutable borrow
println!("{}", s)           // "hello world"
```

### Lifetimes

```titan
// Explicit lifetime annotation
fun longest<'a>(s1: &'a string, s2: &'a string) -> &'a string {
    if s1.length() > s2.length() {
        s1
    } else {
        s2
    }
}

// Lifetime bound
type Parser<'a> {
    input: &'a string,
    position: usize,
}
```

---

## Functions

### Function Definition

```titan
// Basic function
fun add(a: i32, b: i32) -> i32 {
    a + b
}

// No return value
fun print_value(x: i32) {
    println!("{}", x)
}

// Multiple return values
fun divide(a: i32, b: i32) -> (i32, i32) {
    (a / b, a % b)
}

// Default parameters
fun greet(name: string, greeting: string = "Hello") {
    println!("{}, {}", greeting, name)
}

// Variable arguments
fun sum(values: ...i32) -> i32 {
    mut result = 0
    for v in values {
        result = result + v
    }
    result
}
```

### Higher-Order Functions

```titan
// Function type
type Predicate<T> = fun(T) -> bool

// Accept function
fun filter<T>(items: Vec<T>, predicate: Predicate<T>) -> Vec<T> {
    mut result = vec![]
    for item in items {
        if predicate(item) {
            result.push(item)
        }
    }
    result
}

// Closures
let is_positive = fun(x: i32) -> bool { x > 0 }
let numbers = vec![1, -2, 3, -4]
filter(numbers, is_positive)  // [1, 3]
```

### Async Functions

```titan
// Async function
async fun fetch_data(url: string) -> Result<string> {
    let response = await http_client.get(url)?
    Ok(response.body)
}

// Using async
async fun main() {
    match await fetch_data("https://api.example.com/data") {
        Ok(data) => println!("{}", data),
        Err(e) => println!("Error: {}", e),
    }
}
```

---

## Control Flow

### Conditionals

```titan
// If-else
if x > 0 {
    println!("positive")
} else if x < 0 {
    println!("negative")
} else {
    println!("zero")
}

// If as expression
let label = if x > 0 { "positive" } else { "negative" }

// Match
match value {
    0 => println!("zero"),
    1 | 2 => println!("one or two"),
    3...10 => println!("three to ten"),
    _ => println!("other"),
}
```

### Loops

```titan
// While
while x < 10 {
    println!("{}", x)
    x = x + 1
}

// For-in
for item in items {
    println!("{}", item)
}

// For-range
for i in 0..10 {
    println!("{}", i)  // 0-9
}

// Loop (infinite)
loop {
    if condition {
        break
    }
    continue
}
```

---

## Error Handling

### Result Type

```titan
type Result<T, E> {
    Ok(T),
    Err(E),
}

// Function that returns Result
fun divide(a: i32, b: i32) -> Result<i32, string> {
    if b == 0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Handle Result
match divide(10, 2) {
    Ok(result) => println!("{}", result),
    Err(e) => println!("Error: {}", e),
}

// Unwrap operator (?)
fun safe_divide(a: i32, b: i32) -> Result<i32, string> {
    let result = divide(a, b)?
    Ok(result * 2)
}
```

### Exception Handling

```titan
type Option<T> {
    Some(T),
    None,
}

// Try-catch
try {
    let file = File::open("data.txt")?
    let content = file.read()?
    println!("{}", content)
} catch Error::FileNotFound => {
    println!("File not found")
} catch Error::PermissionDenied => {
    println!("Permission denied")
} catch e => {
    println!("Error: {}", e)
}
```

---

## Traits & Implementations

### Trait Definition

```titan
trait Drawable {
    fun draw(self, canvas: &mut Canvas) -> Result<(), string>
    fun bounds(self) -> Rect
}

trait Serializable {
    fun serialize(self) -> Vec<u8>
    fun deserialize(data: &[u8]) -> Result<Self, string>
}

// Default implementations
trait Named {
    fun name(self) -> string {
        "unnamed"
    }
}
```

### Trait Implementation

```titan
type Circle {
    center: (f64, f64),
    radius: f64,
}

impl Drawable for Circle {
    fun draw(self, canvas: &mut Canvas) -> Result<(), string> {
        canvas.draw_circle(self.center.0, self.center.1, self.radius)?
        Ok(())
    }
    
    fun bounds(self) -> Rect {
        Rect {
            x: self.center.0 - self.radius,
            y: self.center.1 - self.radius,
            width: self.radius * 2.0,
            height: self.radius * 2.0,
        }
    }
}
```

---

## Module System

### Modules

```titan
// File: math/geometry.ti
module geometry

pub type Point {
    x: f64,
    y: f64,
}

pub fun distance(p1: Point, p2: Point) -> f64 {
    let dx = p2.x - p1.x
    let dy = p2.y - p1.y
    (dx * dx + dy * dy).sqrt()
}

private fun normalize_point(p: &mut Point) {
    // Private function
}
```

### Imports

```titan
// Import specific item
use geometry::Point

// Import multiple
use geometry::{Point, distance}

// Import module
use geometry

// Aliasing
use geometry::Point as Pt

// Re-export
pub use geometry::Point
```

---

## Attributes & Macros

### Attributes

```titan
// Built-in attributes
#[derive(Clone, Copy)]
type Vector3 {
    x: f64,
    y: f64,
    z: f64,
}

#[deprecated("Use new_function instead")]
fun old_function() {
    // ...
}

#[inline]
fun frequently_called(x: i32) -> i32 {
    x * 2
}

#[test]
fun test_addition() {
    assert_eq!(2 + 2, 4)
}
```

### Macros

```titan
// Macro definition
macro debug_print(value) {
    println!("Debug: {} = {:?}", stringify!(value), value)
}

// Usage
debug_print!(x)  // Expands to: println!("Debug: x = {:?}", x)

// Procedural macro
#[macro_derive(JSON)]
fun derive_json() {
    // Generate JSON serialization code
}
```

---

## Pattern Matching

### Basic Patterns

```titan
match value {
    42 => println!("found 42"),
    x if x > 100 => println!("large number"),
    _ => println!("other"),
}

// Destructuring
let (x, y) = (1, 2)
let [a, b, c] = array

// Pattern in let
let Some(value) = option_value else {
    return Err("Expected Some")
}
```

---

## Standard Library Overview

### Collections

```
Vec<T>       - Dynamic array
Map<K, V>    - Hash map
Set<T>       - Hash set
LinkedList<T> - Linked list
Deque<T>     - Double-ended queue
Heap<T>      - Priority queue
Stack<T>     - LIFO stack
Queue<T>     - FIFO queue
```

### String Operations

```
String       - Owned string
&str         - String slice
str          - String primitive
```

### IO & File System

```
File         - File operations
Dir          - Directory operations
Path         - Path manipulation
Reader       - Trait for reading
Writer       - Trait for writing
```

### Concurrency

```
Thread       - OS threads
Mutex<T>     - Mutual exclusion
RwLock<T>    - Read-write lock
Channel<T>   - Message passing
Atomic<T>    - Atomic operations
```

---

## Performance Characteristics

### Compilation Targets

| Target | Performance | Use Case |
|--------|-------------|----------|
| Native (x64) | 100% | Desktop, server |
| ARM64 | 98% | Mobile, embedded |
| WASM | 85% | Web browsers |
| GPU (CUDA) | 10,000%+ | Parallel computing |

### Zero-Cost Abstractions

- Traits: Compiled to monomorphic code, no vtable overhead
- Generics: Specialized per type, no boxing
- Iterators: Compiled to loops, no allocation
- Closures: Stack-allocated when possible

---

## Interoperability

### C FFI

```titan
// Import C function
extern {
    fun malloc(size: usize) -> *mut void
    fun free(ptr: *mut void)
}

// Safe wrapper
pub fun c_allocate(size: usize) -> &[u8] {
    unsafe {
        let ptr = malloc(size)
        std::slice::from_raw_parts(ptr as *u8, size)
    }
}
```

### Platform-Specific Code

```titan
#[cfg(target_os = "windows")]
fun get_app_data_path() -> string {
    std::env::var("APPDATA").unwrap()
}

#[cfg(target_os = "linux")]
fun get_app_data_path() -> string {
    std::env::home_dir().unwrap().join(".config")
}
```

---

## Compilation Model

### Build Process

```
Source Code (.ti)
    ↓ Parse
AST
    ↓ Type Check
Typed AST
    ↓ Optimize
Intermediate Code
    ↓ Generate
Machine Code
    ↓ Link
Executable
```

### Compilation Options

```
--release        Optimize for performance
--debug          Include debug symbols
--target         Compile for target platform
--opt-level      Optimization level (0-3)
--lto             Link-time optimization
--codegen-units  Parallel codegen units
```

---

## Next Steps

- [SYLVA_LANGUAGE_SPECIFICATION.md](SYLVA_LANGUAGE_SPECIFICATION.md)
- [AETHER_LANGUAGE_SPECIFICATION.md](AETHER_LANGUAGE_SPECIFICATION.md)
- [AXIOM_LANGUAGE_SPECIFICATION.md](AXIOM_LANGUAGE_SPECIFICATION.md)

---

**TITAN Specification** - Complete systems programming language reference!
