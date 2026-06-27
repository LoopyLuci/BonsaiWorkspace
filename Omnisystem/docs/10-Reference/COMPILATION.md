# Compilation & Deployment Guide
## Build and Deploy Omnisystem Programs

---

## Overview

The Omnisystem compilation pipeline is **unified and simple**:

```
Your Source Code
    ↓
Language-Specific Compiler (Lexer → Parser → Code Generator)
    ↓
C99 Intermediate Code
    ↓
C Compiler (Clang or MSVC)
    ↓
Native Executable (Windows/Linux/macOS)
```

---

## Quick Start

### Step 1: Create Your Program

Create `hello.ti`:
```titan
fn main() {
    println!("Hello, Omnisystem!");
}
```

Or `main.sv` for SYLVA, `main.vr` for VERA, etc.

### Step 2: Build

```bash
cd Z:\Projects\Omnisystem
.\Build-Omnisystem.ps1
```

### Step 3: Run

```bash
.\Omnisystem.exe
```

That's it! Your program is now a native Windows executable.

---

## Compilation Process

### Phase 1: Language-Specific Compilation

Each language has a **complete compiler** that:

1. **Lexical Analysis** — Tokenizes source code
2. **Parsing** — Builds Abstract Syntax Tree (AST)
3. **Type Checking** — Verifies type safety
4. **Code Generation** — Produces C99 code

**Example for TITAN:**
```
Input:  hello.ti
Output: Omnisystem.c (with C code that implements the TITAN program)
```

### Phase 2: C Compilation

The generated C99 code is compiled by:
- **Clang** (LLVM) — Preferred, faster
- **MSVC** (Visual Studio) — Fallback, fully compatible

```bash
# Clang
clang -o Omnisystem.exe Omnisystem.c -std=c99 -O3

# MSVC
cl /Fe:Omnisystem.exe Omnisystem.c /O2
```

### Phase 3: Linking & Optimization

The compiled object files are linked and optimized:
- Standard C library linking
- Optimization passes (if `-O3` specified)
- Platform-specific optimizations

---

## Build Options

### Using the Build Script

```bash
# Basic build
.\Build-Omnisystem.ps1

# Build and launch
.\Build-Omnisystem.ps1 -Launch

# Custom source file
$env:SOURCE_FILE = "my_program.ti"
.\Build-Omnisystem.ps1
```

### Advanced Build Options

**From command line** (modify script or environment):

```powershell
# Custom output name
$OUTPUT_NAME = "my_app.exe"

# Optimization level
$OPTIMIZE = "-O3"  # Options: -O0 (none), -O1, -O2, -O3 (max)

# Debug symbols
$DEBUG_SYMBOLS = "-g"  # Include for debugging
```

---

## Compilation Targets

### Windows (Default)

```bash
# Standard Windows PE executable
.\Build-Omnisystem.ps1

# Output: Omnisystem.exe (Windows x86-64)
```

### Linux

```bash
# On Linux, use equivalent build command
clang -o omnisystem main.ti.c -std=c99 -O3

# Output: omnisystem (Linux ELF executable)
```

### macOS

```bash
# On macOS
clang -o omnisystem main.ti.c -std=c99 -O3

# Output: omnisystem (macOS Mach-O executable)
```

---

## Performance Optimization

### Compiler Optimization Levels

```c
// Build with different optimization levels:

// No optimization (fastest compile, slowest runtime)
clang -O0 main.c

// Level 1 optimization
clang -O1 main.c

// Level 2 optimization (balanced)
clang -O2 main.c

// Level 3 optimization (slowest compile, fastest runtime)
clang -O3 main.c

// Size optimization
clang -Os main.c

// Aggressive size optimization
clang -Oz main.c
```

### Recommended Settings

**For Development:**
```bash
# Fast compilation, easier debugging
clang -O0 -g program.c
```

**For Production:**
```bash
# Optimal performance
clang -O3 -march=native program.c
```

### Language-Specific Optimizations

**TITAN:**
```
- Link-time optimization (LTO)
- Function inlining
- Vector code generation (SIMD)
```

**SYLVA:**
```
- Automatic vectorization
- Cache optimization
- GPU kernel fusion
```

**HELIX:**
```
- Loop unrolling
- Inline assembly for hot paths
- CPU target selection
```

---

## Debugging

### With Debug Symbols

```bash
# Build with debug symbols
clang -g -O0 program.c -o program

# Run under debugger
gdb ./program
```

### Using the Built-in Debugger

```titan
use std::debug::*;

fn main() {
    set_breakpoint("line 10");  // Set breakpoint
    let value = compute();
    print_call_stack();         // Print stack trace
}
```

### Time-Travel Debugging

```titan
// Record execution
let session = start_recording();
run_program();
let recording = stop_recording();

// Replay and inspect
let debugger = load_recording(recording);
debugger.jump_to_line(42);
debugger.step_backwards();
```

---

## Deployment

### Single File Distribution

```bash
# Your executable can be distributed as-is
# No runtime required, no dependencies
cp Omnisystem.exe ~/projects/my_app/

# User can run directly
./my_app.exe
```

### Platform-Specific Packaging

**Windows:**
```bash
# Create installer
# Can use NSIS, WiX, or similar
```

**Linux:**
```bash
# Create .deb package
fpm -s dir -t deb -n myapp -v 1.0 -C /tmp/build myapp.exe
```

**macOS:**
```bash
# Create .app bundle and .dmg
mkdir -p MyApp.app/Contents/MacOS
cp omnisystem MyApp.app/Contents/MacOS/
```

### Docker Deployment

```dockerfile
FROM ubuntu:20.04

# Copy compiled executable
COPY Omnisystem /app/omnisystem

WORKDIR /app
CMD ["./omnisystem"]
```

### Cloud Deployment

**AWS Lambda:**
```bash
# Omnisystem executables can run on Lambda custom runtimes
zip lambda.zip omnisystem
aws lambda create-function --runtime provided.al2 ...
```

**Google Cloud Run:**
```dockerfile
FROM gcr.io/distroless/base
COPY omnisystem /omnisystem
CMD ["/omnisystem"]
```

**Kubernetes:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: omnisystem-app
spec:
  containers:
  - name: app
    image: myregistry/omnisystem:latest
```

---

## Common Build Issues

### Issue: Compiler Not Found

**Solution:**
```bash
# Install Clang
# On Windows: Install Visual Studio or Clang for Windows
# On Linux: sudo apt-get install clang
# On macOS: xcode-select --install
```

### Issue: Build Script Permission Denied

**Solution (Windows PowerShell):**
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Issue: Out of Memory During Compilation

**Solution:**
- Reduce optimization level: `-O1` instead of `-O3`
- Build on a machine with more RAM
- Use `-flto=thin` instead of `-flto` for link-time optimization

### Issue: Symbol Collision

**Solution:**
```c
// Use namespaces to avoid collisions
// In C99, use unique prefixes:
// myapp_function() instead of function()
```

---

## Advanced Topics

### Link-Time Optimization (LTO)

```bash
# Enable LTO for maximum optimization
clang -flto=thin program.c -o program
```

### Static Linking

```bash
# Create fully self-contained executable
clang -static program.c -o program
```

### Cross-Compilation

```bash
# Compile for different target
clang --target=aarch64-linux-gnu program.c -o program_arm64
```

### Custom Flags

```bash
# CPU-specific optimizations
clang -O3 -march=native program.c

# SIMD instructions
clang -O3 -mavx2 program.c

# Profile-guided optimization
clang -O3 -fprofile-generate program.c -o program_prof
./program_prof  # Run to generate profile
clang -O3 -fprofile-use program.c -o program_final
```

---

## Build Troubleshooting

### Getting Build Details

```powershell
# Verbose build output
$env:VERBOSE = "1"
.\Build-Omnisystem.ps1

# Save compiler output
clang -v program.c 2>&1 | Out-File build.log
```

### Checking Generated C Code

```powershell
# Look at generated C code
cat Omnisystem.c | head -50

# Search for specific function
grep -n "my_function" Omnisystem.c
```

### Profile the Build

```bash
# Time the compilation
time clang -O3 program.c -o program

# See what's slow
clang -ftime-trace program.c
# View timeline.json in Chrome
```

---

## Next Steps

- **[Language Guides](LANGUAGES.md)** — Language-specific build details
- **[Getting Started](GETTING_STARTED.md)** — First program walkthrough
- **[Examples](EXAMPLES.md)** — Real compilable code

---

**🚀 Build Fast. Deploy Anywhere. Run Everywhere.**
