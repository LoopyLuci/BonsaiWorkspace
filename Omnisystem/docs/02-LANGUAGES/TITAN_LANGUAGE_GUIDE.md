# TITAN Language Guide - Complete Tutorial

**Dynamic, Type-Safe Systems Programming Language**

---

## Table of Contents
1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Basic Syntax](#basic-syntax)
4. [Type System](#type-system)
5. [Functions](#functions)
6. [Collections](#collections)
7. [Control Flow](#control-flow)
8. [Memory Management](#memory-management)
9. [Error Handling](#error-handling)
10. [Advanced Features](#advanced-features)

---

## Introduction

TITAN is a modern programming language combining:
- **Dynamic typing** for flexibility
- **Static safety** for correctness
- **Memory safety** for reliability
- **Performance** for speed

### Quick Facts
- **Execution Model**: Interpreted with JIT compilation
- **Memory Management**: Automatic (Arc/Mutex)
- **Concurrency**: Native support via threads
- **Type System**: Hybrid static/dynamic

---

## Getting Started

### Your First TITAN Program

```titan
// hello.ti
fun main() {
    let message = "Hello, TITAN!"
    println!(message)
}
```

Run it:
```bash
omnisystem run hello.ti
```

### Interactive REPL

```bash
omnisystem repl
> let x = 42
> println!(x)
42
> x + 8
50
```

---

## Basic Syntax

### Variables

```titan
// Immutable binding (recommended)
let x = 42
let name = "Alice"

// Mutable binding
let mut counter = 0
counter += 1

// Type annotation
let pi: f64 = 3.14159
let numbers: Vec<i64> = [1, 2, 3]
```

### Comments

```titan
// Single line comment

/* Multi-line
   comment */

/// Documentation comment
fun add(a: i64, b: i64) -> i64 {
    a + b
}
```

### Constants

```titan
const PI: f64 = 3.14159
const MAX_SIZE: u32 = 1000
const DEBUG: bool = true
```

---

## Type System

### Primitive Types

```titan
// Integers
let a: i8 = -128
let b: i32 = 42
let c: i64 = 9223372036854775807
let d: u32 = 4294967295

// Floating point
let pi: f32 = 3.14
let e: f64 = 2.71828

// Boolean
let yes: bool = true
let no: bool = false

// Character
let letter: char = 'A'

// String
let greeting: string = "Hello"
let name: str = "World"

// Null
let empty: null = null
```

### Composite Types

```titan
// Array (fixed size)
let coords: [i64; 3] = [10, 20, 30]

// Vector (dynamic)
let numbers: Vec<i64> = [1, 2, 3, 4, 5]

// HashMap
let scores: HashMap<string, i64> = {
    "Alice" => 100,
    "Bob" => 95
}

// Struct
type Point {
    x: f64,
    y: f64
}
```

### Type Conversion

```titan
// Implicit conversion (when safe)
let x: i64 = 42
let y: f64 = x as f64  // Explicit cast

// String conversion
let num = 42
let text = num.to_string()  // "42"
let back = text.parse::<i64>()  // 42
```

---

## Functions

### Basic Functions

```titan
// No parameters, no return
fun greet() {
    println!("Hello!")
}

// Parameters and return
fun add(a: i64, b: i64) -> i64 {
    a + b
}

// Multiple returns
fun divide(x: i64, y: i64) -> (i64, i64) {
    (x / y, x % y)
}

// Default parameters
fun power(base: i64, exp: i64 = 2) -> i64 {
    base ^ exp
}
```

### Calling Functions

```titan
greet()
let sum = add(3, 4)
let (quotient, remainder) = divide(10, 3)
let squared = power(5)  // Uses default exp=2
```

### Closures

```titan
// Closure with captured variables
let x = 10
let add_x = |a| { a + x }
let result = add_x(5)  // 15

// Closure as parameter
fun apply<T>(value: T, f: |T| -> T) -> T {
    f(value)
}

let doubled = apply(5, |n| { n * 2 })
```

---

## Collections

### Working with Vectors

```titan
// Creation
let mut numbers = Vec::new()
let numbers = vec![1, 2, 3, 4, 5]
let zeros = vec![0; 10]  // 10 zeros

// Operations
numbers.push(6)
let last = numbers.pop()
let len = numbers.len()

// Iteration
for num in numbers {
    println!("{}", num)
}

// Functional operations
let doubled = numbers.map(|n| { n * 2 })
let evens = numbers.filter(|n| { n % 2 == 0 })
```

### Working with HashMaps

```titan
// Creation
let mut map = HashMap::new()
map.insert("key", "value")

// Literal syntax
let scores = {
    "Alice" => 100,
    "Bob" => 95,
    "Charlie" => 87
}

// Access
let score = scores.get("Alice")  // Some(100)
scores["Bob"]  // 95 (panics if not found)

// Iteration
for (name, score) in scores {
    println!("{}: {}", name, score)
}
```

---

## Control Flow

### If/Else

```titan
let x = 42
if x > 50 {
    println!("Large")
} else if x > 25 {
    println!("Medium")
} else {
    println!("Small")
}

// Expression form
let category = if x > 50 { "large" } else { "small" }
```

### Loops

```titan
// While loop
let mut count = 0
while count < 5 {
    println!("{}", count)
    count += 1
}

// For loop
for i in 0..5 {
    println!("{}", i)  // 0, 1, 2, 3, 4
}

// Loop (infinite)
let mut x = 0
loop {
    println!("{}", x)
    x += 1
    if x >= 5 {
        break
    }
}

// For-in loop
for item in collection {
    println!("{}", item)
}
```

### Match/Case

```titan
let value = 2
match value {
    1 => println!("one"),
    2 => println!("two"),
    3 => println!("three"),
    _ => println!("other")
}

// Pattern matching with binding
match result {
    Ok(value) => println!("Success: {}", value),
    Err(error) => println!("Error: {}", error)
}
```

---

## Memory Management

### Ownership & References

```titan
// Ownership (automatic Arc)
let s = "Hello"
let t = s  // Both own the same data (shared)

// Explicit borrowing
let s = "Hello"
let r = &s  // Immutable reference

// Mutable reference
let mut s = "Hello"
let r = &mut s  // Mutable reference
*r = "Hi"

// Dereferencing
let x = 5
let r = &x
println!("{}", *r)  // 5
```

### Smart Pointers

```titan
// Box (owned heap allocation)
let boxed = Box::new(42)
println!("{}", *boxed)

// Arc (shared ownership)
let shared = Arc::new(42)
let clone = shared.clone()  // Atomic increment

// Mutex (thread-safe mutable)
let protected = Mutex::new(42)
let mut guard = protected.lock()
*guard += 1
```

---

## Error Handling

### Result Type

```titan
fun divide(a: i64, b: i64) -> Result<i64, string> {
    if b == 0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}

// Using Result
match divide(10, 2) {
    Ok(result) => println!("Result: {}", result),
    Err(err) => println!("Error: {}", err)
}

// Unwrap (panics on error)
let result = divide(10, 2).unwrap()

// Unwrap with default
let result = divide(10, 0).unwrap_or(0)
```

### Option Type

```titan
fun first(vec: Vec<i64>) -> Option<i64> {
    if vec.len() > 0 {
        Some(vec[0])
    } else {
        None
    }
}

// Using Option
match first(numbers) {
    Some(val) => println!("First: {}", val),
    None => println!("Empty")
}

// Shorthand
if let Some(val) = first(numbers) {
    println!("Got: {}", val)
}
```

---

## Advanced Features

### Generics

```titan
fun first<T>(vec: Vec<T>) -> Option<T> {
    if vec.len() > 0 {
        Some(vec[0])
    } else {
        None
    }
}

// Generic struct
type Pair<T> {
    first: T,
    second: T
}

// Generic with constraints
fun max<T: Comparable>(a: T, b: T) -> T {
    if a > b { a } else { b }
}
```

### Traits (Interfaces)

```titan
trait Drawable {
    fun draw()
    fun get_size() -> i64
}

type Circle {
    radius: f64
}

impl Drawable for Circle {
    fun draw() {
        println!("Drawing circle")
    }
    
    fun get_size() -> i64 {
        (radius * radius) as i64
    }
}
```

### Modules & Imports

```titan
// module.ti
pub fun helper() -> string {
    "Hello from module"
}

// main.ti
use module

fun main() {
    let msg = module::helper()
    println!("{}", msg)
}
```

---

## Best Practices

✅ **DO**
- Use `let` by default (immutability)
- Pattern match on `Result` and `Option`
- Use type annotations for clarity
- Keep functions small and focused
- Document public functions

❌ **DON'T**
- Overuse mutable bindings
- Ignore `Result` and `Option` types
- Create deeply nested conditions
- Make functions do multiple things
- Leave errors unhandled

---

## Common Patterns

### Null Coalescing
```titan
let value = maybe_null ?? default_value
```

### Builder Pattern
```titan
type ConfigBuilder {
    name: string,
    debug: bool
}

impl ConfigBuilder {
    fun with_name(name: string) -> Self {
        ConfigBuilder { name, debug: false }
    }
    
    fun with_debug(debug: bool) -> Self {
        ConfigBuilder { ..self, debug }
    }
}
```

### Iterator Chaining
```titan
let result = numbers
    .filter(|n| { n > 0 })
    .map(|n| { n * 2 })
    .take(5)
    .collect()
```

---

## Performance Tips

1. **Use references** to avoid copying
2. **Leverage Arc** for shared data
3. **Use type hints** to avoid inference overhead
4. **Batch operations** on collections
5. **Profile before optimizing**

---

## Debugging

### Print Debugging
```titan
println!("Variable: {}", var)
println!("Debug: {:?}", struct_instance)
```

### Assertions
```titan
assert!(x > 0, "x must be positive")
assert_eq!(a, b)
```

### Error Tracing
```titan
match result {
    Err(e) => eprintln!("Error: {}", e),
    Ok(v) => v
}
```

---

## See Also
- [API_TITAN.md](API_TITAN.md) - Complete API reference
- [TYPE_SYSTEM.md](TYPE_SYSTEM.md) - Deep dive on type system
- [TITAN_LANGUAGE_SPECIFICATION.md](TITAN_LANGUAGE_SPECIFICATION.md) - Formal specification

---

**Next**: [Web Framework Guide](WEB_FRAMEWORK_GUIDE.md) or [TUTORIAL_WEB_APP.md](TUTORIAL_WEB_APP.md)
