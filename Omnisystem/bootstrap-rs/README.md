# Titan

The one Omni-Language with a real, working toolchain today: a real lexer,
recursive-descent parser, borrow checker, and tree-walking interpreter,
compiled from Rust. Everything under `Omnisystem/src/compiler/` is
aspirational Titan-flavored source with no compiler yet — this crate is
the actual, runnable thing.

## Install

```sh
cargo build --release
# binary at target/release/titan(.exe)
```

## Use

```sh
titan new myproj          # scaffold a runnable hello-world
titan run file.titan      # execute a program
titan tokens file.titan   # dump the token stream (debugging)
titan test ../bootstrap/tests   # run the fixture suite (16/16 passing)
```

## Status

- Real: lexer, parser, borrow checker (move + mutable/immutable borrow
  tracking), tree-walking interpreter, closures, generics, traits, enums,
  pattern matching (incl. range patterns), loop labels, `?`/`From` error
  conversion, a real minimal LSP (`titan-lsp`, stdio JSON-RPC via
  `lsp-server`/`lsp-types`: parse-error diagnostics on open/change, hover
  showing a function's real signature, go-to-definition for top-level
  `fn`s, completion listing real fn/struct/enum names).
- Not real yet: no package manager, no compiled binary output
  (interpreted only) — see `Omnisystem/src/compiler/` for the
  designed-but-unimplemented native-codegen path.

## Editor setup (VS Code, generic LSP client)

Point any LSP-client extension at `target/release/titan-lsp(.exe)` for
`.titan` files — stdio transport, full-document sync.

## Next steps toward "installable for individuals"

1. `titan fmt` — a formatter.
2. Package/module resolution beyond single-file programs.
