# Contributing to Omnisystem

Thank you for your interest in contributing to Omnisystem! This document
describes how to participate correctly so that the self-hosting contract
is never broken.

---

## Core Rules (Non-Negotiable)

1. **No external languages.** All source code must be written in Titan (`.ti`),
   Aether (`.ae`), Sylva (`.sy`), or Axiom (`.ax`). No Rust, Python, C, or
   any other language is permitted in the active source tree.

2. **No cross-file imports.** Every `.ti` file is self-contained. If you need
   a function from another module, inline it with a unique prefix.

3. **Every module must score 111.** The `main()` function of every module
   must return `111` as the result of actual computation, not as a literal.

4. **The bootstrap binary is the only tool.** All compilation uses
   `titan-bootstrap/output/titan-compiler.exe`. No other compiler, linker,
   or interpreter is permitted.

5. **Every test must be committed.** Test files in `tests/` are tracked by
   git. Do not use `git add -f` to sneak in files that `.gitignore` would
   block — update `.gitignore` instead.

---

## How to Add a New Module

1. Create `titan/<subsystem>/<phase>_<name>.ti` with a unique function prefix.
2. Begin the file with `extern "titan" { ... }` declaring only the heap
   functions your module uses.
3. Implement `pub fn main() -> i64` that returns `111` after real computation.
4. Create `tests/test_<phase>.ti` with a `t<prefix>_` test prefix.
5. Create `scripts/verification/verify_<PHASE>.ps1` using the standard
   verification script pattern.
6. Run your verify script. It must exit 0.
7. Commit: `git add titan/... tests/... scripts/verification/...`

---

## Standard Verify Script Pattern

```powershell
# verify_XX.ps1 — Phase XX: Description
$exe = Join-Path $PSScriptRoot "..\..\titan-bootstrap\output\titan-compiler.exe"
if (-not (Test-Path $exe)) { Write-Error "Bootstrap not found: $exe"; exit 1 }

$modules = @("titan/<subsystem>/<phase>.ti", "tests/test_<phase>.ti")
$regression = @("titan/<subsystem>/<prev1>.ti", "titan/<subsystem>/<prev2>.ti")
$failed = 0
Write-Host "`n=== Phase XX: Description ===" -ForegroundColor Cyan
foreach ($mod in $modules) {
    $out = & $exe $mod 2>&1 | Out-String
    if ($out -match "Result:\s*111") { Write-Host "  PASS  $mod" -ForegroundColor Green }
    else { Write-Host "  FAIL  $mod  ($($out.Trim()))" -ForegroundColor Red; $failed++ }
}
Write-Host "`n=== Regression ===" -ForegroundColor Cyan
foreach ($mod in $regression) {
    $out = & $exe $mod 2>&1 | Out-String
    if ($out -match "Result:\s*111") { Write-Host "  PASS  $mod" -ForegroundColor Green }
    else { Write-Host "  FAIL  $mod  ($($out.Trim()))" -ForegroundColor Red; $failed++ }
}
$total = $modules.Count + $regression.Count
Write-Host ""
if ($failed -eq 0) { Write-Host "RESULT: ALL $total VERIFIED — Phase XX complete [score: 111]" -ForegroundColor Green; exit 0 }
else { Write-Host "RESULT: $failed/$total FAILED" -ForegroundColor Red; exit 1 }
```

---

## Titan Language Rules

- `let mut x: i64 = ...` for mutable variables; plain `let` is immutable.
- No boolean type — use `i64`: `1` = true, `0` = false.
- No tuple returns, no array literals, no `break`.
- No `||` in conditions — split into two sequential `if` statements.
- All constants require `: i64` type annotation.
- `extern "titan"` parameters must be named: `fn heap_set_tag(id: i64, tag: i64)`.
- `pub fn` for exported functions, undecorated `fn` for internal helpers.

---

## Commit Message Format

```
feat: Phase XX — Short description (key features). N modules, 111.
```

Examples:
```
feat: Phase AX3 — Program Verification (Hoare triples, spec/check/verify). 2+2 modules, 111.
feat: Phase F7 — Audio Engine (mixer, envelope, reverb). 2+2 modules, 111.
```

---

## Reporting Issues

Open an issue at https://github.com/LoopyLuci/Omnisystem/issues with:
- The failing verify script and its output
- The module file that fails
- Steps to reproduce
