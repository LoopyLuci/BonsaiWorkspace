# OmniCC Bootstrap — a runnable Titan compiler & runtime

This is the **seed bootstrap** for the Omnisystem languages: a complete,
runnable front end and tree-walking runtime for **Titan**, the systems language
in which the rest of the Omni compiler is written. It exists to break the
chicken‑and‑egg problem — you cannot run the Titan‑in‑Titan compiler until some
seed can execute Titan. This seed does exactly that.

It is written in TypeScript and runs **directly on Node ≥ 22.6 with no build
step** (Node strips the TypeScript types natively). That keeps it maximally
individual‑friendly: clone, run, done.

```bash
node src/cli.ts run   program.titan     # compile and execute
node src/cli.ts check program.titan     # parse + resolve, fast feedback
node src/cli.ts build program.titan     # check + report readiness
node src/cli.ts tokens program.titan    # dump the token stream
node src/cli.ts ast   program.titan     # dump the AST as JSON
node src/cli.ts test  tests             # run the test suite
# or, as an installed bin, from anywhere:
omnicc run program.titan
```

## Architecture (pipeline)

| Stage | File | Responsibility |
|-------|------|----------------|
| Diagnostics | `src/diagnostics.ts` | Source spans + rustc‑style caret errors |
| Lexer | `src/lexer.ts` | Titan tokens: numbers (hex/bin/oct/float/suffix), strings/chars, lifetimes, nested block comments, full operator set |
| AST | `src/ast.ts` | Typed nodes (discriminated unions) |
| Parser | `src/parser.ts` | Recursive descent for items/stmts/patterns/types + Pratt expression precedence; handles `>>` generic closing, struct‑literal ambiguity, `?`, closures, ranges |
| Values | `src/values.ts` | Runtime value model + lexical environments |
| Interpreter | `src/interpreter.ts` | Evaluator: structs, enums, `impl` methods, associated consts, `match`, control flow, the `?` operator |
| Builtins | `src/builtins.ts` | `Vec`, `HashMap`, `HashSet`, `String`, `Option`, `Result`, numeric/char/range methods, `println!`/`format!`/`vec!`/`assert!` |
| Linker | `src/linker.ts` | Multi‑file module resolution (`mod name;` → sibling files) into one program |
| OmniCC CLI | `src/cli.ts` | `run` / `check` / `build` / `tokens` / `ast` / `test` |

## What it covers

Structs & tuple structs, enums (incl. `Option`/`Result`), inherent `impl`
methods and associated functions/constants, generics (parsed, type‑erased at
runtime), closures, pattern matching (literals, enums, structs, tuples, `|`
alternatives, guards, bindings), `if/else`, `if let`, `while`, `while let`,
`for … in` ranges/collections, `loop`/`break`/`continue`, the `?` operator,
ranges, method‑chaining iterators (`map`/`filter`/`fold`/…), and formatted
printing.

It parses **and runs** real code from `../src/stdlib/TitanStdlib.titan`
(34 structs, 150 methods): `omnicc check ../src/stdlib/TitanStdlib.titan`.

## Honest boundaries (this is a seed, by design)

- **Dynamic, not statically checked**: types and generics are parsed but not
  type‑checked or monomorphized; there is no borrow checker. Programs run with
  runtime dispatch. This is the correct scope for a seed — enough to *execute*
  Titan and thereby host a real self‑hosted compiler later.
- **Interpreted, not native**: no machine‑code/ELF/PE emission here. The
  in‑repo `src/compiler/backend` describes that path; the seed's job is to make
  Titan runnable so those can be developed and tested.
- **Integers are f64‑backed** in the runnable subset (no 64‑bit wraparound).
- **Titan only**: the other six languages (Vera/Helix/Aether/Axiom/Sylva/Nexus)
  have distinct grammars and are not parsed by this seed. Titan is the
  bootstrap target because the compiler itself is written in Titan.

## Next step toward self‑hosting

Grow the covered subset until this seed can run the Titan‑written compiler in
`../src/compiler`, then compile that compiler with itself — at which point the
seed is no longer needed. The test suite in `tests/` (with `//@ expect-stdout`
/ `//@ expect-exit` / `//@ expect-error` assertions) is the ratchet that keeps
each step honest.
