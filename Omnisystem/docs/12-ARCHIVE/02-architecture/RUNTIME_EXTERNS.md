# Titan Runtime Extern Interface

Every Titan source file that allocates or inspects heap terms declares the
following extern block. These functions are built into the Titan runtime and
require no linking or import beyond the declaration itself.

```titan
extern "titan" {
    fn heap_alloc_term() -> i64;
    fn heap_set_tag(id: i64, tag: i64);
    fn heap_get_tag(id: i64) -> i64;
    fn heap_set_body(id: i64, val: i64);
    fn heap_get_body(id: i64) -> i64;
    fn heap_set_arg(id: i64, val: i64);
    fn heap_get_arg(id: i64) -> i64;
    fn heap_set_domain(id: i64, val: i64);
    fn heap_get_domain(id: i64) -> i64;
}
```

## Function Reference

### `heap_alloc_term() -> i64`
Allocates a fresh term cell on the Titan heap. Returns an opaque `i64`
handle. All fields (`tag`, `body`, `arg`, `domain`) are initialized to `0`.

### `heap_set_tag(id, tag)` / `heap_get_tag(id) -> i64`
The `tag` field stores the type discriminant for a term cell. Use distinct
`i64` constants per type (e.g. `const T_NIL: i64 = 28101`). The `get`
variant returns the tag stored on cell `id`.

### `heap_set_body(id, val)` / `heap_get_body(id) -> i64`
The `body` field is the primary payload. For constructor terms it holds
the main data value (head element, integer value, key, etc.). For counter
nodes it holds the running count.

### `heap_set_arg(id, val)` / `heap_get_arg(id) -> i64`
The `arg` field is the secondary payload. Commonly used for the tail
pointer of a cons cell, the post-condition of a spec, or an action code.

### `heap_set_domain(id, val)` / `heap_get_domain(id) -> i64`
The `domain` field is the tertiary payload. Used for next-pointer chains,
port identifiers stored separately from counts, or additional metadata.

## Cell Layout Summary

| Field    | Typical use                               |
|----------|-------------------------------------------|
| `tag`    | Type discriminant constant                |
| `body`   | Primary value / head / counter            |
| `arg`    | Secondary value / tail / post-condition   |
| `domain` | Tertiary value / next-pointer / metadata  |

## Tag Constant Ranges

Each Omnisystem subsystem owns a tag range to prevent collisions:

| Range         | Subsystem                         |
|---------------|-----------------------------------|
| 27000–27099   | OmniCloak (C1–C8)                 |
| 27100–27199   | OmniBot (H1–H9)                   |
| 27200–27399   | OmniDesign (F1–F15)               |
| 27400–27499   | OmniLib L5 (Distributed)         |
| 27500–27599   | OmniLib L4 (ML)                   |
| 27600–27699   | OmniLib L3 (Media)                |
| 27700–27799   | OmniLib L2 (DB)                   |
| 27800–27899   | OmniLib L1 (Net)                  |
| 28000–28099   | Axiom AX1 (Nat arithmetic)        |
| 28100–28199   | Axiom AX2 (List theory)           |
| 28200–28299   | Axiom AX3 (Program verification)  |
| 28300–28399   | Axiom AX4 (Concurrency)           |
| 28400–28499   | Axiom AX5 (Crypto)                |

## Design Rules

1. **No cross-file imports.** Every `.ti` file is self-contained. If a
   function from another module is needed, inline it with a unique prefix.
2. **Unique function prefixes.** All public functions in a module use a
   prefix matching the module (e.g. `ax1_`, `h3_`, `f7_`). Test files use
   the `t` prefix variant (e.g. `tax1_`, `th3_`).
3. **No hardcoded 111.** The score `111` must be the computed result of
   actual operations. Any test that returns a literal `111` without prior
   computation is a stub and will be rejected.
4. **Mutable variables.** Use `let mut x: i64 = ...` for variables that
   are reassigned. Immutable `let` cannot be changed after declaration.
5. **No boolean type.** Use `i64` for truth values: `1` = true, `0` = false.
6. **No tuple returns, no array literals, no `break`, no `||` in conditions.**
   Split compound conditions into sequential `if` statements.
