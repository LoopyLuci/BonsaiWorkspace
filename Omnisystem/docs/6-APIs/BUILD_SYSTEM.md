# OmniCC Build System Reference

## Overview

OmniCC is the unified compiler and build orchestrator for all seven Omni-Languages. It handles parsing, type checking, optimization, code generation, linking, package management, testing, formatting, documentation generation, and formal verification — all from a single tool.

---

## `BUILD.omnisystem` — Project Manifest

Every OmniOS project requires a `BUILD.omnisystem` file at the project root.

```
project "myapp" {
    version: "1.0.0",
    description: "My OmniOS application",
    authors: ["Your Name <your@email.com>"],
    license: "MIT",

    // Languages used in this project
    languages: [titan, vera, aether],

    // Default build target
    target: x86_64-windows,

    // Default optimization level
    opt: O2,

    // Entry point (for executables)
    entry: "src/main.titan",

    // Source directories
    sources: ["src/**/*.titan", "src/**/*.vera", "src/**/*.aether"],

    // Output directory
    out: "target"
}

dependencies {
    omni-http: "^1.2.0",
    omni-json: "^2.0.0"
}

[dev-dependencies]
    omni-testing: "^1.0.0"

[build-targets]
    release {
        opt: O3,
        lto: true,
        strip: true
    }
    debug {
        opt: O0,
        debug-info: true
    }
    wasm {
        target: wasm32,
        opt: O2
    }
```

---

## Command Reference

### `omnicc build`

Compile the project.

```
omnicc build [OPTIONS]

Options:
  --target <triple>    Build target (default: from BUILD.omnisystem)
                       Values: x86_64-windows, x86_64-linux, aarch64-linux,
                               aarch64-macos, wasm32, wasm32-wasi
  --opt <level>        Optimization level (default: O2)
                       Values: O0 (none), O1 (basic), O2 (balanced), O3 (full)
  --release            Shorthand for --opt O3 --lto --strip
  --debug              Shorthand for --opt O0 --debug-info
  --jobs <n>           Number of parallel frontend workers (default: CPU count)
  --no-cache           Disable incremental build cache
  --fmt-only           Format source files, do not compile
  --check-only         Type-check only, do not produce output
  --check              Alias for --check-only
  --doc                Generate HTML documentation to target/doc/
  --verify-axiom       Run Axiom formal verification after type checking
  --watch              Watch mode: rebuild on file change (implies O0)
  --system             Run compiler self-diagnostics
  --verbose            Show all compiler phases and timings
  --quiet              Suppress all output except errors
```

**Exit codes:**
- `0` — success
- `1` — compilation errors
- `2` — internal compiler error (please report)
- `3` — configuration error (bad BUILD.omnisystem)

---

### `omnicc run`

Build and run the project executable.

```
omnicc run [OPTIONS] [-- ARGS...]

Options:
  All options from `omnicc build`
  -- ARGS    Arguments to pass to the compiled program
```

If the project targets `wasm32`, the program runs inside the OmniCC WASM runtime.

---

### `omnicc test`

Build and run all tests.

```
omnicc test [OPTIONS] [FILTER]

Options:
  --verbose    Show output from passing tests (default: only failures)
  --nocapture  Do not capture stdout from tests
  --filter <pattern>  Only run tests matching the pattern
  FILTER       Shorthand for --filter
```

**Test format in Titan:**
```titan
module MyTests;

#[test]
fn test_addition() {
    assert_eq(add(2, 3), 5);
}

#[test]
fn test_empty_list() {
    let list: Vec<i32> = Vec::new();
    assert_eq(list.len(), 0);
}
```

**Output:**
```
Running 12 tests in 3 suites

suite: MyTests
  ✓ test_addition         (0.3ms)
  ✓ test_empty_list       (0.1ms)
  ✗ test_overflow         (0.2ms) FAILED
    Expected: 255
    Actual:   0
    at src/math.titan:42

Results: 11 passed, 1 failed, 0 skipped (2.1ms total)
```

---

### `omnicc clean`

Remove all build artifacts.

```
omnicc clean [OPTIONS]

Options:
  --target <triple>  Clean only artifacts for this target
  --cache            Also clear the incremental build cache
```

---

### `omnicc fmt`

Format source files according to the canonical style guide.

```
omnicc fmt [OPTIONS] [FILES...]

Options:
  --all        Format all source files in the project
  --check      Report unformatted files without modifying them (for CI)
  --diff       Show a diff of changes without applying them
  FILES        Specific files to format (default: all in sources)
```

**Titan formatting rules:**
- 4-space indentation
- Space after keywords: `if (`, `while (`, `for (`
- Space around binary operators: `a + b`, not `a+b`
- Opening brace on same line as declaration
- Maximum 100 columns
- Trailing newline

---

### `omnicc check`

Run type checking without producing any output files. Faster than a full build; ideal for editor integration.

```
omnicc check [OPTIONS]

Options:
  --all        Check all files, even those not changed since last check
  --system     Run system self-diagnostic checks
```

---

### `omnicc doc`

Generate HTML documentation from source file doc-comments.

```
omnicc doc [OPTIONS]

Options:
  --out <dir>    Output directory (default: target/doc/)
  --open         Open generated docs in browser after generation
  --private      Include private (unexported) items
```

**Doc-comment format in Titan:**
```titan
/// Computes the factorial of n.
///
/// Returns 1 for n = 0 (base case).
/// Panics if n > 20 (result exceeds i64::MAX).
///
/// Example:
/// ```
/// assert_eq(factorial(5), 120);
/// ```
fn factorial(n: i64) -> i64 {
    if n <= 1 { return 1; }
    return n * factorial(n - 1);
}
```

---

### `omnicc verify`

Run Axiom formal verification on `.axiom` theorem files.

```
omnicc verify [OPTIONS]

Options:
  --axiom          Run Axiom theorem proofs (default: true)
  --timeout <sec>  Max seconds per theorem proof (default: 30)
  --verbose        Show proof search steps
```

**Output:**
```
Verifying 8 theorems in ValidationLayer.axiom

  ✓ BoundsCheck         proved in 0.12s   (SMT: z3)
  ✓ NullSafety          proved in 0.08s   (SMT: z3)
  ✗ TerminationGuard    timeout after 30s (increase --timeout or simplify)
  ✓ TypeInvariant       proved in 0.34s   (SMT: z3)

Results: 7 proved, 1 timeout, 0 failed (2 pending manual review)
```

---

### `omnicc pm` (Package Manager)

```
omnicc pm <subcommand>

Subcommands:
  add <name>[@<version>]    Install a package
  remove <name>             Uninstall a package
  update [name]             Update one or all packages
  list                      List installed packages
  search <query>            Search the registry
  audit                     Check for vulnerabilities
  publish                   Publish a package to the registry
  login                     Authenticate with the registry
  logout                    Clear registry credentials
```

---

### `omnicc lsp`

Run the Language Server Protocol server. Used by the VS Code extension — not normally invoked directly.

```
omnicc lsp [OPTIONS]

Options:
  --stdio    Communicate over stdin/stdout (default)
  --port <n> Communicate over TCP port n
  --debug    Enable debug logging to stderr
```

---

### `omnicc runtime`

Run the OmniOS runtime server. Used by the VS Code extension — not normally invoked directly.

```
omnicc runtime [OPTIONS]

Options:
  --ipc         Expose JSON-RPC 2.0 IPC over stdin/stdout
  --port <n>    Expose IPC over TCP port n
  --cwd <path>  Working directory for file system operations
  --debug       Enable debug logging
```

---

## Incremental Builds

OmniCC uses content-based hashing for incremental builds. A file is recompiled only if:
1. Its source content has changed (FNV-1a hash stored in `target/.cache/hashes.json`)
2. Any of its imported modules have changed
3. The compiler version has changed
4. The build configuration has changed (target, opt level, flags)

The build cache is stored in `target/.cache/`. It is safe to delete (forces a full rebuild). The cache is never shared between different targets or opt levels.

---

## Build Phases (Detail)

### 1. Parse
- Reads all source files listed in `BUILD.omnisystem`
- Runs the language-appropriate parser for each file extension
- Produces an AST (Abstract Syntax Tree) for each file
- Errors: syntax errors with exact file, line, and column numbers

### 2. Resolve
- Builds the module dependency graph
- Resolves `import` statements to their source modules
- Detects circular dependencies (error) and unused imports (warning)
- Produces a global symbol table: all exported names and their types

### 3. Type
- Runs type inference bottom-up through all AST nodes
- Resolves generic types and protocol conformances
- Checks all function call argument types
- Errors: type mismatches, missing protocol implementations, undeclared variables

### 4. Lower
- Transforms each AST into a linear IR (Intermediate Representation)
- IR instructions: `Add`, `Sub`, `Mul`, `Div`, `Call`, `Return`, `Jump`, `JumpIf`, `Load`, `Store`, `Alloc`, `Free`, `Cast`
- Each IR instruction retains a pointer to its source AST node for error attribution

### 5. Opt (Optimization)
- **Dead code elimination**: removes IR instructions whose results are never used
- **Constant folding**: evaluates constant expressions at compile time
- **Inlining**: inlines small functions (< 20 instructions) at call sites
- **Common subexpression elimination**: deduplicates repeated computations
- Disabled at `O0`, partial at `O1`, full at `O2` and `O3`

### 6. Codegen
- Translates IR to the target machine code or bytecode
- Targets: x86-64 (Windows and Linux), AArch64 (macOS and Linux), WASM
- Register allocation: linear scan allocator
- Instruction selection: pattern matching against architecture-specific instruction templates

### 7. Link
- Combines all compiled object modules into a single output binary
- Resolves cross-module symbol references
- Applies dead code elimination at the link level (removes unused exported symbols)
- Writes the final binary: `.exe` (Windows), ELF (Linux), Mach-O (macOS), `.wasm`

---

## Error Message Format

All OmniCC errors follow a consistent format:

```
error[E0041]: type mismatch
 --> src/main.titan:42:8
  |
40 |     let count: i32 = 0;
41 |     let message: str = "hello";
42 |     count = message;
  |     ^^^^^ expected i32, found str
  |
help: if you meant to convert the string to an integer, use `message.parse::<i32>()?`
```

Fields:
- **Error code**: `E0001`–`E9999`, documented in the error code reference
- **Source location**: file, line, column
- **Context**: the relevant source lines with carets pointing at the problem
- **Help**: a concrete suggestion for how to fix the error (not always available)

---

## Supported Targets

| Triple | OS | Architecture | Status |
|---|---|---|---|
| `x86_64-windows` | Windows 10+ | x86-64 | Production |
| `x86_64-linux` | Linux (glibc 2.17+) | x86-64 | Production |
| `aarch64-linux` | Linux | AArch64 | Production |
| `aarch64-macos` | macOS 11+ | AArch64 (Apple Silicon) | Production |
| `x86_64-macos` | macOS 10.15+ | x86-64 | Maintained |
| `wasm32` | Browser / WASI | WASM | Production |
| `wasm32-wasi` | WASI runtimes | WASM | Experimental |
