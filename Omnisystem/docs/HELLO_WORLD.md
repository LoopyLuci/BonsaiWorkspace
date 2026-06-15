# Hello World - All Languages

**Your first program in each Omnisystem language**

---

## TITAN - Systems Programming

### Hello World
```titan
fun main() {
    println!("Hello, World from TITAN!")
}
```

### Save as `hello.ti`
```bash
omnisystem run hello.ti
# Hello, World from TITAN!
```

### With Variables
```titan
fun main() {
    let name = "TITAN"
    let version = 2.0
    println!("Hello, {} v{}", name, version)
}
```

### With Functions
```titan
fun greet(name: &str) -> string {
    format!("Hello, {}!", name)
}

fun main() {
    let message = greet("World")
    println!("{}", message)
}
```

### With Collections
```titan
fun main() {
    let languages = vec!["TITAN", "SYLVA", "AETHER", "AXIOM"]
    
    for lang in &languages {
        println!("Hello from {}", lang)
    }
}
```

---

## SYLVA - Machine Learning

### Hello World
```sylva
fun main() -> Result<()> {
    println!("Hello from SYLVA!")
    Ok(())
}
```

### Save as `hello.sy`
```bash
omnisystem run hello.sy
# Hello from SYLVA!
```

### With Tensors
```sylva
fun main() -> Result<()> {
    let data = Tensor::from_vec(vec![1.0, 2.0, 3.0], vec![3])?
    println!("Tensor: {:?}", data)
    println!("Sum: {}", data.sum())
    Ok(())
}
```

### With Neural Network
```sylva
use sylva::nn::*

fun main() -> Result<()> {
    let model = Sequential::new()
        .add(Dense::new(3, 5))
        .add(Dense::new(5, 1))
    
    let input = Tensor::randn([1, 3])
    let output = model.forward(&input)?
    
    println!("Output: {:?}", output)
    Ok(())
}
```

### With Training
```sylva
fun main() -> Result<()> {
    let mut model = Sequential::new()
        .add(Dense::new(10, 5))
        .add(Dense::new(5, 1))
    
    let mut optimizer = Adam::new(0.01)
    
    for epoch in 0..5 {
        let pred = model.forward(&input)?
        let loss = mse_loss(&pred, &target)
        model.backward(&loss)?
        optimizer.step(model.parameters())
        println!("Epoch {}: loss = {}", epoch, loss)
    }
    
    Ok(())
}
```

---

## AETHER - Distributed Systems

### Hello World
```aether
fun main() -> Result<(), string> {
    println!("Hello from AETHER!")
    Ok(())
}
```

### Save as `hello.ae`
```bash
omnisystem run hello.ae
# Hello from AETHER!
```

### With Cluster
```aether
use aether::cluster::*

fun main() -> Result<(), string> {
    let mut cluster = Cluster::new()
    cluster.add_node("node1", "127.0.0.1:5001")?
    
    println!("Cluster created")
    Ok(())
}
```

### With Consensus
```aether
fun main() -> Result<(), string> {
    let mut cluster = Cluster::new()
    cluster.add_node("node1", "127.0.0.1:5001")?
    cluster.add_node("node2", "127.0.0.1:5002")?
    
    cluster.start_consensus(ConsensusType::Raft)?
    cluster.start()?
    
    match cluster.get_leader() {
        Some(leader) => println!("Leader: {}", leader),
        None => println!("No leader yet"),
    }
    
    Ok(())
}
```

### With Distributed Storage
```aether
fun main() -> Result<(), string> {
    let store = DistributedStore::new(Arc::new(cluster))
    
    store.put("key1", "value1", Durability::Persistent)?
    let value = store.get("key1")?
    
    println!("Stored: {}", value)
    Ok(())
}
```

---

## AXIOM - Formal Verification

### Hello World
```axiom
fun main() -> Result<()> {
    println!("Hello from AXIOM!")
    Ok(())
}
```

### Save as `hello.ax`
```bash
omnisystem run hello.ax
# Hello from AXIOM!
```

### With Logic
```axiom
use axiom::logic::*

fun main() -> Result<()> {
    let formula = Formula::Atom("P".to_string())
    println!("Formula: {:?}", formula)
    
    let negated = Formula::Not(Box::new(formula))
    println!("Negated: {:?}", negated)
    
    Ok(())
}
```

### With Theorem Proving
```axiom
fun main() -> Result<()> {
    let mut prover = TheoremProver::new()
    
    // Tautology: P → P
    let theorem = Formula::Implies(
        Box::new(Formula::Atom("P".to_string())),
        Box::new(Formula::Atom("P".to_string()))
    )
    
    match prover.prove(&theorem) {
        Ok(proof) => println!("✓ Proved in {} steps", proof.steps.len()),
        Err(_) => println!("✗ Could not prove"),
    }
    
    Ok(())
}
```

### With Specifications
```axiom
fun divide(a: i32, b: i32) -> i32
    where {
        precondition: b != 0,
        postcondition: result * b + remainder = a,
    }
{
    a / b
}

fun main() -> Result<()> {
    let result = divide(10, 2)
    println!("10 / 2 = {}", result)
    Ok(())
}
```

---

## Running from REPL

### TITAN REPL
```bash
$ omnisystem repl --language titan
omnisystem-titan> 2 + 3
5
omnisystem-titan> let x = "Hello"
omnisystem-titan> println!("{}", x)
Hello
omnisystem-titan> :quit
```

### SYLVA REPL
```bash
$ omnisystem repl --language sylva
omnisystem-sylva> let t = Tensor::ones([2, 3])
omnisystem-sylva> t.shape()
[2, 3]
omnisystem-sylva> t.sum()
6.0
omnisystem-sylva> :quit
```

### AETHER REPL
```bash
$ omnisystem repl --language aether
omnisystem-aether> let node = Node::new("test", "127.0.0.1:5000")
omnisystem-aether> node.id()
"test"
omnisystem-aether> :quit
```

### AXIOM REPL
```bash
$ omnisystem repl --language axiom
omnisystem-axiom> let f = Formula::Atom("P")
omnisystem-axiom> f.to_string()
"P"
omnisystem-axiom> :quit
```

---

## One-Liners

### TITAN
```bash
omnisystem run --code 'println!("Hello!")'
```

### SYLVA
```bash
omnisystem run --code 'println!("{}", Tensor::ones([2, 3]).sum())'
```

### AETHER
```bash
omnisystem run --code 'println!("{:?}", Cluster::new())'
```

### AXIOM
```bash
omnisystem run --code 'println!("{:?}", Formula::Atom("P".to_string()))'
```

---

## Project Templates

### Create from template
```bash
# TITAN project
omnisystem new --language titan my-app

# SYLVA project
omnisystem new --language sylva my-ml

# AETHER project
omnisystem new --language aether my-cluster

# AXIOM project
omnisystem new --language axiom my-verify
```

### Project structure
```
my-project/
├── omnisystem.toml
├── src/
│   ├── main.ti        (or .sy, .ae, .ax)
│   └── lib.ti
├── tests/
└── examples/
```

---

## Next Steps

1. Choose your language based on your use case
2. Complete the language-specific tutorial:
   - TITAN: [TITAN_LANGUAGE_GUIDE.md](TITAN_LANGUAGE_GUIDE.md)
   - SYLVA: [SYLVA_LANGUAGE_GUIDE.md](SYLVA_LANGUAGE_GUIDE.md)
   - AETHER: [AETHER_LANGUAGE_GUIDE.md](AETHER_LANGUAGE_GUIDE.md)
   - AXIOM: [AXIOM_LANGUAGE_GUIDE.md](AXIOM_LANGUAGE_GUIDE.md)

3. Check [QUICK_REFERENCE.md](QUICK_REFERENCE.md) for syntax

4. Explore framework guides:
   - [WEB_FRAMEWORK_GUIDE.md](WEB_FRAMEWORK_GUIDE.md)
   - [SYSTEMS_FRAMEWORK_GUIDE.md](SYSTEMS_FRAMEWORK_GUIDE.md)

---

**Ready to build!** Choose your language and get started! 🚀
