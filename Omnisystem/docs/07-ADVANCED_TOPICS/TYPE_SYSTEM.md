# Type System Deep Dive

**Complete guide to Omnisystem's hybrid static/dynamic type system**

---

## Type System Overview

Omnisystem uses a **hybrid type system** combining:
- **Static types** (compile-time checking, TITAN/AETHER/AXIOM)
- **Dynamic types** (runtime flexibility, SYLVA)
- **Type inference** (automatic type derivation)
- **Unification** (constraint solving)

---

## Primitive Types

### TITAN Primitives
```
Integer:    i8, i16, i32, i64, i128
Unsigned:   u8, u16, u32, u64, u128
Float:      f32, f64
Boolean:    bool
Character:  char
String:     string (owned), &str (borrowed)
Unit:       ()
```

### SYLVA Numeric
```
All operations on f32/f64
Tensor<T> where T: Numeric
Complex numbers supported
```

### AETHER Specific
```
NodeId, NetworkAddress
Duration, Instant
MessageType enum
ConsensusType enum
```

### AXIOM Logic
```
Type::Unit, Type::Bool
Type::Int, Type::Float
Type::String
Type::Array, Type::Tuple
```

---

## Composite Types

### Structs
```titan
type Point {
    x: i32,
    y: i32,
}

let p = Point { x: 10, y: 20 }
```

### Enums
```titan
enum Color {
    Red,
    Green,
    Blue,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

### Tuples
```titan
let tuple: (i32, string) = (42, "answer")
tuple.0  // 42
tuple.1  // "answer"
```

### Collections
```titan
Vec<T>              // Dynamic array
HashMap<K, V>       // Hash map
HashSet<T>          // Hash set
[T; N]              // Array with size
```

---

## Type Parameters & Generics

### Generic Functions
```titan
fun identity<T>(x: T) -> T {
    x
}

fun first<T>(items: Vec<T>) -> Option<T> {
    items.first()
}
```

### Generic Structs
```titan
type Container<T> {
    item: T,
}

type Result<T, E> {
    Ok(T),
    Err(E),
}
```

### Trait Bounds
```titan
fun process<T: Clone>(item: T) -> T {
    item.clone()
}

fun sum<T: Add + Zero>(items: Vec<T>) -> T {
    items.iter().fold(T::zero(), |a, b| a + b)
}
```

---

## Type Inference

### Basic Inference
```titan
let x = 5              // inferred: i32
let x = 5.0            // inferred: f64
let x = "hello"        // inferred: &str
```

### Collection Inference
```titan
let v = vec![1, 2, 3]  // inferred: Vec<i32>
let m = HashMap::new() // requires type annotation
let m: HashMap<&str, i32> = HashMap::new()
```

### Function Return Inference
```titan
fun get_value() -> i32 {
    5                  // type inferred from return type
}
```

### Constraint Solving
```titan
let x = 5
let y = 10.0
// let z = x + y     // ERROR: can't add i32 and f64
let z = (x as f64) + y  // OK: explicit cast
```

---

## Type Checking

### Compile-Time Checking (TITAN)
```titan
let x: i32 = 5
let y: i32 = "hello"   // ERROR: type mismatch
```

### Runtime Checking (SYLVA)
```sylva
let t: Tensor = input
// Type checked at runtime
t.reshape([3, 3])?     // Can fail if shape incompatible
```

### Distributed Checking (AETHER)
```aether
// Types must be serializable
pub fn send<T: Serialize>(msg: T) -> Result<()>
```

### Formal Checking (AXIOM)
```axiom
// All types must be definable in logic
Type::Int
Type::String
Type::Array(Box::new(Type::Int))
```

---

## Type Classes & Traits

### Common Traits
```titan
trait Clone { fn clone(&self) -> Self }
trait Copy { /* marker trait */ }
trait Debug { fn fmt(&self, f: &mut Formatter) -> Result }
trait Display { fn fmt(&self, f: &mut Formatter) -> Result }
trait Eq { fn eq(&self, other: &Self) -> bool }
trait Ord { fn cmp(&self, other: &Self) -> Ordering }
```

### Custom Traits
```titan
trait Shape {
    fn area(&self) -> f64
    fn perimeter(&self) -> f64
}

trait Drawable: Shape {
    fn draw(&self)
}
```

---

## Variance & Subtyping

### Covariance
```titan
// If Dog is subtype of Animal
fn take_animals(animals: Vec<Animal>) { }
let dogs: Vec<Dog> = vec![dog1, dog2]
// take_animals(dogs)  // ERROR: not covariant
```

### Contravariance
```titan
fn handler<T>(f: impl Fn(T)) { }
// More specific handlers work with less specific types
```

---

## Advanced Type Features

### Higher-Ranked Types
```titan
fun map<F>(f: for<'a> fn(&'a str) -> i32) { }
```

### Associated Types
```titan
trait Iterator {
    type Item
    fn next(&mut self) -> Option<Self::Item>
}
```

### Type Aliases
```titan
type Integer = i32
type ByteString = Vec<u8>
```

---

## Type System Statistics

### TITAN
```
Base Types:       7 (int, uint, float, bool, char, string, unit)
Composite Types:  4 (struct, enum, tuple, array)
Traits:           15+ (standard library)
Generic Support:  Full
Inference:        Strong (local)
```

### SYLVA
```
Base Types:       2 (f32, f64)
Composite:        Tensor<T> only
Type Safety:      Runtime checked
Inference:        Per-operation
```

### AETHER
```
Serializable:     Required for all network types
Custom Types:     Message, Node, Cluster
Type Reflection:  Full
```

### AXIOM
```
Formulas:         First-order logic
Type System:      Full static
Proof Support:    Yes
```

---

## Type Conversion

### Implicit Conversions
```titan
let x: f64 = 5      // i32 to f64 (lossless)
```

### Explicit Casting
```titan
let x = 5_i32 as f64
let x = 3.14_f64 as i32  // truncates
```

### Into/From Traits
```titan
let s = String::from("hello")
let s: String = "hello".into()
```

---

## Type Errors

### Common Errors
```
Type Mismatch:       Expected T, found U
Borrow Checker:      Can't borrow as mutable
Generic Bounds:      T doesn't implement required trait
Inference Failure:   Can't infer type
Unification Error:   Can't unify types
```

### Debugging Types
```titan
fn debug_type<T>() {
    println!("{}", std::any::type_name::<T>())
}

debug_type::<i32>()  // prints "i32"
```

---

## Best Practices

✅ **DO**
- Use type annotations for public APIs
- Let inference work for local variables
- Use concrete types when possible
- Leverage trait bounds for generics

❌ **DON'T**
- Rely on inference for complex types
- Over-generalize with type parameters
- Ignore type error messages
- Use `as` casting unnecessarily

---

## Next Steps

- Reference: [QUICK_REFERENCE.md](QUICK_REFERENCE.md)
- Bridges: [LANGUAGE_BRIDGES.md](LANGUAGE_BRIDGES.md)
- Performance: [PERFORMANCE.md](PERFORMANCE.md)

---

**Omnisystem Type System** - Flexible, powerful, safe, and fast.
