# Titan Language Reference

**Titan** is the systems programming language of the Omnisystem. It occupies the lowest level of the four-language stack, handling code where performance, memory layout, and safety guarantees must all be simultaneously provable. Titan compiles to LLVM IR and then to native machine code. Its compiler is written in Titan itself (self-hosted), bootstrapped from a minimal Python seed.

---

## 1. Design Principles

Titan is built on four non-negotiable constraints:

1. **No hidden allocations.** Every heap allocation requires an explicit `alloc` effect declaration. A function without `alloc` in its effect signature cannot allocate, statically.
2. **No undefined behavior.** Arithmetic overflow, null dereference, use-after-free, and data races are compile-time errors, not runtime events. The `unsafe` block is the single escape hatch, and its use is always visible in the source.
3. **No garbage collector.** Memory is managed through ownership and borrowing. When a binding goes out of scope, its memory is freed deterministically — no pause, no collector thread.
4. **All effects declared.** I/O, network access, panic, telemetry, and device operations are declared in function signatures. A function that is not declared with `io` cannot perform I/O. This is enforced by both the compiler and the OmniCore runtime.

---

## 2. Lexical Structure

### 2.1 Comments

```titan
// Single-line comment

/* Multi-line
   comment */
```

### 2.2 Identifiers

Identifiers begin with a letter or underscore and contain letters, digits, and underscores. They are case-sensitive. The following are reserved keywords and may not be used as identifiers:

```
fn       pub      extern   let      mut      return   if       else
while    loop     for      in       break    continue struct   enum
match    mod      use      move     true     false    self     type
where    impl     trait    alloc    emit     await    async    unsafe
```

### 2.3 Literals

| Literal | Examples | Notes |
|---------|----------|-------|
| Integer | `0`, `42`, `0xFF`, `0b1010`, `1_000_000` | Underscores allowed as separators |
| Float | `3.14`, `1.0e-6`, `0.5f32` | `f32` or `f64` suffix |
| Boolean | `true`, `false` | |
| String | `"hello"`, `"line\n"` | UTF-8, `\n \t \r \\ \"` escapes |
| Unit | `()` | The single value of type `()` |

### 2.4 Operators

**Arithmetic:** `+` `-` `*` `/` `%`

**Bitwise:** `&` `|` `^` `~` `<<` `>>`

**Comparison:** `==` `!=` `<` `>` `<=` `>=`

**Logical:** `&&` `||` `!`

**Assignment:** `=` `+=` `-=` `*=` `/=` `%=`

**Reference:** `&` (borrow) `&mut` (mutable borrow) `*` (dereference)

**Path:** `::` (module separator) `.` (field access) `[n]` (index)

**Effect:** `!` (effect declaration, appears after `)` in function signatures)

**Operator precedence** (lowest to highest):

| Level | Operators |
|-------|-----------|
| 1 | `\|\|` |
| 2 | `&&` |
| 3 | `==` `!=` `<` `>` `<=` `>=` |
| 4 | `+` `-` |
| 5 | `*` `/` `%` |
| 6 | unary `!` `-` `&` `*` |
| 7 | `.` `[]` `()` (postfix) |

---

## 3. Type System

Titan has a **static, strong, nominal** type system with optional dependent types for invariants and a graded modality system for resource tracking.

### 3.1 Primitive Types

```titan
i8    i16   i32   i64   i128  isize   // signed integers (8, 16, 32, 64, 128, platform bits)
u8    u16   u32   u64                 // unsigned integers
f32   f64                             // IEEE 754 floats
bool                                  // true or false
str                                   // UTF-8 string slice (immutable, borrowed)
()                                    // unit — the return type of functions that return nothing
```

### 3.2 Compound Types

```titan
// Struct (named product type)
pub struct Point {
    x: f64,
    y: f64,
}

// Enum (tagged union)
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

// Reference types
&T        // immutable borrow of T — read-only, no ownership transfer
&mut T    // mutable borrow of T — read-write, exclusive access

// Array
[T; N]    // fixed-size array of N elements of type T
[T]       // slice — borrowed view into contiguous memory

// Function type
fn(i64, i64) -> i64              // function taking two i64, returning i64
fn(i64) -> i64 ! { alloc }      // same with alloc effect
```

### 3.3 Type Inference

Titan infers types for local bindings. Explicit annotations are required only at function boundaries:

```titan
fn compute() -> i64 {
    let x = 10;          // inferred: i64
    let y = x + 5;       // inferred: i64
    let flag = x > 0;    // inferred: bool
    return y;
}
```

### 3.4 Generics

Functions and structs can be parameterized over types:

```titan
pub struct Vec<T> {
    data: *mut T,
    len: i64,
    cap: i64,
}

pub fn first<T>(slice: &[T]) -> Option<&T> {
    if slice.len() == 0 { return None; }
    Some(&slice[0])
}
```

Type parameters can be constrained with trait bounds using `where`:

```titan
pub fn sort<T>(data: &mut [T]) where T: Ord {
    // ... sorting algorithm
}
```

### 3.5 Graded Modalities

Every value in Titan carries a **grade** — a usage annotation from quantitative type theory:

| Grade | Symbol | Meaning |
|-------|--------|---------|
| `ZERO` | `0` | Erased at runtime. Appears in proofs only; no machine code generated. |
| `ONE` | `1` | Linear. Must be consumed exactly once. Use twice = compile error. |
| `MANY` | `ω` | Unrestricted. Can be used any number of times (default for most values). |

In practice, grades are mostly inferred. You interact with them through ownership (linear types) and the borrow checker.

---

## 4. Effects

Every function that performs a side effect must declare it. Effects appear after the `)` of a function's parameter list:

```titan
pub fn read_file(path: str) -> str ! { io } { ... }
pub fn allocate_buffer(n: i64) -> *u8 ! { alloc } { ... }
pub fn log_event(msg: str) ! { telemetry } { ... }
pub fn risky_op() ! { alloc, io, panic } { ... }
```

### 4.1 Available Effects

| Effect | Keyword | Meaning |
|--------|---------|---------|
| Allocation | `alloc` | Heap allocation (`Box::new`, `Vec::new`, etc.) |
| I/O | `io` | File, stdin/stdout, system calls |
| File read | `read_fs("path")` | Read a specific file path |
| File write | `write_fs("path")` | Write a specific file path |
| Network | `network("*.api.com")` | Network connections matching pattern |
| Panic | `panic` | May call `panic!()` or abort |
| Unsafe | `unsafe` | Contains an unsafe block |
| Telemetry | `telemetry` | Emits structured events to OmniCore |
| Device | `device(gpu)` | Offloads to GPU, FPGA, TPU, or SIMD |

### 4.2 Effect Propagation

Effects are infectious upward. A function that calls an `alloc`-effectful function must itself be declared `! { alloc }`:

```titan
pub fn inner() -> i64 ! { alloc } {
    let v = Vec::new();   // alloc effect
    return 0;
}

pub fn outer() -> i64 ! { alloc } {  // must also declare alloc
    return inner();
}
```

### 4.3 Pure Functions

A function with no `!` clause is **pure** — no effects, no heap allocations, no I/O, guaranteed to terminate (barring infinite loops):

```titan
pub fn add(a: i64, b: i64) -> i64 {
    return a + b;    // pure: no effects
}
```

---

## 5. Variables and Bindings

### 5.1 Immutable Bindings

`let` creates an immutable binding. Reassignment is a compile error:

```titan
let x = 42;
x = 10;          // ERROR: cannot assign to immutable binding 'x'
```

### 5.2 Mutable Bindings

`let mut` allows reassignment:

```titan
let mut counter = 0;
counter = counter + 1;    // OK
```

### 5.3 Type Annotations

Optional when the type can be inferred; required when it cannot:

```titan
let n: i64 = 42;
let s: str = "hello";
let v: Vec<i64> = Vec::new();
```

### 5.4 Shadowing

A new `let` binding in the same scope shadows the previous one:

```titan
let x = 5;
let x = x + 1;    // shadows previous x; x is now 6
```

---

## 6. Functions

### 6.1 Declaration Syntax

```titan
[pub] fn name(param: Type, ...) [-> ReturnType] [! { effects }] {
    body
}
```

- `pub` exports the function from its module
- Parameters always require explicit type annotations
- Return type is required unless the function returns `()` (unit)
- Effects are declared after `)` with `! { ... }`

### 6.2 Examples

```titan
// Simple pure function
pub fn add(a: i64, b: i64) -> i64 {
    return a + b;
}

// Void function (returns unit)
pub fn print_greeting(name: str) ! { io } {
    // body
}

// Multiple effects
pub fn save_to_file(path: str, data: str) ! { io, write_fs("*") } {
    // body
}

// With lifetimes
pub fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

### 6.3 Return Values

The `return` keyword is explicit. The last expression in a block may also serve as the return value (expression-oriented style):

```titan
pub fn double(x: i64) -> i64 {
    x * 2           // implicit return — no semicolon
}

pub fn abs(x: i64) -> i64 {
    return if x < 0 { -x } else { x };
}
```

### 6.4 External Functions

`extern fn` declares a function implemented outside Titan (in C or LLVM IR):

```titan
extern fn malloc(size: i64) -> *u8 ! { alloc, unsafe };
extern fn free(ptr: *u8) ! { unsafe };
```

---

## 7. Control Flow

### 7.1 If / Else

`if` is both a statement and an expression. The condition does not use parentheses:

```titan
// As statement
if x > 0 {
    // positive
} else {
    // non-positive
}

// As expression
let sign = if x > 0 { 1 } else { -1 };

// Chains
if x > 100 {
    "big"
} else if x > 10 {
    "medium"
} else {
    "small"
}
```

**Important:** Struct literal syntax (`Name { ... }`) is suppressed in `if` conditions to avoid ambiguity with the opening brace. Use a binding: `let s = MyStruct { ... }; if s.field > 0 { ... }`.

### 7.2 While Loop

```titan
let mut i = 0;
while i < 10 {
    i = i + 1;
}
```

### 7.3 Infinite Loop

```titan
loop {
    let input = read_line();
    if input == "quit" { break; }
    process(input);
}
```

`loop` can return a value via `break expr`:

```titan
let result = loop {
    let val = try_compute();
    if val > 0 { break val; }
};
```

### 7.4 For Loop

Iterates over any value implementing the `Iterator` trait:

```titan
for i in 0..10 {
    // i goes from 0 to 9
}

for item in &my_vec {
    // item is &T
}
```

### 7.5 Match

Exhaustive pattern matching over enums and primitives:

```titan
match result {
    Ok(value) => {
        return value;
    }
    Err(msg) => {
        panic(msg);
    }
}

match x {
    0       => "zero",
    1..=9   => "single digit",
    _       => "large",
}
```

---

## 8. Structs

### 8.1 Definition

```titan
pub struct Rectangle {
    width: f64,
    height: f64,
}
```

### 8.2 Construction

```titan
let r = Rectangle {
    width: 10.0,
    height: 5.0,
};
```

### 8.3 Field Access

```titan
let area = r.width * r.height;
```

### 8.4 Methods (impl blocks)

```titan
impl Rectangle {
    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn scale(&mut self, factor: f64) {
        self.width = self.width * factor;
        self.height = self.height * factor;
    }

    pub fn new(w: f64, h: f64) -> Rectangle {
        Rectangle { width: w, height: h }
    }
}

// Usage
let mut r = Rectangle::new(10.0, 5.0);
let a = r.area();
r.scale(2.0);
```

---

## 9. Enums

```titan
pub enum Color {
    Red,
    Green,
    Blue,
    Custom(u8, u8, u8),    // tuple variant with data
}

pub enum Option<T> {
    Some(T),
    None,
}

pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

Enums are matched exhaustively:

```titan
match color {
    Color::Red           => "red",
    Color::Green         => "green",
    Color::Blue          => "blue",
    Color::Custom(r,g,b) => "custom",
}
```

---

## 10. Ownership and Borrowing

Titan's memory model is based on **single-owner semantics**: exactly one binding owns each value at any point in time. When the owner goes out of scope, the value is freed.

### 10.1 Move Semantics

Assignment transfers ownership. After a move, the original binding is invalid:

```titan
let s1 = String::new("hello");
let s2 = s1;          // s1 is moved into s2
// s1 is now invalid — any use is a compile error
```

### 10.2 Copy Types

Primitive types (`i64`, `f64`, `bool`, `()`, references, and any type implementing `Copy`) are duplicated on assignment, not moved:

```titan
let x = 42;
let y = x;     // x is copied, not moved
let z = x;     // still valid — x is Copy
```

### 10.3 Immutable Borrows

`&T` borrows a value without taking ownership. Multiple immutable borrows can coexist:

```titan
let s = String::new("hello");
let r1 = &s;
let r2 = &s;    // OK — multiple immutable borrows allowed
// s is still valid
```

### 10.4 Mutable Borrows

`&mut T` provides exclusive write access. Only one mutable borrow may exist at a time, and no immutable borrows may coexist with it:

```titan
let mut v = Vec::new();
let r = &mut v;
r.push(1);
// r ends here
v.push(2);     // OK — mutable borrow r has ended
```

### 10.5 Binding States

The borrow checker tracks each binding through four states:

| State | Meaning |
|-------|---------|
| `UNBORN` | Declared but not yet initialized |
| `ALIVE` | Accessible; may be read or borrowed |
| `MOVED` | Ownership was transferred; any further use is a compile error |
| `DEAD` | Out of scope; memory freed |

### 10.6 Lifetimes

Lifetimes annotate how long a reference remains valid. They are mostly elided (inferred) by three rules:

1. Each parameter gets its own lifetime.
2. If there is exactly one input lifetime, it is assigned to all output lifetimes.
3. If one parameter is `&self` or `&mut self`, its lifetime is assigned to all output lifetimes.

Explicit annotation is required only when elision is ambiguous:

```titan
pub fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

---

## 11. Traits

Traits define shared behavior. They are Titan's mechanism for bounded polymorphism:

```titan
pub trait Display {
    fn fmt(&self) -> str;
}

pub trait Ord: Eq {
    fn cmp(&self, other: &Self) -> i32;
}

pub trait Hash {
    fn hash(&self) -> i64;
}

// Implement a trait for a type
impl Display for Point {
    fn fmt(&self) -> str {
        // format string
    }
}

// Trait objects (dynamic dispatch)
fn print_all(items: &[&dyn Display]) {
    for item in items {
        // item.fmt() dispatched at runtime
    }
}
```

---

## 12. Modules

```titan
// Define a module
mod math {
    pub fn add(a: i64, b: i64) -> i64 { a + b }
    pub fn mul(a: i64, b: i64) -> i64 { a * b }
}

// Use items from a module
use math::add;
use math::*;     // import all public items

// Call with full path
let result = math::add(3, 4);
```

---

## 13. Unsafe

The `unsafe` block grants access to operations the borrow checker cannot verify:

```titan
unsafe {
    let raw: *mut i64 = allocate_raw(8);
    *raw = 42;                           // raw pointer dereference
    let val = *raw;
    free_raw(raw);
}
```

Unsafe operations require `! { unsafe }` in the enclosing function's effect signature. The scope of `unsafe` is always lexically visible in the source — it cannot be hidden behind an abstraction boundary without also propagating the `unsafe` effect.

---

## 14. The Stage 0 Subset

The **Stage 0** subset is the minimal Titan dialect used to bootstrap the compiler. It is what the Python-based reference compiler understands. It excludes:

- Generics
- `impl` blocks and `trait` definitions
- `enum` with payload variants (use `struct` with i64 discriminant tags instead)
- `match` expressions (use `if`/`else` chains)
- Closures (`|x| x + 1`)
- Char literals (`'a'`)
- Dynamic string concatenation

Stage 0 programs use `fn` and `pub fn` declarations, `struct` types, and the full expression and statement set including `if`, `while`, `loop`, `for`, `return`, `break`, `continue`, and `let`/`let mut`. All of the Stage 3B self-hosting modules (`lexer.ti`, `parser.ti`, `borrow_checker.ti`, `lowering.ti`, `codegen.ti`) are valid Stage 0 programs.

---

## 15. Compiler Pipeline

The Titan compiler has five stages, each a separate module:

| Stage | File | Input → Output |
|-------|------|----------------|
| Lexer | `lexer.ti` | Source text → token stream |
| Parser | `parser.ti` | Token stream → AST |
| Borrow Checker | `borrow_checker.ti` | AST → checked AST + diagnostics |
| Lowering | `lowering.ti` | Checked AST → UniIR SSA module |
| Codegen | `codegen.ti` | UniIR → LLVM IR text |

Each stage is independently testable. Diagnostics include line and column numbers. The borrow checker emits errors as `BorrowError` records with error kind tags (`ERR_USE_AFTER_MOVE`, `ERR_CANNOT_BORROW_MUT`, etc.).

---

## 16. Complete Example: Fibonacci

```titan
pub fn fibonacci(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    let mut a = 0;
    let mut b = 1;
    let mut i = 2;
    while i <= n {
        let tmp = a + b;
        a = b;
        b = tmp;
        i = i + 1;
    }
    return b;
}
```

## 17. Complete Example: Struct with Methods and Error Handling

```titan
pub struct Stack {
    data: [i64; 256],
    top: i64,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { data: [0; 256], top: -1 }
    }

    pub fn push(&mut self, val: i64) -> Result<(), str> {
        if self.top >= 255 {
            return Err("stack overflow");
        }
        self.top = self.top + 1;
        self.data[self.top] = val;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<i64, str> {
        if self.top < 0 {
            return Err("stack underflow");
        }
        let val = self.data[self.top];
        self.top = self.top - 1;
        Ok(val)
    }

    pub fn peek(&self) -> Option<i64> {
        if self.top < 0 { None } else { Some(self.data[self.top]) }
    }
}

pub fn main() ! { io } {
    let mut s = Stack::new();
    s.push(10);
    s.push(20);
    match s.pop() {
        Ok(val)  => { /* use val */ }
        Err(msg) => { /* handle error */ }
    }
}
```

---

## 18. Quick Reference Card

```
TYPES          i64  f64  bool  str  ()  &T  &mut T  [T;N]  [T]
KEYWORDS       fn   pub  let   mut  if  else  while  loop  for  in
               return  break  continue  struct  enum  match  impl  trait
               mod  use  extern  unsafe  move  async  await  alloc  emit
OPERATORS      + - * / %    & | ^ ~ << >>    == != < > <= >=    && || !
               = += -= *=    . :: []    & &mut *    !{}
EFFECTS        alloc  io  panic  telemetry  unsafe  network(pat)
               read_fs(path)  write_fs(path)  device(gpu|fpga|tpu|simd)
LITERALS       42  0xFF  0b101  3.14f64  true  false  "string"  ()
```
