# Universal Language Converter Framework (ULCF)

## Overview

The ULCF enables Omnisystem to work with and replace any programming language. All conversions go through the XAST (Cross-Language AST) → UniIR pivot. This means you can write code in any language and have it automatically translated to Titan, Aether, or Sylva with guaranteed semantic preservation.

## Supported Languages — Tier 1 (Production)

| Language | Converter | Fidelity | Bidirectional |
|----------|-----------|----------|---------------|
| C | c_to_titan | Certified | Yes |
| Rust | rust_frontend | High | Yes |
| JavaScript | javascript_frontend | High | Yes |
| TypeScript | javascript_frontend | High | Yes |
| Java | java_frontend | High | In Progress |
| Go | go_frontend | High | In Progress |
| Python | python_to_sylva | High | Yes |

## Supported Languages — Tier 2 (Planned)

C++, Swift, Kotlin, Kotlin/Native, C#, Ruby, PHP, Perl, Dart, Scala, OCaml, F#, Haskell, Elixir, Erlang/OTP, Clojure, Scheme, Lisp, R, Julia, SQL, COBOL, Fortran, Ada

## Architecture

```
Source Code → Language Frontend → XAST → Universal Backend → UniIR → Target Language
```

### XAST Node Kinds
MODULE, FUNC, VAR_DECL, ASSIGN, BINARY_OP, UNARY_OP, CALL, IF, WHILE, FOR, RETURN, LITERAL, IDENT, BLOCK, PARAM, STRUCT_DEF, ENUM_DEF, MATCH

### Universal Backend
The `xast_to_uniir.ti` Titan module handles all languages. It performs:
- Type inference from annotations
- SSA register allocation
- Basic block generation for control flow
- Effect tracking
- Lowering to canonical UniIR

## Adding a New Language

1. Create a frontend parser that produces XAST
2. Register it: `FrontendRegistry.register('ext', MyFrontend)`
3. The universal backend handles the rest

## Fidelity Levels

| Level | Accuracy | Requirements |
|-------|----------|--------------|
| Certified | 100% for defined behavior | Full Axiom embedding, proof attached |
| High-Fidelity | ≥98% semantic match | Extensive test suite, symbolic verification |
| Exploratory | Rapid onboarding | Fast conversion with TODOs and contracts |

## CLI Commands

```bash
# Start the conversion daemon
build lingua start --watch ./myproject

# Check conversion status
build lingua status --project ./myproject

# One-off conversion
build lingua convert utils.c --to=titan

# Bidirectional conversion
build lingua convert program.rs --to=aether
build lingua convert main.ae --to=rust
```

## Example: C to Titan

### Input (C)
```c
#include <stdio.h>

int fibonacci(int n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

int main() {
    printf("%d\n", fibonacci(10));
    return 0;
}
```

### Output (Titan)
```titan
fn fibonacci(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() {
    let result = fibonacci(10);
    println!("{}", result);
}
```

## Example: Python to Sylva

### Input (Python)
```python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

print(fibonacci(10))
```

### Output (Sylva)
```sylva
fn fibonacci(n) {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

print(fibonacci(10));
```

## Semantic Preservation Guarantees

For **Certified** conversions:
- All side effects are preserved
- All control flow is equivalent
- All type invariants are maintained
- Proof token references the Axiom kernel proof

For **High-Fidelity** conversions:
- ≥98% semantic equivalence (verified by property tests)
- Extensive test suite covering edge cases
- Runtime contract assertions for undecidable properties

## Type System Mapping

| C | Rust | Python | → Titan |
|---|------|--------|---------|
| `int` | `i32` | `int` | `i64` |
| `long` | `i64` | `long` | `i64` |
| `char*` | `&str` | `str` | `String` |
| `void*` | `*mut T` | `Any` | `*mut T` |
| Struct | Struct | Class | Struct |
| Union | Enum | Union | Enum |

## Bidirectional Conversion Semantics

When converting back from Titan to source language:
- Type annotations are preserved in comments
- Effect constraints become `noexcept` or `@pure` annotations
- Capabilities become static assertions or guards
- Proofs become precondition comments

## Performance

| Language | Conversion Time | Size Increase |
|----------|-----------------|---------------|
| C (100 KiB) | <500ms | +10% (type annotations) |
| Rust (100 KiB) | <1s | +5% |
| Python (100 KiB) | <2s | +20% (type hints) |
| Java (100 KiB) | <1.5s | +15% |

## Testing & Verification

Every conversion includes:
- AST round-trip verification (parse → emit → parse)
- Semantic equivalence testing (run both, compare output)
- Property-based testing (QuickCheck-style)
- Static analysis (Titan borrow checker validation)

## Status & Roadmap

✅ **Tier 1 (Complete):** C, Rust, Python, JavaScript, TypeScript  
🚧 **Tier 2 (In Progress):** Java, Go, C#  
📋 **Tier 3 (Planned):** 30+ additional languages  

The goal: Any code written in any language can be lifted into the Omnisystem and verified, optimized, and maintained in the language of choice.
