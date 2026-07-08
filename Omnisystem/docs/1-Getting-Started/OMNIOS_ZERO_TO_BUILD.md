# OmniOS — Zero to Building Real Software

This guide is for someone who has never used OmniOS before. By the end of this page, you will have built and run a real OmniOS program.

---

## What is OmniOS?

OmniOS is an operating system and development platform built on seven purpose-built programming languages called the Omni-Languages. Every part of OmniOS — the window manager, the compiler, the package manager, the runtime — is written in one of these seven languages.

You can use OmniOS as:
- A VS Code extension (the primary experience — start here)
- A standalone desktop application (Tauri-based, no VS Code needed)
- A containerized environment (for servers and CI/CD)
- A virtual machine (for isolated environments)
- In a browser tab (zero install, try it instantly)

---

## The Seven Omni-Languages

Each language has a specific domain. You do not need to learn all seven to get started — pick the one that matches what you want to build.

| Language | File Extension | Purpose | Start Here If You Want To... |
|---|---|---|---|
| **Titan** | `.titan` | Systems, core logic, CLI tools, data structures | Build anything — this is the foundation |
| **Vera** | `.vera` | Desktop UI, forms, dashboards, visual apps | Build a desktop application with windows and buttons |
| **Helix** | `.helix` | GPU shaders, 3D graphics, visual effects | Build a game or graphics application |
| **Aether** | `.aether` | Concurrent systems, actors, servers, async I/O | Build a web server, API, or distributed system |
| **Axiom** | `.axiom` | Formal verification, proofs, safety guarantees | Prove your code is correct mathematically |
| **Sylva** | `.sylva` | Machine learning, neural networks, data pipelines | Build and train an ML model |
| **Nexus** | `.nexus` | Responsive layouts, breakpoints, design systems | Define how your Vera UI adapts to different screen sizes |

---

## Step 1: Open the OmniOS Desktop

In VS Code, press `Ctrl+Shift+P` and type:
```
OmniOS: Launch Desktop
```

Press Enter. The OmniOS Desktop opens as a panel.

---

## Step 2: The Welcome Screen (First Time Only)

On first launch, you will see the Welcome screen. Choose what you want to build:

- **Desktop Application** — A windowed app with UI components
- **Web Server** — An HTTP server or REST API
- **ML Pipeline** — A machine learning model and training loop
- **Convert Existing Code** — Bring your JavaScript, Python, Rust, or other code to OmniOS
- **Explore OmniOS** — Open the full desktop environment

Choose one. OmniOS will create the project files and open the right tools automatically.

---

## Step 3: Your First Build

After the project is created, the **OmniCC Build** app opens automatically.

Click the **▶ Build** button.

You will see the build output stream in real time. For a starter project, the build completes in under 2 seconds:

```
[parse]   Parsing src/main.titan (12 LOC)
[resolve] Symbol resolution complete (8 symbols)
[type]    Type checking passed
[lower]   Lowering to IR (24 instructions)
[opt]     Optimizations applied (-3 instructions)
[codegen] Generating x86_64-windows target...
[link]    Linking complete

✓ Build complete: target/x86_64-windows/main.exe (48 KB)
```

All seven build phases will turn green. Your program is built.

---

## Step 4: Run Your Program

Click **⚡ Run** in the Build app. The Terminal app opens and runs your program:

```
OmniOS Terminal v2.0

Z:\Projects\myapp> omnicc run
Hello, OmniOS!
```

You have just written, compiled, and run your first OmniOS program.

---

## Understanding Your Project Files

After scaffolding, your project contains:

```
myapp/
├── BUILD.omnisystem      Project configuration and dependencies
├── src/
│   └── main.titan        Your main program entry point
└── target/               Build output (created by OmniCC)
    └── x86_64-windows/
        └── main.exe
```

**`BUILD.omnisystem`** tells OmniCC about your project:
```
project "myapp" {
    version: "0.1.0",
    languages: [titan],
    target: x86_64-windows,
    entry: "src/main.titan"
}

dependencies {}
```

**`src/main.titan`** is your starting code:
```titan
module Main;

fn main() -> i32 {
    log("Hello, OmniOS!");
    return 0;
}
```

---

## The Desktop Apps

The OmniOS Desktop has ten built-in apps. Click any icon on the desktop to open it.

| App | Icon | What It Does |
|---|---|---|
| Files | 📁 | Browse and manage your project files |
| Terminal | 💻 | Real command-line terminal with full shell access |
| Code Studio | ✨ | Create new files, scaffold projects, run quick actions |
| OmniCC Build | ⚙️ | Compile, run, test, and watch your project |
| Bonsai Hub | 🌿 | Launch and manage the Bonsai app ecosystem |
| ML Studio | 🧠 | Design and train machine learning models |
| OmniPM | 📦 | Install and manage packages |
| App Converter | 🔄 | Convert existing code to OmniOS |
| Settings | ⚙ | Configure OmniOS and the VS Code extension |
| System Monitor | 📊 | View runtime health and resource usage |

---

## Getting Help

**In any app**: press `?` to open the contextual help panel for that app.

**For error messages**: every error in the Build app has two buttons:
- 🔧 **Fix it** — OmniCC suggests and applies the fix automatically
- ℹ️ **Explain it** — Plain English explanation of what went wrong and why

**For the language reference**: see the docs in `Omnisystem/docs/4-Languages/`.

**Community**: file issues and ask questions at the Omnisystem GitHub repository.

---

## Next Steps

Once you have run your first build, explore these topics:

- [Titan Language Guide](../4-Languages/TITAN_LANGUAGE_GUIDE.md) — Learn Titan's syntax, types, and standard library
- [Vera UI Guide](../4-Languages/VERA_LANGUAGE_GUIDE.md) — Build a desktop UI with Vera components
- [Aether Concurrency Guide](../4-Languages/AETHER_LANGUAGE_GUIDE.md) — Build servers and concurrent systems
- [OmniPM Package Guide](../5-Core-Systems/OMNIPM.md) — Add packages to your project
- [App Converter Guide](../5-Core-Systems/OMNIOS_APPS.md) — Convert your existing code
- [Build System Reference](../6-APIs/BUILD_SYSTEM.md) — All OmniCC build options
