# Omnisystem Self-Hosting Bootstrap Guide

## Quick Start: Install Rust and Build the Seed Compiler

### Prerequisites (Windows 10/11)

1. **Install Visual Studio Build Tools 2022**
   - Download: https://visualstudio.microsoft.com/downloads/
   - Select: "Desktop development with C++"
   - This provides MSVC compiler and Windows SDK

2. **Install Rust**
   ```powershell
   # Download rustup installer
   # https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe
   
   # Or via winget:
   winget install Rustlang.Rust.MSVC
   
   # Verify installation
   rustc --version
   cargo --version
   ```

### Step 1: Build the Rust Seed Compiler (Cranelift Backend)

```powershell
cd z:\Projects\Omnisystem\titan-bootstrap
cargo build --release
```

**Expected output:**
```
   Compiling titan-bootstrap v0.2.0 (...)
    Finished release [optimized] target(s) in X.XXs
```

**Troubleshooting:**
- If build fails with "could not compile Cranelift":
  - Ensure Visual Studio Build Tools are installed
  - Run: `cargo clean && cargo build --release` again
  - Alternative: Use WSL2 with Linux build (no MSVC needed)

### Step 2: Test the Seed Compiler

Create test program `test_basic.ti`:
```rust
fn main() -> i64 {
    return 42;
}
```

Compile and run:
```powershell
cd z:\Projects\Omnisystem\titan-bootstrap
cargo run --release -- ..\test_basic.ti --run --verbose
```

**Expected output:**
```
Titan Bootstrap Compiler v0.2.0 (Cranelift backend)
Source: ..\test_basic.ti
  Lex: 12 tokens
  Parse: 1 functions
  Borrow Check: ok
  Codegen: complete
Result: 42
```

### Step 3: Verify Bootstrap Correctness

Once the seed compiler works, verify it produces bit-identical output by:

```powershell
# Create a simple Titan program
echo 'fn add(a: i64, b: i64) -> i64 { return a + b; }' > test.ti

# Compile with seed
cargo run --release -- test.ti -o seed_output.bin

# Recompile the source
# (Once Titan compiler is written, compile again with Titan compiler)
# cargo run --release -- test.ti -o titan_output.bin

# Compare checksums
certutil -hashfile seed_output.bin SHA256
certutil -hashfile titan_output.bin SHA256

# Should be identical
```

---

## Architecture Overview

The self-hosting chain follows this path:

```
┌─────────────────────────────────────────────────────────┐
│  PHASE 1: Rust Seed Bootstrap (YOU ARE HERE)            │
│  ✓ Rust seed compiler (Cranelift backend) — compiles Ti│
│  • Produces: Titan object files                         │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│  PHASE 2: Port Compiler to Titan                        │
│  • Write: Titan lexer, parser, borrow checker, codegen  │
│  • Compile: Each .ti file with Rust seed               │
│  • Verify: Bit-identical output                         │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│  PHASE 3: Retire Rust Seed (FULLY SELF-HOSTED)         │
│  • Delete: Rust seed (never needed again)              │
│  • Source: Titan compiler compiles itself              │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│  PHASE 4: Build Runtime Layers                          │
│  • OmniCore: Task scheduling, capabilities, telemetry  │
│  • Aether: Actor runtime, supervision, CRDT            │
│  • Sylva: REPL, time-travel debugger, IDE              │
│  • Axiom: Dependent types, formal verification         │
└─────────────────────────────────────────────────────────┘
```

---

## File Organization

```
z:\Projects\Omnisystem\
├── titan-bootstrap/              [STEP 1: Rust seed compiler]
│   ├── Cargo.toml                (Cranelift dependencies)
│   ├── src/
│   │   ├── main.rs               (Entry point, CLI parsing)
│   │   ├── lexer.rs              (Tokenizer)
│   │   ├── parser.rs             (Recursive descent parser)
│   │   ├── ast.rs                (AST definitions)
│   │   ├── borrow_checker.rs     (Lifetime analysis)
│   │   ├── codegen_cranelift.rs  (Cranelift code generator)
│   │   └── error.rs              (Error types)
│   └── target/release/           (Compiled seed compiler)
│
├── titan/                        [STEP 2-3: Self-hosted compiler]
│   ├── compiler/
│   │   ├── lexer.ti              (Titan lexer — compiles with seed)
│   │   ├── parser.ti             (Titan parser — compiles with seed)
│   │   ├── borrow_checker.ti     (Titan borrow checker)
│   │   ├── codegen.ti            (Titan codegen)
│   │   └── main.ti               (Titan compiler entry point)
│   ├── stdlib/                   (Standard library)
│   └── seed/                     (Pre-compiled seed output)
│
├── omnicore/                     [STEP 2: OmniCore interpreter]
│   ├── src/
│   │   ├── lib.rs                (UniIR executor, capabilities, telemetry)
│   │   ├── interpreter.rs        (SSA instruction dispatch)
│   │   └── tests.rs              (Unit tests)
│   └── Cargo.toml
│
├── aether/                       [STEP 4: Actor runtime]
│   ├── src/
│   │   ├── lib.rs                (Actor spawning, mailbox, supervision)
│   │   ├── crdt.rs               (GCounter, vector clocks)
│   │   └── tests.rs
│   └── Cargo.toml
│
├── sylva/                        [STEP 5: Interactive frontend]
│   ├── src/
│   │   ├── lib.rs                (Expression evaluator, REPL)
│   │   ├── debugger.rs           (Time-travel debugging)
│   │   └── tests.rs
│   └── Cargo.toml
│
└── axiom/                        [STEP 6: Proof kernel]
    ├── src/
    │   ├── lib.rs                (Type checker, normalizer)
    │   └── tests.rs
    └── Cargo.toml
```

---

## Build Commands Reference

```powershell
# Step 1: Build Rust seed (one time)
cd z:\Projects\Omnisystem\titan-bootstrap
cargo build --release

# Step 2: Compile Titan source with seed
./target/release/titan-bootstrap.exe ../titan/compiler/lexer.ti --run --verbose

# Step 3: Build OmniCore Rust interpreter
cd z:\Projects\Omnisystem\omnicore
cargo build --release

# Step 4-6: Compile Aether, Sylva, Axiom with Titan compiler
./target/release/titan-bootstrap.exe ../aether/src/runtime.ti --run
./target/release/titan-bootstrap.exe ../sylva/src/repl.sy --run
./target/release/titan-bootstrap.exe ../axiom/src/kernel.ax --run
```

---

## Next Steps

Once the seed compiler builds successfully:

1. **Run the test suite**
   ```powershell
   cd z:\Projects\Omnisystem\titan-bootstrap
   cargo test --release
   ```

2. **Create integration tests** to verify output correctness

3. **Begin Step 2**: Implement OmniCore Rust interpreter (minimal SSA executor)

4. **Begin Step 3**: Port Titan compiler to Titan (starting with lexer, then parser, etc.)

---

## Troubleshooting

**Q: Build fails with "LLVM version mismatch"**
A: The Cranelift backend doesn't use LLVM. This error shouldn't occur. If it does:
   - `cargo clean`
   - `cargo update`
   - `cargo build --release`

**Q: "could not find native static library cranelift"**
A: Cranelift is pure Rust, no native libraries needed. Check Cargo.toml has correct version (0.106).

**Q: Compilation is very slow**
A: First compile is expected to be slow. Try:
   - Compile once with `--release` flag
   - Subsequent compiles are faster
   - On Windows MSVC, initial link may take 30-60 seconds

**Q: Can I use WSL2 instead?**
A: Yes! The build is identical on WSL2 Linux:
   ```bash
   cd /mnt/z/Projects/Omnisystem/titan-bootstrap
   cargo build --release
   ```
   This avoids any MSVC/Windows-specific issues.

---

## References

- **Cranelift Documentation**: https://docs.rs/cranelift-codegen/
- **Rust Compiler Book**: https://rustc-dev-guide.rust-lang.org/
- **LLVM Removed**: We use Cranelift instead of LLVM for cross-platform compatibility
- **Bootstrap Philosophy**: https://en.wikipedia.org/wiki/Bootstrapping_(compilers)

---

## Success Criteria

- ✓ `cargo build --release` completes without errors
- ✓ Seed compiler runs: `./target/release/titan-bootstrap test.ti --run --verbose`
- ✓ Seed produces output for test programs
- ✓ Borrow checker detects errors correctly
- ✓ Cranelift JIT executes code and produces correct results

Once all criteria are met, move to Step 2: Building OmniCore interpreter.
