# Omnisystem Language Reference

## Titan — The Systems Language

### Type System
- Static, strong, nominal with structural generic constraints
- Optional dependent types for value-level invariants
- Explicit effect system: io, alloc, panic, telemetry, user-defined
- Quantitative types: 0 (erased), 1 (linear), ω (unrestricted)

### Memory Management
- Ownership + borrowing with lifetime elision (90% automatic)
- Region allocation for arena-style batch deallocation
- No garbage collector
- `unsafe` blocks require formal proof or bounded verification

### Concurrency
- Tasks mapping to native threads
- SIMD annotations for data-parallel operations
- Zero-cost stackless coroutines for async/await

### Standard Library
- `Vec<T>`: Dynamic array with grow/shrink
- `HashMap<K,V>`: Hash table with linear probing
- String utilities: concat, contains, split, trim
- Math: checked arithmetic, overflow detection
- I/O: capability-enforced file and network operations

### Example
```titan
fn safe_add(a: i64, b: i64) -> Result<i64, String> {
    if a > 0 && b > i64::MAX - a {
        return Err("overflow".to_string());
    }
    Ok(a + b)
}
```

---

## Aether — The Application Language

### Actor Model
- Private mutable state per actor, accessible only through message handlers
- Mailbox: ordered queue processed one message at a time
- Supervision tree: one_for_one, one_for_all, rest_for_one strategies
- Location transparency: same syntax for local and remote actors

### Consistency Types
- `Consistent<T>`: Linearizable, single-writer or consensus-backed
- `Eventually<CRDT>`: Conflict-free commutative operations
- `Causal<T>`: Happens-before ordering via vector clocks
- `BoundedStaleness<Duration>`: Acceptable lag window

### CRDT Primitives
- `GCounter`: Grow-only counter, commutative increment
- `GSet`: Grow-only set
- `ORMap`: Observed-Remove Map
- `LWWRegister`: Last-Writer-Wins Register

### Standard Library
- HTTP server/client actors
- Database handles (SQL, key-value)
- JSON/HTML templating
- Built-in observability (OpenTelemetry)

### Example
```aether
actor BankAccount {
    var balance: i64 = 0;
    
    handle Deposit(amount: i64) {
        self.balance = self.balance + amount;
    }
    
    handle Withdraw(amount: i64) -> Result<i64> {
        if amount <= self.balance {
            self.balance = self.balance - amount;
            Ok(self.balance)
        } else {
            Err("insufficient funds")
        }
    }
}
```

---

## Sylva — The Interactive Language

### Type System
- Dynamic by default, optional static checking
- Gradual typing: untyped code runs, types added incrementally
- Refinement contracts: pre/post conditions checked at runtime

### Syntax
- Homoiconic: Code is data, manipulable by macros
- Modern infix notation with Lisp-like AST
- Hygienic macros for custom control flow

### Interactive Features
- Time-travel debugging: rewind, replay, what-if mutation
- Live hot-reload of modules
- Notebook environment (Jupyter/ObservableHQ-style)
- Automatic GPU/FPGA offload via effect inference

### Standard Library
- `DataFrame`: CSV loading, filtering, selection, statistical summary
- `histogram()`: Text-based visualization
- `mean()`, `std()`, `min()`, `max()`: Statistical functions
- Tensor operations with automatic Titan offload

### Example
```sylva
fn fibonacci(n) {
    if n <= 1 { return n; }
    fibonacci(n - 1) + fibonacci(n - 2)
}

let result = fibonacci(10);
print("fib(10) = ", result);
```

---

## Axiom — The Proof Language

### Type System
- Full dependent types with infinite universe hierarchy
- Curry-Howard correspondence: types are propositions, terms are proofs
- Quantitative type theory for erasure

### Proof Development
- Tactics: induction, apply, intro, case, auto
- AI-assisted synthesis: model suggests lemmas and completions
- SMT-backed automation for decidable fragments
- Incremental verification: prove critical 20% first

### Kernel (~500 LOC Titan)
- Weak-head normal form reduction
- Bidirectional dependent type checking
- Universe level checking
- Alpha-equivalence and convertibility
- Trusted axioms: induction, universe hierarchy

### Compilation
- Proof-carrying Titan code
- Erased proofs (0-use) removed at compile time
- Undischarged obligations become runtime contracts

### Example
```axiom
theorem add_assoc: forall (a b c : i64),
    add(add(a, b), c) = add(a, add(b, c))
proof
    intro a b c;
    induction a;
    case 0:
        simp;
    case succ a':
        simp [add_succ];
        assumption;
end
```

---

## Cross-Language Interop

### Calling Titan from Aether
```aether
actor Calculator {
    handle Compute(x: i64, y: i64) -> i64 {
        return titan::math::safe_add(x, y);
    }
}
```

### Calling Aether from Sylva
```sylva
let calculator = spawn_actor("titan://Calculator");
let result = calculator ! Compute(40, 2);
print("Result: ", result);
```

### Embedding Axiom in Titan
```titan
fn verified_multiply(a: i64, b: i64) -> i64 {
    // Proof: mult_comm_law
    return a * b;
}
```

---

## UniIR Lower-Level Representation

For advanced users and compiler developers:

```
module hello_world
type i64 = 64-bit signed integer
effect io = file, network, console

define add_checked(a: i64, b: i64) -> Result<i64, String>
  %entry:
    %cond = i64.gt %a, 9223372036854775807
    br %cond, %overflow, %ok
  %overflow:
    %err = string.lit "overflow"
    %result = Result.Err %err
    ret %result
  %ok:
    %sum = i64.add %a, %b
    %result = Result.Ok %sum
    ret %result

export add_checked
```
