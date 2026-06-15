# Quick Reference - Syntax Cheat Sheet

**Quick lookup for syntax in all Omnisystem languages**

---

## TITAN - Systems Programming

### Variables & Types
```titan
let x = 5                   // Type inferred
let x: i32 = 5             // Explicit type
let mut x = 5              // Mutable
const MAX = 100            // Constant
```

### Functions
```titan
fun add(a: i32, b: i32) -> i32 {
    a + b
}

fun greet(name: &str) {    // No return type = ()
    println!("Hello, {}", name)
}
```

### Control Flow
```titan
if x > 0 { println!("positive") }
else if x < 0 { println!("negative") }
else { println!("zero") }

match x {
    0 => println!("zero"),
    1..=10 => println!("small"),
    _ => println!("large"),
}

while x < 10 { x += 1 }
for i in 0..10 { println!("{}", i) }
```

### Collections
```titan
let arr = [1, 2, 3]        // Array (fixed size)
let vec = vec![1, 2, 3]    // Vector (dynamic)
let map = HashMap::new()   // Hash map
let map = maplit::hashmap! { "a" => 1 }

vec.push(4)
vec.len()
vec[0]                     // Access
```

### Memory Management
```titan
let x = Box::new(5)        // Heap allocation
let x = Rc::new(5)         // Reference counted
let x = Arc::new(5)        // Atomic RC
let x = Mutex::new(5)      // Mutual exclusion
```

### Error Handling
```titan
Result<T, E>               // Result type
Ok(value)                  // Success
Err(error)                 // Error

fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 { Err("division by zero".into()) }
    else { Ok(a / b) }
}

match divide(10, 2) {
    Ok(result) => println!("{}", result),
    Err(e) => println!("Error: {}", e),
}

value?  // Propagate error
```

### Traits & Impl
```titan
trait Shape {
    fn area(&self) -> f64
}

struct Circle { radius: f64 }

impl Shape for Circle {
    fn area(&self) -> f64 { 3.14 * self.radius * self.radius }
}
```

---

## SYLVA - Machine Learning

### Tensors
```sylva
let t = Tensor::zeros([2, 3])
let t = Tensor::ones([3, 4])
let t = Tensor::random([5, 5])
let t = Tensor::randn([10, 10])   // Normal dist

t.shape()                   // [2, 3]
t.reshape([6])?             // Reshape
t.flatten()                 // To 1D
t.transpose()?              // Transpose
t.sum() / t.mean() / t.std()
```

### Neural Networks
```sylva
let model = Sequential::new()
    .add(Dense::new(10, 5))
    .add(Dense::new(5, 1))

let output = model.forward(&input)?
model.backward(&loss)?
model.save("model.bin")?
```

### Loss Functions
```sylva
mse_loss(&pred, &target)    // Mean squared error
cross_entropy(&pred, &target) // Classification
bce_loss(&pred, &target)    // Binary classification
```

### Optimizers
```sylva
let mut opt = Adam::new(0.001)
let mut opt = SGD::new(0.01)
let mut opt = RMSprop::new(0.001)

opt.step(model.parameters())
opt.zero_grad()
```

### Activation Functions
```sylva
relu(&x)      // ReLU
sigmoid(&x)   // Sigmoid
tanh(&x)      // Tanh
softmax(&x)   // Softmax
gelu(&x)      // GELU
```

---

## AETHER - Distributed Systems

### Cluster
```aether
let mut cluster = Cluster::new()
    .with_min_replicas(3)

cluster.add_node("node1", "127.0.0.1:5001")?
cluster.start_consensus(ConsensusType::Raft)?
cluster.start()?

cluster.get_leader()
cluster.member_count()
```

### Consensus Types
```aether
ConsensusType::Raft      // Raft consensus
ConsensusType::Paxos     // Paxos
ConsensusType::Byzantine // Byzantine FT
```

### Distributed Storage
```aether
let store = DistributedStore::new(Arc::new(cluster))
    .with_replication_factor(3)
    .with_consistency_level(ConsistencyLevel::Strong)

store.put("key", "value", Durability::Persistent)?
store.get("key")?
store.delete("key")?
store.range("pattern")?
```

### Messages
```aether
let msg = Message::new(MessageType::Data, "target", "payload")
node.send_message(&msg)?
node.broadcast(&msg)?
```

### Sharding
```aether
let part = Partitioner::new(16)
let shard = part.get_shard("key")
let replicas = part.get_replicas("key")
```

---

## AXIOM - Formal Verification

### Formulas
```axiom
Formula::Atom("P")
Formula::Not(Box::new(f))
Formula::And(Box::new(f1), Box::new(f2))
Formula::Or(Box::new(f1), Box::new(f2))
Formula::Implies(Box::new(f1), Box::new(f2))
Formula::ForAll("x", Box::new(f))
Formula::Exists("x", Box::new(f))
Formula::Equals("x", "5")
```

### Types
```axiom
Type::Unit
Type::Bool
Type::Int
Type::Float
Type::String
Type::Array(Box::new(Type::Int))
Type::Function(vec![Type::Int], Box::new(Type::Int))
```

### Theorem Proving
```axiom
let mut prover = TheoremProver::new()
prover.add_axiom("axiom_string")?
prover.prove(&formula)?
```

### Specifications
```axiom
spec func(x: Int) {
    precondition: x > 0
    postcondition: result >= x
    invariant: x != 0
}
```

---

## Common Operations by Language

### Output
```
TITAN:   println!("Hello, {}", x)
SYLVA:   println!("Tensor: {:?}", tensor)
AETHER:  println!("Node: {}", node.id())
AXIOM:   println!("Formula: {:?}", formula)
```

### Create Type
```
TITAN:   type Point { x: i32, y: i32 }
SYLVA:   type Model { layers: Vec<Dense> }
AETHER:  type Node { id: String, address: String }
AXIOM:   type Proof { steps: Vec<ProofStep> }
```

### Error Handling
```
TITAN:   Err("error message".to_string())
SYLVA:   Err(LayerError::InvalidInputShape)
AETHER:  Err(DistributedError::QuorumNotReached)
AXIOM:   Err(ProofError::TimeoutExceeded)
```

### Loop
```
TITAN:   for i in 0..10 { ... }
SYLVA:   for batch in loader { ... }
AETHER:  for node in cluster.get_replicas() { ... }
AXIOM:   for step in proof.steps { ... }
```

---

## Type System Quick Lookup

### Primitive Types
```
TITAN:   i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool, char
SYLVA:   f32, f64 (tensors are type-generic)
AETHER:  String, u32 (for node IDs), Duration
AXIOM:   Type::Int, Type::Bool, Type::String, Type::Float
```

### Composite Types
```
TITAN:   Vec<T>, HashMap<K, V>, (T1, T2), [T; N]
SYLVA:   Tensor<T>
AETHER:  Message, Node, Cluster
AXIOM:   Formula, Type, Proof
```

---

## Module/Import Syntax

```
TITAN:   use std::collections::HashMap
SYLVA:   use sylva::nn::*
AETHER:  use aether::cluster::*
AXIOM:   use axiom::logic::*
```

---

## File Extensions

```
TITAN:     .ti
SYLVA:     .sy
AETHER:    .ae
AXIOM:     .ax
OMNI:      .omni
Config:    .toml
```

---

## Command Line

```bash
omnisystem run program.ti                  # Run program
omnisystem compile program.ti              # Compile
omnisystem repl                            # Start REPL
omnisystem new --language titan myapp     # New project
omnisystem module list                     # List modules
omnisystem module install <name>           # Install module
omnisystem build                           # Build project
omnisystem test                            # Run tests
omnisystem fmt                             # Format code
omnisystem lint                            # Lint code
```

---

## Common Patterns

### Iterate with Index
```titan
for (i, item) in vec.iter().enumerate() { ... }
```

### Map/Filter
```titan
let doubled: Vec<_> = vec.iter().map(|x| x * 2).collect()
let evens: Vec<_> = vec.iter().filter(|x| x % 2 == 0).collect()
```

### Option/Result Chaining
```titan
value.ok_or(error)?
result.map(|v| v + 1).unwrap_or(0)
```

### Closures
```titan
let add = |x, y| x + y
let doubled = vec.iter().map(|x| x * 2)
```

---

## Key Differences

| Feature | TITAN | SYLVA | AETHER | AXIOM |
|---------|-------|-------|--------|-------|
| **Use** | Systems | ML/AI | Distributed | Verification |
| **Paradigm** | Imperative | Functional | Distributed | Logical |
| **Memory** | Manual | Auto | Replicated | Logical |
| **Error** | Result | Option | Consensus | Logic |
| **Concurrency** | Threads | Data parallelism | Network | Proof |

---

## Quick Tips

- **TITAN**: Use `?` operator for error propagation
- **SYLVA**: Always handle Result types from operations
- **AETHER**: Always handle replication and consensus
- **AXIOM**: Use `:?` for debug format in println

---

## Need More?

- Full syntax: Read language guides
- API details: Check API_*.md files
- Examples: See HELLO_WORLD.md
- Tutorials: Follow TUTORIAL_*.md files

---

**Print This Page** - Great for desk reference!
