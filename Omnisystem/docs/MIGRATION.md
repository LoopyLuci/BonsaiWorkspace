# Migration Guide

**Migrate code from other languages to Omnisystem**

---

## Migration Overview

| From | To | Difficulty | Time |
|------|-----|-----------|------|
| C++ | TITAN | Medium | 2x lines |
| Rust | TITAN | Low | 1x lines |
| Java | TITAN | Medium | 1.5x lines |
| Python | SYLVA | Low | 1x lines |
| Go | AETHER | Medium | 1.5x lines |

---

## Rust → TITAN

### Similarities
- Ownership and borrowing
- Pattern matching
- Traits for polymorphism
- Strong type system

### Key Differences
```rust
// Rust
fn add(a: i32, b: i32) -> i32 { a + b }

// TITAN (no significant change)
fun add(a: i32, b: i32) -> i32 { a + b }
```

### Migration Steps
1. Rename `.rs` to `.ti`
2. Replace `fn` with `fun`
3. Update module system (use → use)
4. Update standard library calls
5. Test thoroughly

---

## C++ → TITAN

### Memory Management
```cpp
// C++
int* ptr = new int(5);
delete ptr;

// TITAN
let ptr = Box::new(5)  // Automatic cleanup
```

### Classes → Structs + Impl
```cpp
// C++
class Point {
    int x, y;
public:
    Point(int x, int y) : x(x), y(y) {}
};

// TITAN
type Point { x: i32, y: i32 }
impl Point {
    fun new(x: i32, y: i32) -> Point {
        Point { x, y }
    }
}
```

### Migration Strategy
1. Port data structures (class → type)
2. Port methods (class methods → impl blocks)
3. Port memory management (new/delete → Box/Arc)
4. Port error handling (exceptions → Result)
5. Test incrementally

---

## Java → TITAN

### Object Model
```java
// Java
class Animal {
    public String speak() { return "noise"; }
}

// TITAN
type Animal { name: string }
impl Animal {
    fun speak(&self) -> string { "noise" }
}
```

### Null Handling
```java
// Java
String name = null;
if (name != null) { ... }

// TITAN
let name: Option<string> = None
match name {
    Some(n) => { ... },
    None => { ... }
}
```

### Collections
```java
// Java
List<Integer> items = new ArrayList<>();
items.add(5);

// TITAN
let mut items = Vec::new()
items.push(5)
```

---

## Python → SYLVA

### Type Hints
```python
# Python (no types)
def add(a, b):
    return a + b

# SYLVA (typed)
fun add(a: f32, b: f32) -> f32 {
    a + b
}
```

### NumPy → Tensors
```python
# Python/NumPy
import numpy as np
x = np.array([[1, 2], [3, 4]])
y = np.sum(x)

# SYLVA
let x = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2])?
let y = x.sum()
```

### Lists → Vectors
```python
# Python
items = [1, 2, 3]
items.append(4)

# SYLVA
let mut items = vec![1.0, 2.0, 3.0]
items.push(4.0)
```

### Migration Strategy
1. Add type annotations
2. Replace NumPy with Tensors
3. Replace lists with Vectors
4. Rewrite control flow (more functional)
5. Test models on same data

---

## Go → AETHER

### Goroutines → Tasks
```go
// Go
go func() {
    fmt.Println("Concurrent")
}()

// AETHER (async message passing)
cluster.send_message(Message::new(...))
```

### Channels → Message Passing
```go
// Go
ch := make(chan int)
ch <- 5

// AETHER
node.send_message(&msg)?
node.on_message(|msg| { ... })
```

### Interfaces → Traits
```go
// Go
type Reader interface {
    Read(p []byte) (n int, err error)
}

// AETHER
trait Reader {
    fun read(&self, buf: &mut [u8]) -> Result<usize>
}
```

### Migration Strategy
1. Replace goroutines with message passing
2. Replace channels with async messages
3. Replace interfaces with traits
4. Add consensus/distribution logic
5. Test networking carefully

---

## JavaScript → TITAN

### No Dynamic Types
```javascript
// JavaScript
let x = 5
x = "hello"  // OK

// TITAN
let x: i32 = 5
x = "hello"  // ERROR
```

### Callbacks → Futures/Async
```javascript
// JavaScript
fetch('/api/data').then(r => r.json())

// TITAN
let resp = http::get("/api/data")?
let json = resp.json()?
```

### Objects → Structs
```javascript
// JavaScript
let point = { x: 5, y: 10 }

// TITAN
type Point { x: i32, y: i32 }
let point = Point { x: 5, y: 10 }
```

---

## SQL → AETHER Storage

### Queries → KV Operations
```sql
-- SQL
SELECT * FROM users WHERE id = 5

-- AETHER
let user = store.get("users:5")?
```

### Transactions → Distributed Transactions
```sql
-- SQL
BEGIN;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;
UPDATE accounts SET balance = balance + 100 WHERE id = 2;
COMMIT;

-- AETHER
let tx = DistributedTransaction::new()
    .add_operation(Operation::Write("accounts:1", ...))
    .add_operation(Operation::Write("accounts:2", ...))

cluster.commit(&tx)?
```

---

## Testing Migration

### Create Harness
```bash
# Keep old system running
# Route 5% of traffic to new TITAN system
# Monitor metrics
# Gradually increase traffic
# Monitor error rates, latency
```

### Validate Correctness
```bash
# Test against same inputs
# Compare outputs
# Verify performance
# Load test
```

---

## Common Mistakes

❌ **DON'T**
- Migrate everything at once
- Skip testing
- Ignore performance
- Force functional style inappropriately

✅ **DO**
- Migrate piece by piece
- Test incrementally
- Monitor performance
- Use appropriate paradigms

---

## Migration Checklist

- [ ] Identify modules to migrate
- [ ] Create test suite with baseline
- [ ] Port incrementally
- [ ] Validate correctness
- [ ] Performance test
- [ ] Security review
- [ ] Deploy gradually
- [ ] Monitor in production

---

## Timeline Estimates

**Small project (< 5k lines)**
- C++ → TITAN: 1-2 weeks
- Python → SYLVA: 3-5 days
- Go → AETHER: 2-3 weeks

**Medium project (5-50k lines)**
- C++ → TITAN: 2-4 weeks
- Python → SYLVA: 1-2 weeks
- Go → AETHER: 3-6 weeks

**Large project (> 50k lines)**
- Phased migration
- 2-3 months minimum
- Parallel running period recommended

---

## Next Steps

- Language guides for reference
- [COMPARISON.md](COMPARISON.md) for feature comparison
- [FAQ.md](FAQ.md) for common questions
- Community support: omnisystem.io/forum

---

**Migration** - Smoothly transition to Omnisystem!
