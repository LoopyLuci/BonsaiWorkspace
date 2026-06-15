# Axiom Language Reference

**Axiom** is the formal verification language of the Omnisystem. It sits alongside Titan, Aether, and Sylva in the language stack, but serves a different purpose: instead of computing values, Axiom constructs mathematical proofs that other languages can carry and check at compile time or runtime.

Axiom is founded on dependent type theory — the same foundation used by Coq, Lean, and Agda. A dependent type is a type that can mention values. This makes it possible to state and prove arbitrarily precise specifications: not just "this function returns a number" but "this function returns a number that is strictly less than its input."

The Axiom system has two layers:

1. **The kernel** (~500 lines, `axiom/kernel/kernel.py`): a minimal, auditable trusted computing base that accepts or rejects proof terms. It never guesses; it only checks. Everything outside the kernel is untrusted — including tactics, proof search, and AI assistance.
2. **The tactic layer** (outside the kernel): tools for constructing proof terms interactively. Tactics produce terms; the kernel validates them.

A proof accepted by the kernel is correct by construction, regardless of how it was found.

---

## 1. Design Principles

1. **Minimal trusted base.** The kernel is small enough to be audited in a day. No external dependencies. The entire logical foundation fits in one file.
2. **Tactics are untrusted.** Tactics, proof search, and automation all live outside the kernel. They produce candidate proof terms. The kernel is the final arbiter.
3. **Dependent types for specifications.** Types can depend on values, so you can express `Vec n` (a vector of exactly `n` elements), `x < y` (a proof that x is strictly less than y), or `sorted xs` (a proof that the list xs is sorted).
4. **Curry-Howard correspondence.** Propositions are types; proofs are programs. A proof of `P → Q` is a function from evidence of P to evidence of Q. A proof of `P ∧ Q` is a pair of evidence for P and Q.
5. **Integration with Titan.** Axiom proofs can be attached to Titan functions as proof-carrying code. The proof is erased at compile time (Prop erasure) or kept as a runtime contract (decidable properties).
6. **Impredicative Prop.** `Prop` is impredicative: you can quantify over all propositions inside `Prop` without universe overflow. This is essential for general induction principles.

---

## 2. The Term Language

Every Axiom expression — including types, values, and proofs — is a **term**. The kernel recognizes exactly nine term constructors.

### 2.1 Term Constructors

| Constructor | Syntax | Meaning |
|-------------|--------|---------|
| `TVar(n)` | `#n` | Bound variable at De Bruijn index n |
| `TConst(name)` | `name` | Global constant or axiom |
| `TApp(f, a)` | `f a` | Apply function f to argument a |
| `TLam(T, b)` | `λ(x:T). b` | Lambda abstraction |
| `TPi(T, B)` | `Π(x:T). B` | Dependent function type |
| `TSort(n)` | `Prop` / `Type n` | Universe at level n |
| `TLet(T, v, b)` | `let x:T := v; b` | Local definition |
| `TNat(n)` | `0`, `1`, `2`, ... | Natural number literal |
| `TProof(P, e)` | `proof P by e` | Proof object pairing proposition with evidence |

### 2.2 De Bruijn Indexing

Axiom uses **De Bruijn indices** to represent bound variables. Instead of naming variables (`x`, `y`, `z`), each variable is a number counting how many binders are between the variable and its binding site:

```
λ(x:Nat). x          →   TLam(Nat, TVar(0))
                               ↑ 0 binders between x and its λ

λ(x:Nat). λ(y:Nat). x  →  TLam(Nat, TLam(Nat, TVar(1)))
                                              ↑ 1 binder (the inner λ) between x and its binding λ

λ(x:Nat). λ(y:Nat). y  →  TLam(Nat, TLam(Nat, TVar(0)))
                                              ↑ 0 binders between y and its λ
```

De Bruijn indexing eliminates variable capture and α-equivalence issues. You never need to rename variables to avoid capture. The kernel's substitution (`subst`) and lifting (`_lift`) functions handle index arithmetic automatically.

**Index 0 = innermost binder.** Every time you cross a binder (enter a `TLam`, `TPi`, or `TLet`), the depth counter increases by one.

As a user writing Axiom surface syntax, you write named variables; the compiler translates them to De Bruijn indices before passing them to the kernel.

### 2.3 Universe Hierarchy

Axiom has an infinite tower of universes:

```
Prop   = TSort(-1)   -- propositions; proof-irrelevant; impredicative
Type0  = TSort(0)    -- small types (Nat, Bool, etc.)
Type1  = TSort(1)    -- types of types
Type2  = TSort(2)    -- etc.
```

Each universe lives in the next:

```
Prop  : Type0
Type0 : Type1
Type1 : Type2
Type(n) : Type(n+1)
```

Universe rules for `Π` types:

| Parameter universe | Body universe | Result universe |
|-------------------|---------------|-----------------|
| Any | `Prop` | `Prop` (impredicativity) |
| `Type(i)` | `Type(j)` | `Type(max(i,j))` |

Impredicativity of `Prop` means you can write `Π(P : Prop). P → P` (the type of the identity function on propositions) and the whole thing still lives in `Prop`, not `Type1`. This is what makes general induction principles (which quantify over all predicates) expressible.

### 2.4 Built-in Types and Constants

The standard axiom set pre-populates the global context with:

```
Nat  : Type0    -- natural numbers
Bool : Type0    -- booleans
Unit : Type0    -- unit type
Eq   : Type0 → α → α → Prop   -- propositional equality
```

And the trusted axioms:

```
funext  : ∀(f g : α → β). (∀x. f x = g x) → f = g
propext : ∀(P Q : Prop). (P ↔ Q) → P = Q
choice  : ∀(α : Type). Nonempty α → α       -- classical
ind_nat : natural number induction           -- termination
```

---

## 3. Propositions and Proofs

### 3.1 The Curry-Howard Correspondence

In Axiom, propositions and types are the same thing. A proposition is any term whose type is `Prop`. A proof of proposition `P` is any term whose type is `P`.

| Logic | Type Theory |
|-------|-------------|
| Proposition `P` | Type `P : Prop` |
| Proof of `P` | Term `t : P` |
| `P → Q` (implication) | `Π(_ : P). Q` |
| `P ∧ Q` (conjunction) | `Σ(p : P). Q` (dependent pair) |
| `P ∨ Q` (disjunction) | Sum type |
| `¬P` (negation) | `P → False` |
| `∀(x:T). P x` (universal) | `Π(x:T). P x` |
| `∃(x:T). P x` (existential) | `Σ(x:T). P x` |
| Proof of `P → Q` | Function of type `P → Q` |
| Proof of `P ∧ Q` | Pair `(proof_P, proof_Q)` |

### 3.2 Declaring Theorems

```axiom
-- A theorem with a name, optional parameters, and a proposition
theorem identity_id(n: Nat) : n = n :=
    rfl

-- A lemma (identical to theorem, signals supporting role)
lemma add_zero(n: Nat) : n + 0 = n :=
    induction n {
        base => rfl
        step(k, ih) => simp [ih]
    }

-- A property (used for attaching proofs to Titan functions)
property add_commutative(a b: Nat) : a + b = b + a :=
    induction a {
        base => simp [add_zero]
        step(k, ih) => simp [add_succ, ih]
    }
```

### 3.3 Proof Blocks

For longer proofs, use the `proof ... qed` block syntax:

```axiom
theorem add_associative(a b c: Nat) : a + (b + c) = (a + b) + c :=
proof
    induction a with
    | zero =>
        -- base: 0 + (b + c) = (0 + b) + c
        simp [add_zero_left]
    | succ k ih =>
        -- step: (k+1) + (b+c) = ((k+1)+b)+c
        -- ih: k + (b+c) = (k+b)+c
        simp [add_succ_left, ih]
qed
```

### 3.4 Hypotheses and Context

Inside a proof, local hypotheses are in scope:

```axiom
theorem modus_ponens(P Q: Prop)(h1: P)(h2: P → Q) : Q :=
proof
    apply h2
    exact h1
qed
```

---

## 4. Reduction and Definitional Equality

### 4.1 Reduction Rules

The kernel normalizes terms using three reduction rules:

**β-reduction** (function application):
```
(λ(x:T). b) a  →  b[a/x]
```

**δ-reduction** (let unfolding):
```
let x:T := v; b  →  b[v/x]
```

**ι-reduction** (definition unfolding):
```
TConst("name")  →  its definition (if it has one)
```

### 4.2 Weak Head Normal Form (WHNF)

The kernel reduces terms to **Weak Head Normal Form (WHNF)**: it applies reduction rules only at the outermost position (the "head"), not under binders. This is enough for type checking — it stops as soon as the outermost constructor is stable.

A term is in WHNF if its outermost constructor is one of: `TVar`, `TLam`, `TPi`, `TSort`, `TNat`, or `TProof`. Applications are reduced as needed.

```python
# From the kernel: fuel limit prevents non-termination
_FUEL_LIMIT = 10_000

def whnf(term, ctx, fuel=_FUEL_LIMIT):
    if isinstance(term, TApp):
        head = whnf(term.func, ctx, fuel-1)
        if isinstance(head, TLam):         # β-reduction
            return whnf(subst(head.body, term.arg), ctx, fuel-1)
        return TApp(head, term.arg)
    elif isinstance(term, TLet):           # δ-reduction (let)
        return whnf(subst(term.body, term.val), ctx, fuel-1)
    elif isinstance(term, TConst):         # δ-reduction (definition)
        entry = ctx.lookup_const(term.name)
        if entry and entry.val is not None:
            return whnf(entry.val, ctx, fuel-1)
    return term
```

### 4.3 Definitional Equality

Two terms are **definitionally equal** if they reduce to the same WHNF (up to structural equality). The kernel checks definitional equality via `conv`:

```
(λ x. x) 5  ≡  5         -- beta-equal
let x := 7 in x  ≡  7   -- delta-equal
```

Definitional equality is used everywhere types must match: function arguments, return types, proof checking. It is the kernel's notion of "same type."

---

## 5. Type Checking

### 5.1 Bidirectional Checking

The kernel uses **bidirectional type checking**: two modes that work together.

- **Infer** (`infer(term, ctx) → type`): compute the type of a term from scratch
- **Check** (`check(term, expected_ty, ctx)`): verify that a term has a specific type

Check mode delegates to infer and then verifies definitional equality:

```python
def check(self, term, expected_ty, ctx):
    inferred = self.infer(term, ctx)
    if not conv(whnf(inferred, ctx), whnf(expected_ty, ctx), ctx):
        raise KernelError(f"Type mismatch: expected {expected_ty}, inferred {inferred}")
```

### 5.2 Typing Rules

The kernel implements the following rules:

**Variable:** Look up the De Bruijn index in the context, lift the type.
```
Γ(i) = (x : T)
──────────────────
Γ ⊢ #i : ↑ⁱT
```

**Sort:** Each universe has a type in the next.
```
─────────────────────
Γ ⊢ Type(n) : Type(n+1)
Γ ⊢ Prop : Type(0)
```

**Lambda:** Infer parameter type is a sort; extend context; infer body type.
```
Γ ⊢ T : Sort   Γ, x:T ⊢ b : B
─────────────────────────────────
Γ ⊢ λ(x:T). b  :  Π(x:T). B
```

**Pi (T-Pi):** Both parameter and body types must be sorts. Result universe is max.
```
Γ ⊢ T : Sort(i)   Γ, x:T ⊢ B : Sort(j)
──────────────────────────────────────────
Γ ⊢ Π(x:T). B  :  Sort(max(i,j))     -- unless j = -1 (Prop), then Prop
```

**Application (T-App):** Infer function has a Pi type; check argument against parameter type; substitute argument into body type.
```
Γ ⊢ f : Π(x:T). B   Γ ⊢ a : T
─────────────────────────────────
Γ ⊢ f a  :  B[a/x]
```

**Proof:** Check that evidence has the stated proposition as its type.
```
Γ ⊢ P : Prop   Γ ⊢ e : P
──────────────────────────
Γ ⊢ proof P by e  :  P
```

### 5.3 The Global Context

The kernel merges a local proof context with a global context of definitions and axioms:

```python
def _merge(self, ctx):
    return Context(ctx._entries + self._global._entries)
```

Definitions with a `val` can be unfolded (δ-reduction). Axioms have no `val` and are simply trusted as typed constants.

---

## 6. Writing Proofs

### 6.1 Tactic Reference

Tactics are the user-facing tools for constructing proof terms. Each tactic transforms the current proof state (a goal to prove, plus context). The final proof term is extracted and sent to the kernel for validation.

**`rfl`** — prove `a = a` by reflexivity
```axiom
theorem zero_eq_zero : 0 = 0 := rfl
```

**`intro name`** — introduce the hypothesis or variable at the head of a `Π`/implication
```axiom
theorem impl_self(P: Prop) : P → P :=
proof
    intro h    -- h : P
    exact h
qed
```

**`exact term`** — close the goal with a specific term (must typecheck against goal)
```axiom
theorem trivial(n: Nat) : n = n :=
proof
    exact rfl
qed
```

**`apply term`** — apply a function/lemma to the goal; generates subgoals for each required argument
```axiom
theorem by_hyp(P Q: Prop)(h: P → Q)(hp: P) : Q :=
proof
    apply h
    exact hp
qed
```

**`cases term`** — case split on an inductive value or disjunction
```axiom
theorem bool_exhaust(b: Bool) : b = true ∨ b = false :=
proof
    cases b {
        true  => left; rfl
        false => right; rfl
    }
qed
```

**`induction term`** — structural induction; produces base and step subgoals
```axiom
lemma add_comm_zero(n: Nat) : n + 0 = n :=
proof
    induction n {
        base        => rfl
        step(k, ih) => simp [add_succ, ih]
    }
qed
```

**`simp [lemmas...]`** — simplify using named rewriting lemmas, plus built-in β/δ
```axiom
theorem simplify_example(n: Nat) : (n + 0) + 0 = n :=
proof
    simp [add_comm_zero]
qed
```

**`unfold name`** — unfold a definition (δ-reduction by name)
```axiom
theorem unfold_example : double 3 = 6 :=
proof
    unfold double   -- double n := n + n
    simp [add_nat]
qed
```

**`auto`** — automated proof search using available hypotheses and lemmas; completes trivially decidable goals
```axiom
theorem auto_example(n m: Nat)(h: n < m) : n ≤ m :=
proof
    auto
qed
```

**`left` / `right`** — choose a branch in a disjunction `P ∨ Q`
```axiom
theorem or_intro(P Q: Prop)(h: P) : P ∨ Q :=
proof
    left; exact h
qed
```

**`split`** — split a conjunction `P ∧ Q` into two subgoals
```axiom
theorem and_intro(P Q: Prop)(hp: P)(hq: Q) : P ∧ Q :=
proof
    split
    · exact hp
    · exact hq
qed
```

**`rewrite [h]`** — rewrite using an equation `h : a = b` (replace `a` with `b`)
```axiom
theorem rewrite_example(a b: Nat)(h: a = b) : a + 1 = b + 1 :=
proof
    rewrite [h]
    rfl
qed
```

**`assumption`** — close the goal with a matching hypothesis in the context
```axiom
theorem use_hyp(P: Prop)(h: P) : P :=
proof
    assumption
qed
```

### 6.2 Proof State

At any point in a proof, the proof state shows:

```
Goals (2 remaining):
  [1] n k : Nat, ih : k + 0 = k ⊢ succ k + 0 = succ k
  [2] ⊢ 0 + 0 = 0
```

The format is: `context ⊢ goal`. Context entries are separated by commas. Each goal can be addressed by tactics in turn.

### 6.3 Bullet Points and Focusing

When a tactic generates multiple subgoals, use bullet syntax to focus on each:

```axiom
theorem and_comm(P Q: Prop)(h: P ∧ Q) : Q ∧ P :=
proof
    split
    · exact h.right     -- goal 1: Q
    · exact h.left      -- goal 2: P
qed
```

---

## 7. Kernel API

For embedding and programmatic use, the kernel exposes a Python API:

### 7.1 Core Functions

```python
from axiom.kernel.kernel import (
    TVar, TConst, TApp, TLam, TPi, TSort, TLet, TNat, TProof,
    Prop, Type0, Type1,
    Context, KernelError,
    subst, whnf, conv,
    TypeChecker, standard_axioms,
    make_kernel, verify_proof, check_term,
)
```

**`make_kernel() → TypeChecker`** — create a type checker with the standard axiom set
```python
kernel = make_kernel()
```

**`check_term(term) → (bool, type, error_msg)`** — type-check in the standard context
```python
ok, ty, err = check_term(TNat(42))
# ok=True, ty=TConst("Nat"), err=None
```

**`verify_proof(prop, evidence) → bool`** — verify that evidence proves prop; never raises
```python
identity = TLam(TConst("Nat"), TVar(0))
prop = TPi(TConst("Nat"), TConst("Nat"))
ok = verify_proof(prop, identity)   # True — identity proves Nat → Nat
```

**`TypeChecker.infer(term, ctx) → Term`** — infer the type of a term
```python
kernel = make_kernel()
ctx = Context()
ty = kernel.infer(Type0, ctx)   # TSort(1) = Type1
```

**`TypeChecker.check(term, expected_ty, ctx)`** — check type or raise `KernelError`
```python
kernel.check(TNat(5), TConst("Nat"), Context())  # ok
kernel.check(TNat(5), Prop, Context())           # raises KernelError
```

**`whnf(term, ctx) → Term`** — reduce to weak head normal form
```python
identity = TLam(TConst("Nat"), TVar(0))
app = TApp(identity, TNat(7))
result = whnf(app, standard_axioms())  # TNat(7)
```

**`conv(t1, t2, ctx) → bool`** — check definitional equality
```python
identity = TLam(TConst("Nat"), TVar(0))
conv(TApp(identity, TNat(5)), TNat(5), standard_axioms())  # True
```

**`subst(term, value, depth=0) → Term`** — De Bruijn substitution
```python
# subst(TVar(0), TNat(5), 0) = TNat(5)
result = subst(TVar(0), TNat(5), 0)
```

### 7.2 Building Context

```python
ctx = Context()

# Add a variable binding (hypothesis)
ctx = ctx.extend("n", TConst("Nat"))          # n : Nat

# Add a definition
ctx = ctx.extend("zero", TConst("Nat"), TNat(0))  # zero : Nat := 0

# Look up by De Bruijn index
entry = ctx.lookup(0)    # most recently added: "n"

# Look up by name
entry = ctx.lookup_const("n")   # ContextEntry(name="n", ty=TConst("Nat"), val=None)
```

### 7.3 The Standard Axiom Set

```python
axioms = standard_axioms()
# Contains:
#   funext, propext, choice, ind_nat  (trusted axioms)
#   Nat, Bool, Unit                   (base types : Type0)
#   Eq                                (equality : Type0 → α → α → Prop)
```

### 7.4 Normalizer API

The normalizer (`axiom.kernel.normalizer`) exposes an alternative term representation used by the WHNF engine:

```python
from axiom.kernel.normalizer import (
    Var, Lam, App, Pi, Universe, Let, Definition,
    whnf, substitute, shift,
    alpha_equal, convertible, term_to_string,
)

# Var uses an integer index and optional name hint
v = Var(0, "x")

# Universe uses a non-negative integer level
u = Universe(0)   # Type 0

# alpha_equal: structural equality ignoring variable names
alpha_equal(Lam(Var(0, "x"), None), Lam(Var(0, "y"), None))  # True

# convertible: check equality after normalization
convertible(App(Lam(Var(0), None), Universe(0)), Universe(0))  # True
```

---

## 8. Integration with Titan

### 8.1 Proof-Carrying Code

Titan functions can carry Axiom proofs as annotations. The proof is erased at compile time for `Prop` results and inlined as a runtime assertion for decidable properties:

```titan
// Function with proof annotation
#[proof: add_commutative]
fn add(a: i64, b: i64) -> i64 {
    return a + b;
}

// The Axiom proof
property add_commutative(a b: i64) : add(a, b) = add(b, a) :=
    by simp [add_comm_nat]
```

### 8.2 Preconditions and Postconditions

```titan
// Express preconditions as Prop-typed parameters
fn safe_div(a: i64, b: i64, proof_nonzero: b ≠ 0) -> i64 {
    return a / b;
}

// The proof_nonzero parameter has type `b ≠ 0` (a Prop)
// At call sites, the caller must supply evidence that b is non-zero
```

### 8.3 Refinement Types

Refinement types embed proof obligations into the type:

```titan
// i64 refined by a predicate
fn sqrt(n: {x: i64 | x >= 0}) -> {y: i64 | y * y <= n && (y+1)*(y+1) > n} {
    // implementation
}
```

The Axiom kernel discharges the refinement predicates. For decidable predicates (`x >= 0` on concrete integers), the check can be compiled to a runtime assertion. For general Prop predicates, the caller must supply a proof term.

### 8.4 Effect Proofs

Titan's effect system can be extended with Axiom proofs:

```titan
// Prove that a function only touches resources it claims
#[effect_proof: bounded_alloc_proof]
fn process_data(data: &[i64]) ! { alloc } -> Vec<i64> {
    // ...
}

property bounded_alloc_proof(data: &[i64]) : 
    allocs(process_data(data)) <= len(data) * 8 :=
    by induction len(data) { ... }
```

---

## 9. Universe Polymorphism

When a proof or definition must work at all universe levels, use universe-polymorphic declarations:

```axiom
-- Identity function polymorphic in its type universe
def id {u: Universe} (A: Type u) (x: A) : A := x

-- Type inference fills in u at call sites
-- id Nat 5     -- u=0, A=Nat, x=5
-- id Type0 Nat -- u=1, A=Type0, x=Nat
```

The Axiom surface language infers universe levels automatically. The kernel enforces that no universe contains itself (`Type n ∉ Type n`, only `Type n : Type(n+1)`).

---

## 10. Inductive Types

### 10.1 Defining Inductive Types

```axiom
-- Natural numbers
inductive Nat : Type0 where
    | zero : Nat
    | succ : Nat → Nat

-- Lists
inductive List (A: Type0) : Type0 where
    | nil  : List A
    | cons : A → List A → List A

-- Propositional equality (built-in, shown for illustration)
inductive Eq {A: Type0} (a: A) : A → Prop where
    | refl : Eq a a
```

### 10.2 Recursor / Elimination

Every inductive type generates an eliminator (recursor) that the kernel accepts as a valid constant. The eliminator is the kernel-trusted primitive; `induction` tactic invokes it:

```axiom
-- Nat.rec is the kernel-trusted eliminator:
--   Nat.rec : Π(P : Nat → Prop).
--               P zero →
--               (Π(n : Nat). P n → P (succ n)) →
--               Π(n : Nat). P n

theorem add_zero_right(n: Nat) : n + 0 = n :=
    Nat.rec
        (fun n => n + 0 = n)           -- motive
        rfl                             -- base: 0 + 0 = 0
        (fun k ih => simp [succ_add, ih])  -- step
        n
```

### 10.3 Structural Recursion and Termination

All recursive proofs must be **structurally recursive** on an inductive argument. The kernel's fuel limit (`_FUEL_LIMIT = 10_000`) provides a safety valve, but well-formed proofs normalize in polynomial steps.

---

## 11. Trusted Axiom Set

The kernel's trusted axioms are the only unproved assumptions in the system. Every other theorem is derived from these:

### 11.1 Function Extensionality (`funext`)

Two functions are equal if they agree on all inputs:
```
funext : ∀(f g : α → β). (∀(x : α). f x = g x) → f = g
```

Without this axiom, `(fun x => x + 0)` and `(fun x => x)` would not be provably equal even though they compute the same values.

### 11.2 Propositional Extensionality (`propext`)

Two propositions are equal if they are logically equivalent:
```
propext : ∀(P Q : Prop). (P ↔ Q) → P = Q
```

### 11.3 Classical Choice (`choice`)

If a type is non-empty, there exists an element of it:
```
choice : ∀(α : Type). Nonempty α → α
```

This is the classical axiom of choice. It makes the logic classical (law of excluded middle follows). Use `#[constructive]` to disable it for a file and stay in intuitionistic logic.

### 11.4 Natural Number Induction (`ind_nat`)

```
ind_nat : ∀(P : Nat → Prop). P 0 → (∀n. P n → P (succ n)) → ∀n. P n
```

This is derivable from the `Nat` inductive type, but it is listed in the axiom set for the initial phase.

---

## 12. Complete Examples

### 12.1 Addition is Commutative

```axiom
-- Helper: 0 + n = n
lemma add_zero_left(n: Nat) : 0 + n = n :=
proof
    induction n {
        base        => rfl
        step(k, ih) => simp [add_succ, ih]
    }
qed

-- Helper: succ a + b = succ (a + b)
lemma add_succ_left(a b: Nat) : succ a + b = succ (a + b) :=
proof
    induction b {
        base        => simp [add_zero_right]
        step(k, ih) => simp [add_succ, ih]
    }
qed

-- Main theorem
theorem add_comm(a b: Nat) : a + b = b + a :=
proof
    induction a {
        base =>
            -- 0 + b = b + 0
            simp [add_zero_left, add_zero_right]
        step(k, ih) =>
            -- succ k + b = b + succ k
            -- ih: k + b = b + k
            simp [add_succ_left, add_succ, ih]
    }
qed
```

### 12.2 Verified Division (No Division by Zero)

```axiom
-- Refinement type: natural number with proof it is positive
def Positive := { n: Nat | n > 0 }

-- Safe division: caller must supply proof that divisor is positive
theorem div_bounded(a: Nat)(b: Positive) :
    a / b.val < a + 1 :=
proof
    -- b.proof gives us b.val > 0
    have hpos := b.proof
    -- use standard library division bound
    apply div_lt_iff hpos
    auto
qed
```

### 12.3 Checked Arithmetic (Titan Integration)

```titan
// In Titan source
fn add_checked(a: i64, b: i64) -> i64 ! { panic } {
    let result = a + b;
    if result < a {
        panic("overflow");
    }
    return result;
}
```

```axiom
-- Axiom proof: if no overflow occurs, the result equals mathematical addition
property add_checked_correct(a b: i64)(no_overflow: a + b >= a) :
    add_checked(a, b) = a + b :=
proof
    unfold add_checked
    -- the overflow branch is unreachable given no_overflow
    simp [no_overflow]
    rfl
qed

-- Associativity (under no-overflow conditions)
property add_checked_associative
    (a b c: i64)
    (h1: a + b >= a)(h2: (a + b) + c >= a + b) :
    add_checked(add_checked(a, b), c) = add_checked(a, add_checked(b, c)) :=
proof
    simp [add_checked_correct h1, add_checked_correct h2]
    -- reduces to i64 arithmetic associativity
    auto
qed
```

### 12.4 Actor Protocol Correctness (Aether Integration)

```axiom
-- Prove that a bank account actor never goes negative
-- given well-formed message sequences

inductive AccountMsg : Type0 where
    | Deposit  : Nat → AccountMsg
    | Withdraw : Nat → AccountMsg

def process_msg(balance: Nat)(msg: AccountMsg) : Nat :=
    match msg {
    | Deposit  n => balance + n
    | Withdraw n => if n <= balance then balance - n else balance
    }

-- Prove balance stays non-negative
theorem balance_nonneg(initial: Nat)(msgs: List AccountMsg) :
    List.foldl process_msg initial msgs >= 0 :=
proof
    induction msgs {
        base => exact Nat.zero_le initial
        step(msg, rest, ih) =>
            simp [List.foldl]
            cases msg {
                Deposit n  => simp [process_msg]; auto
                Withdraw n =>
                    simp [process_msg]
                    cases (le_dec n (List.foldl process_msg initial rest)) {
                        true  h => simp [h]; exact Nat.sub_nonneg
                        false h => simp [h]; exact ih
                    }
            }
    }
qed
```

---

## 13. Kernel Self-Test

The kernel runs a self-test at import time to catch regressions. The test verifies:

1. `Type0 : Type1` — sort hierarchy is correct
2. `λ A x. x : Π(A:Type0). Π(x:A). A` — identity function type-checks
3. Substitution and β-reduction produce correct results

If the self-test fails (e.g., after a kernel edit), importing the kernel raises an `AssertionError`. This is intentional: a broken kernel must not silently accept incorrect proofs.

---

## 14. Quick Reference Card

```
TERM CONSTRUCTORS
  TVar(n)          -- bound variable, De Bruijn index n (0 = innermost)
  TConst("name")   -- global constant or axiom
  TApp(f, a)       -- apply f to a  [f a]
  TLam(T, b)       -- lambda  [λ(x:T). b]
  TPi(T, B)        -- dependent function type  [Π(x:T). B]
  TSort(n)         -- universe: -1=Prop, 0=Type0, 1=Type1, ...
  TLet(T, v, b)    -- local definition  [let x:T := v; b]
  TNat(n)          -- natural number literal
  TProof(P, e)     -- proof object  [proof P by e]

UNIVERSE HIERARCHY
  Prop   = TSort(-1)   -- propositions, impredicative
  Type0  = TSort(0)    -- small types
  Type1  = TSort(1)    -- Prop:Type0, Type0:Type1, ...
  Π(x:T).Prop : Prop   -- impredicativity (critical for induction)

DECLARATIONS
  theorem  name(params) : proposition := proof_term
  lemma    name(params) : proposition := proof_term
  property name(params) : proposition := proof_term   -- for Titan attachment
  proof ... qed                                       -- block syntax

TACTICS
  rfl              -- close a = a
  intro name       -- introduce parameter / hypothesis
  exact term       -- close with a specific proof term
  apply lemma      -- apply a lemma; generates subgoals
  cases term       -- case split on inductive / disjunction
  induction term { base => ... ; step(k,ih) => ... }
  simp [lemmas...] -- simplify with rewriting rules
  unfold name      -- delta-unfold a definition
  auto             -- automated search
  rewrite [h]      -- rewrite with equation h : a = b
  assumption       -- close with matching hypothesis
  left / right     -- choose disjunct in P ∨ Q
  split            -- split conjunction P ∧ Q into two goals

REDUCTION RULES
  β: (λ(x:T). b) a  →  b[a/x]
  δ: let x := v; b  →  b[v/x]
  ι: TConst(name)   →  its definition (if any)

TRUSTED AXIOMS
  funext  : (∀x. f x = g x) → f = g
  propext : (P ↔ Q) → P = Q
  choice  : Nonempty α → α
  ind_nat : P 0 → (∀n. P n → P(n+1)) → ∀n. P n

PYTHON API
  make_kernel()              → TypeChecker with standard axioms
  check_term(term)           → (ok, type, error)
  verify_proof(prop, evid)   → bool (never raises)
  whnf(term, ctx)            → WHNF form
  conv(t1, t2, ctx)          → bool (definitional equality)
  subst(term, value, depth)  → substituted term

KERNEL INVARIANTS
  - Fuel limit: 10,000 reduction steps (non-termination safety valve)
  - All KernelErrors are caught at verify_proof; never leak silently
  - Self-test runs at import; failure = AssertionError (not silent)
  - Prop is erased at Titan compile time; decidable predicates → runtime checks
```
