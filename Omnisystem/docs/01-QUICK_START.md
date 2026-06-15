# Quick Start Guide - Omnisystem V2.0

## Installation

### Prerequisites
- Rust 1.70+ (for compilation)
- 4GB RAM minimum
- 2GB disk space

### Install from Source

```bash
# Clone repository
git clone https://github.com/omnisystem/omnisystem.git
cd omnisystem

# Build release version
cargo build --release

# Run tests
cargo test --all --release

# Install CLI tools
cargo install --path .
```

### Verify Installation

```bash
# Check version
omni --version

# Run interactive REPL
omni repl

# List available commands
omni --help
```

---

## Your First Program

### 1. TITAN - Systems Programming

**File: hello.titan**
```titan
// Simple hello world
fn main() {
    println!("Hello from Omnisystem!");
}
```

**Run:**
```bash
omni run hello.titan
```

### 2. SYLVA - Machine Learning

**File: neural.sylva**
```sylva
module SimpleNN {
    pub struct NeuralNetwork {
        weights: vector<f64>,
        layers: i32,
    }

    impl NeuralNetwork {
        pub fn new(layers: i32) -> Self {
            NeuralNetwork {
                weights: vector::new(),
                layers,
            }
        }

        pub fn predict(&self, input: f64) -> f64 {
            input * 0.5 + 0.1  // Simple linear function
        }
    }
}

fn main() {
    let nn = NeuralNetwork::new(3);
    let result = nn.predict(5.0);
    println!("Prediction: {}", result);
}
```

**Run:**
```bash
omni run neural.sylva
```

### 3. AETHER - Distributed Systems

**File: distributed.aether**
```aether
module SimpleCluster {
    pub struct ClusterNode {
        node_id: string,
        address: string,
    }

    impl ClusterNode {
        pub fn new(id: string, addr: string) -> Self {
            ClusterNode {
                node_id: id,
                address: addr,
            }
        }

        pub fn send_message(&self, msg: string) -> Result<(), string> {
            println!("🔗 Sending from {}: {}", self.node_id, msg);
            Ok(())
        }
    }
}

fn main() {
    let node = ClusterNode::new("node-1".to_string(), "127.0.0.1:8080".to_string());
    node.send_message("Hello cluster".to_string()).ok();
}
```

**Run:**
```bash
omni run distributed.aether
```

### 4. AXIOM - Formal Verification

**File: verify.axiom**
```axiom
module Verification {
    pub struct Proof {
        statement: string,
        verified: bool,
    }

    impl Proof {
        pub fn new(stmt: string) -> Self {
            Proof {
                statement: stmt,
                verified: false,
            }
        }

        pub fn verify(&mut self) -> bool {
            println!("✓ Verifying: {}", self.statement);
            self.verified = true;
            true
        }
    }
}

fn main() {
    let mut proof = Proof::new("2+2=4".to_string());
    if proof.verify() {
        println!("✅ Proof verified!");
    }
}
```

**Run:**
```bash
omni run verify.axiom
```

---

## Working with Projects

### Create a New Project

```bash
# Create project structure
omni new my-project
cd my-project
```

### Project Structure

```
my-project/
├── Cargo.toml           # Project manifest
├── src/
│   ├── main.titan       # Entry point
│   └── lib/
│       ├── utils/       # Utilities
│       └── core/        # Core logic
├── tests/               # Integration tests
├── examples/            # Example programs
└── docs/                # Documentation
```

### Build and Run

```bash
# Build project
omni build

# Run with default entry
omni run

# Run specific file
omni run src/main.titan

# Run in debug mode
omni run --debug

# Run with optimizations
omni run --release
```

---

## Using the REPL

### Interactive Mode

```bash
omni repl
```

Then interact:

```
omni> let x = 5;
omni> println!("x = {}", x);
x = 5
omni> let y = x * 2;
omni> println!("y = {}", y);
y = 10
omni> :exit
```

### Commands

```
:help           Show help
:exit           Exit REPL
:clear          Clear screen
:load <file>    Load file
:vars           Show variables
:types          Show types
:history        Show history
```

---

## Debugging

### Using the Debugger

```bash
# Run with debugger
omni debug my-program.titan
```

### Debugger Commands

```
(gdb) break main           # Set breakpoint at main
(gdb) break function_name  # Break at function
(gdb) continue             # Continue execution
(gdb) step                 # Step into
(gdb) next                 # Step over
(gdb) print variable       # Print variable
(gdb) backtrace            # Show call stack
(gdb) quit                 # Exit debugger
```

### Remote Debugging

```bash
# Start remote debugger
omni debug --remote localhost:9001

# Connect from IDE
# Configure IDE to connect to localhost:9001
```

---

## Package Management

### Declaring Dependencies

**Cargo.toml:**
```toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2024"

[dependencies]
# Built-in modules
titan = "2.0"
sylva = "2.0"
aether = "2.0"
axiom = "2.0"

# Extensions
titan-data-processing = "2.0.22"
sylva-reinforcement-learning = "2.0.22"
aether-networking = "2.0.22"

[dev-dependencies]
test-framework = "2.0"
```

### Install Dependencies

```bash
omni install

# Or update to latest
omni update
```

### Publish Your Package

```bash
# Create account
omni login

# Publish
omni publish

# Publish specific version
omni publish --version 1.0.0
```

---

## Testing

### Write Tests

**File: src/lib/my_module.titan**
```titan
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
}

#[test]
fn test_add_zero() {
    assert_eq!(add(5, 0), 5);
}
```

### Run Tests

```bash
# Run all tests
omni test

# Run specific test
omni test test_add

# Run with output
omni test -- --nocapture

# Run with coverage
omni test --coverage
```

---

## Common Tasks

### Reading Files

```titan
use std::fs;

fn main() {
    let contents = fs::read_to_string("data.txt")
        .expect("Failed to read file");
    println!("{}", contents);
}
```

### Writing Files

```titan
use std::fs;

fn main() {
    fs::write("output.txt", "Hello World!")
        .expect("Failed to write file");
}
```

### HTTP Requests (with networking extension)

```aether
use aether::http;

fn main() {
    let response = http::get("https://api.example.com/data")
        .expect("Request failed");
    
    println!("Status: {}", response.status);
    println!("Body: {}", response.body);
}
```

### Data Processing (with data-processing extension)

```titan
use titan::stream::*;

fn main() {
    let stream = DataStream::new(100);
    
    stream.push(42);
    stream.push(24);
    
    let batch = stream.process_batch();
    println!("Processed: {}", batch.len());
}
```

---

## Performance Tips

### 1. Use Release Builds
```bash
omni run --release my-program.titan
```

### 2. Profile Your Code
```bash
omni profile my-program.titan
```

### 3. Enable Optimizations
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### 4. Use Proper Data Structures
- Use `Vec` for growable arrays
- Use `HashMap` for key-value storage
- Use `BTreeMap` for sorted keys

### 5. Avoid Cloning
```titan
// Bad: clones data
let copy = vec.clone();

// Good: borrows reference
let reference = &vec;
```

---

## Next Steps

1. **Read Architecture Guide**: Understand system design
2. **Learn a Language**: Start with TITAN for systems programming
3. **Explore Extensions**: Choose relevant modules for your use case
4. **Build Examples**: Work through provided examples
5. **Join Community**: Ask questions and share knowledge

---

## Getting Help

### Documentation
- [Full Documentation](./README.md)
- [Language Guides](./03-LANGUAGES/README.md)
- [API Reference](./08-API_REFERENCE/README.md)

### Community
- [GitHub Issues](https://github.com/omnisystem/omnisystem/issues)
- [Discussions](https://github.com/omnisystem/omnisystem/discussions)
- [Discord Community](https://discord.gg/omnisystem)

### Troubleshooting
- [FAQ](./15-FAQ.md)
- [Troubleshooting Guide](./10-DEPLOYMENT/TROUBLESHOOTING.md)
- [Common Issues](./15-FAQ.md#common-issues)

---

**Ready to build? Let's go!** 🚀
