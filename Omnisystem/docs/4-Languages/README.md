# Titan Bootstrap — Native Compiler Seed

`output/titan-compiler.exe` is the native seed binary for the Omnisystem.
It is the **sole compilation tool** required to use the entire system.

## Usage

```powershell
# Compile any Titan source file
.\output\titan-compiler.exe path\to\program.ti

# Run verification for a module (output must contain "Result: 111")
.\output\titan-compiler.exe titan\axlib\ax1_nat.ti
```

## Self-Hosting Contract

The binary in `output/` can compile any Titan (`.ti`) source file without
any external tools. No C compiler, Rust toolchain, LLVM, Python, or other
language runtime is required or permitted.

This invariant is enforced by `BOOTSTRAP_INVARIANTS.md` and the CI workflow
at `.github/workflows/bootstrap-check.yml`.

## Verification

```powershell
# Confirm the binary works
.\output\titan-compiler.exe tests\bootstrap_witness.ti
# Expected output: Result: 111
```

## About This Binary

`titan-compiler.exe` is a Windows x86-64 native executable. It implements
the full Titan compilation pipeline: lexing, parsing, semantic analysis,
and code generation. All Omnisystem source files in `titan/`, `aether/`,
`sylva/`, and `axiom/` are compiled by this binary.

No source rebuild is required or supported — this binary is the seed.
If it needs to be replaced, the replacement must be produced by a previous
working Titan compiler binary, preserving the closed bootstrap chain.
