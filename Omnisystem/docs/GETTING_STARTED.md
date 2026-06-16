# Getting Started with Omnisystem
## Your First Steps into the Next-Generation Language Ecosystem

---

## Welcome!

You're about to explore the **Omnisystem**, a unified ecosystem of 7 production-ready programming languages designed to handle ANY challenge for the next 100+ years.

This guide will help you:
1. Understand what Omnisystem offers
2. Choose the right language for your needs
3. Write your first program
4. Compile and run it
5. Explore advanced features

---

## What is Omnisystem?

**Omnisystem** is not a single language. It's a complete ecosystem of **7 complementary languages**:

| Language | Purpose | Best For |
|----------|---------|----------|
| **TITAN** | Systems Programming | OS, servers, performance-critical code |
| **SYLVA** | Data Science & AI | ML, neural networks, data analysis |
| **AETHER** | Distributed Systems | Microservices, cloud, consensus |
| **AXIOM** | Formal Verification | Safety-critical, proofs, verification |
| **HELIX** | Game Development | Games, graphics, 3D, physics |
| **VERA** | Web Development | Web apps, PWAs, frontend |
| **NEXUS** | Mobile & IoT | Mobile apps, IoT, wearables |

All languages:
- ✅ Compile to native executables (Windows/Linux/macOS)
- ✅ Have 1,000-3,000+ functions in their standard library
- ✅ Seamlessly integrate via 140+ bridge functions
- ✅ Include quantum computing, blockchain, and AI/ML support
- ✅ Are production-ready and fully tested

---

## Step 1: Choose Your Language

### Ask Yourself:

**"What do I want to build?"**

#### Building a Game?
→ Use **HELIX**
- 3D graphics with deferred rendering and ray tracing
- Complete physics engine
- Animation system
- VR/AR support
- [Learn HELIX →](LANGUAGES/HELIX.md)

#### Building a Web App?
→ Use **VERA**
- Reactive components
- State management
- Routing
- REST/GraphQL
- Progressive Web Apps
- [Learn VERA →](LANGUAGES/VERA.md)

#### Building an AI/ML System?
→ Use **SYLVA**
- Neural networks (CNN, RNN, Transformer)
- Computer vision
- NLP
- Reinforcement learning
- Distributed training
- [Learn SYLVA →](LANGUAGES/SYLVA.md)

#### Building a Microservice?
→ Use **AETHER**
- Service discovery
- Load balancing
- Consensus algorithms
- Message queues
- Event streaming
- [Learn AETHER →](LANGUAGES/AETHER.md)

#### Building a Mobile App?
→ Use **NEXUS**
- Cross-platform UI
- Sensor integration
- Camera, GPS, Bluetooth
- Local database
- On-device ML
- [Learn NEXUS →](LANGUAGES/NEXUS.md)

#### Building a Systems Tool?
→ Use **TITAN**
- Memory management
- Async/await
- Cryptography
- Quantum computing
- Blockchain primitives
- [Learn TITAN →](LANGUAGES/TITAN.md)

#### Building Safety-Critical Code?
→ Use **AXIOM**
- Theorem proving
- Model checking
- Program verification
- Formal proofs
- [Learn AXIOM →](LANGUAGES/AXIOM.md)

---

## Step 2: Install and Setup

### Requirements
- Windows 10+, Linux, or macOS
- Clang compiler OR Microsoft Visual Studio
- PowerShell 5.0+ (for build script)

### Installation

1. **Navigate to project root:**
```bash
cd Z:\Projects\Omnisystem
```

2. **Verify the build script exists:**
```bash
ls Build-Omnisystem.ps1
```

3. **You're ready to go!**

---

## Step 3: Write Your First Program

### TITAN Example - Hello World

Create `hello.ti`:
```titan
fn main() {
    println!("Hello, Omnisystem!");
}
```

### SYLVA Example - Simple ML

Create `ml_example.sv`:
```sylva
fn main() {
    // Create a neural network
    let model = create_neural_network(input_size: 10, hidden_size: 128, output_size: 1);
    
    // Train it (pseudo-code)
    let loss = train(model, training_data);
    
    println!("Training loss: {}", loss);
}
```

### VERA Example - Web Component

Create `component.vr`:
```vera
pub component Counter {
    let count = reactive(0);
    
    fn increment() {
        count.value += 1;
    }
    
    render() {
        return html! {
            <div>
                <p>Count: {count}</p>
                <button on:click={increment}>+</button>
            </div>
        }
    }
}
```

### AETHER Example - Microservice

Create `service.ae`:
```aether
pub fn create_service(name: String, port: i32) {
    let registry = ServiceRegistry::new();
    let service = Service { name, port };
    
    registry.register(service);
    service.start_server();
}
```

### HELIX Example - Game Loop

Create `game.hlx`:
```helix
pub fn game_loop() {
    let scene = Scene::new();
    let camera = Camera::new(position: Vec3(0, 5, 10));
    
    loop {
        input::poll_events();
        update_physics(scene, delta_time);
        render(scene, camera);
    }
}
```

### NEXUS Example - Mobile App

Create `app.nx`:
```nexus
pub activity MainActivity {
    fn on_create() {
        set_content_view(R.layout.main);
        
        let camera = Camera::new();
        let button = findViewById(R.id.take_photo);
        button.set_on_click_listener(|| {
            camera.take_photo();
        });
    }
}
```

### AXIOM Example - Formal Proof

Create `proof.ax`:
```axiom
theorem commutativity_of_addition : ∀ a b : ℕ, a + b = b + a := by
    intro a b
    induction a with
    | zero => simp
    | succ k ih => simp [ih]
```

---

## Step 4: Compile Your Program

### Using the Build Script

```bash
# Navigate to project root
cd Z:\Projects\Omnisystem

# Run the build script
.\Build-Omnisystem.ps1

# Optionally launch the executable
.\Build-Omnisystem.ps1 -Launch
```

The build process:
1. Detects your language (by file extension)
2. Runs the language-specific lexer/parser
3. Generates C99 code
4. Compiles with Clang or MSVC
5. Creates native executable in project root

### Compilation Success

If everything works, you'll see:
```
OMNISYSTEM BUILD SCRIPT - TITAN COMPILER

Step 1: TITAN Source Code Compilation Pipeline
Reading TITAN source: your_program.ti
Generating C source code...
✓ C source code generated: Omnisystem.c

Step 2: Compiling C to Windows Executable
Using Clang compiler...
✓ Compiled to Windows executable: Omnisystem.exe (7.7 MB)

SUCCESS: OMNISYSTEM BUILD COMPLETE
Location: Z:\Projects\Omnisystem\Omnisystem.exe
```

---

## Step 5: Run Your Program

### Execute the Compiled Binary

```bash
# Run from command line
Z:\Projects\Omnisystem\Omnisystem.exe

# Or run from PowerShell
.\Omnisystem.exe
```

### Expected Output

For the Hello World example:
```
Hello, Omnisystem!
```

---

## Step 6: Explore the Standard Library

Each language has 1,000-3,000+ built-in functions. Common categories:

### TITAN Standard Library
- String processing (80+ functions)
- Mathematical operations (165+ functions)
- Cryptography (160+ functions)
- File I/O (150+ functions)
- Networking (250+ functions)
- Concurrency (200+ functions)

[TITAN Standard Library Reference →](LANGUAGES/TITAN.md#standard-library)

### SYLVA Standard Library
- Neural networks (400+ functions)
- Data processing (200+ functions)
- NLP (300+ functions)
- Computer vision (200+ functions)
- Reinforcement learning (150+ functions)

[SYLVA Standard Library Reference →](LANGUAGES/SYLVA.md#standard-library)

### VERA Standard Library
- Components (300+ functions)
- State management (150+ functions)
- HTTP client (150+ functions)
- Routing (120+ functions)
- Forms (100+ functions)

[VERA Standard Library Reference →](LANGUAGES/VERA.md#standard-library)

And so on for each language...

---

## Step 7: Use Advanced Features

### Quantum Computing

Any language can use quantum:
```titan
// In TITAN
let circuit = QuantumCircuit::new(3);
circuit.add_gate(hadamard(0));
circuit.add_gate(cnot(0, 1));
circuit.add_measurement(0..3);

let result = execute(circuit);
```

[Learn Quantum Computing →](ADVANCED_FEATURES.md#quantum-computing)

### Blockchain

Any language can deploy smart contracts:
```titan
// In TITAN
let contract = SmartContract::new(bytecode);
let tx = deploy(contract, deployer_address);
```

[Learn Blockchain →](ADVANCED_FEATURES.md#blockchain)

### AI/ML

Train neural networks in any language:
```sylva
// In SYLVA
let model = create_transformer(vocab_size: 50000, d_model: 768);
let optimizer = adam(learning_rate: 0.0001);
train(model, dataset, optimizer, epochs: 100);
```

[Learn AI/ML →](ADVANCED_FEATURES.md#ai-ml)

---

## Step 8: Integrate Multiple Languages

Use **bridge functions** to call one language from another:

```titan
// In TITAN
let model = sylva::load_model("model.bin");
let result = model.predict(input_data);

// Result is automatically converted between TITAN and SYLVA types
```

[Learn Cross-Language Integration →](BRIDGES.md)

---

## Common Tasks

### Create a New Project
```bash
mkdir my_project
cd my_project
# Create your source file (e.g., main.ti for TITAN)
```

### Build and Run
```bash
cd Z:\Projects\Omnisystem
.\Build-Omnisystem.ps1 -Launch
```

### Use Standard Library
```titan
// TITAN example
use std::string::{strlen, substr};
use std::io::{println, readln};
use std::crypto::{sha256};

fn main() {
    let s = "hello";
    println!("Length: {}", strlen(s));
    let hash = sha256(s.as_bytes());
}
```

### Work with Files
```titan
// TITAN file I/O
let file = File::open("data.txt", "r");
let content = file.read_all();
file.close();
```

### Make HTTP Requests
```vera
// VERA HTTP client
let client = HttpClient::new("https://api.example.com");
let response = client.get("/data");
println!("{}", response.body);
```

### Train a Neural Network
```sylva
// SYLVA ML training
let model = create_dense_layer(10, 128);
let optimizer = adam(0.001);
let loss_fn = CrossEntropy();

for epoch in 0..100 {
    for batch in data.batches(32) {
        let output = forward(model, batch);
        let loss = loss_fn(output, batch.labels);
        backward(loss);
        step_optimizer(optimizer);
    }
}
```

---

## Next Steps

1. **Choose a Language** — Pick one based on your use case
2. **Read the Language Guide** — Learn the syntax and features
3. **Study Examples** — See real code in action
4. **Build a Project** — Create something meaningful
5. **Explore Advanced Features** — Use quantum, blockchain, or AI/ML
6. **Integrate Languages** — Use bridge functions for multi-language projects

---

## Troubleshooting

### "Compiler not found"
Solution: Install Clang or Visual Studio

### "Build script fails"
Solution: Make sure you're in the project root directory and have PowerShell 5.0+

### "Executable won't run"
Solution: Check that you're using the correct executable path and your system is supported

[See Troubleshooting Guide →](TROUBLESHOOTING.md)

---

## Learning Resources

- **Language Guides** — [LANGUAGES.md](LANGUAGES.md)
- **Code Examples** — [EXAMPLES.md](EXAMPLES.md)
- **API Reference** — [API_REFERENCE.md](API_REFERENCE.md)
- **Advanced Features** — [ADVANCED_FEATURES.md](ADVANCED_FEATURES.md)
- **FAQ** — [FAQ.md](FAQ.md)

---

## Summary

You now know:
✅ What Omnisystem is  
✅ How to choose a language  
✅ How to write your first program  
✅ How to compile and run code  
✅ Where to find function references  
✅ How to use advanced features  

**Ready to dive deeper?** Pick a language and [go to its guide!](LANGUAGES.md)

🌟 **Welcome to Omnisystem — Building the Future, One Language at a Time**
