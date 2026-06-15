# AXIOM Language Specification - Complete Reference

**Formal specification for AXIOM theorem proving and verification language**

---

## Language Overview

**AXIOM** is a logic-based language for formal verification with:
- First-order logic with quantifiers
- Automated theorem proving
- Proof generation and checking
- Type theory integration
- Contract specifications
- Invariant checking
- Safety and liveness properties

---

## Logic Foundations

### Propositional Logic

```axiom
// Propositions
prop P: Bool
prop Q: Bool
prop R: Bool

// Logical connectives
pred AND(p: Bool, q: Bool) -> Bool {
    p && q
}

pred OR(p: Bool, q: Bool) -> Bool {
    p || q
}

pred NOT(p: Bool) -> Bool {
    !p
}

pred IMPLIES(p: Bool, q: Bool) -> Bool {
    !p || q
}

pred IFF(p: Bool, q: Bool) -> Bool {
    (p && q) || (!p && !q)
}

// Laws of logic
axiom law_of_excluded_middle: P || !P
axiom law_of_non_contradiction: !(P && !P)
axiom double_negation: P = !!P
```

### First-Order Logic

```axiom
// Variables and quantifiers
var x: Int
var y: Int

// Universal quantification
prop forall_positive: forall x: Int, x > 0 => x + 1 > 0

// Existential quantification
prop exists_solution: exists x: Int, x * x = 4

// Nested quantifiers
prop nested: forall x: Int, exists y: Int, y > x

// Equality
pred equals(a: T, b: T) -> Bool {
    a == b
}

// Predicates
pred greater_than(a: Int, b: Int) -> Bool {
    a > b
}

pred is_even(n: Int) -> Bool {
    exists k: Int, n == 2 * k
}
```

---

## Type Theory

### Types & Sorts

```axiom
// Sort definitions
sort Nat  // Natural numbers
sort Bool  // Booleans
sort Int   // Integers

// Type constructors
type List(T) where T: Sort
type Pair(A, B) where A: Sort, B: Sort
type Option(T) where T: Sort

// Dependent types
type Vec(n: Nat, T: Sort)  // Vector of length n
type Matrix(m: Nat, n: Nat, T: Sort)

// Type constraints
pred is_nat(x: Nat) -> Bool {
    x >= 0
}

pred is_list(l: List(T)) -> Bool {
    length(l) >= 0
}
```

### Type Checking

```axiom
// Type inference
infer_type(5) = Nat
infer_type([1, 2, 3]) = List(Nat)
infer_type((true, "hello")) = Pair(Bool, String)

// Type unification
unify(List(T), List(Int)) => T = Int
unify(Pair(A, B), Pair(Int, String)) => A = Int, B = String
```

---

## Specifications

### Contract Specifications

```axiom
// Function specification
fun max(a: Int, b: Int) -> Int
  requires: true  // Precondition
  ensures: (result >= a) && (result >= b)  // Postcondition
  {
    if a >= b { a } else { b }
  }

// Array access specification
fun array_get(arr: Array(T), idx: Nat) -> T
  requires: idx < length(arr)
  ensures: result == arr[idx]
  {
    arr[idx]
  }

// Loop invariant
fun sum_array(arr: Array(Int)) -> Int
  requires: true
  ensures: result == sum(arr)
  {
    mut sum = 0
    for i in 0..length(arr) {
      // Invariant: sum == sum(arr[0..i])
      sum = sum + arr[i]
    }
    sum
  }
```

### Property Specifications

```axiom
// Safety property
prop array_bounds_safety: forall arr: Array(T), idx: Nat,
  idx < length(arr) => access_valid(arr, idx)

// Liveness property
prop eventual_completion: forall task: Task,
  starts(task) => eventually(completes(task))

// Fairness property
prop fair_scheduling: forall process: Process,
  enabled(process) => eventually(scheduled(process))

// State machine property
prop state_transition_safety: forall s: State, s': State,
  valid_transition(s, s') => consistent(s, s')
```

---

## Proof Development

### Theorem Definition

```axiom
// Simple theorem
theorem addition_commutative: forall a: Int, b: Int,
  a + b == b + a
{
  // Proof by induction
  induction on a {
    case 0: {
      calc 0 + b
        == b + 0  // by identity
    },
    case a + 1: {
      calc (a + 1) + b
        == a + (1 + b)  // by associativity
        == a + (b + 1)  // by inductive hypothesis
        == (b + 1) + a  // by arithmetic
        == b + (1 + a)  // by associativity
        == b + (a + 1)  // by commutativity of 1
    }
  }
}

// Theorem with proof tactics
theorem list_append_associative: forall l1: List(T), l2: List(T), l3: List(T),
  (l1 ++ l2) ++ l3 == l1 ++ (l2 ++ l3)
{
  intro l1, l2, l3
  induction l1 {
    case []: simp,
    case h::t: {
      rw [append_cons]
      rw [ih]
    }
  }
}
```

### Proof Tactics

```axiom
// Available tactics
tactic simp {
  // Simplification using equalities and reductions
}

tactic rw [rule] {
  // Rewrite using a lemma or equation
}

tactic induction var {
  // Structural induction
}

tactic cases var {
  // Case analysis on disjunctions
}

tactic intro vars {
  // Introduce variables and assumptions
}

tactic apply lemma {
  // Apply a previously proven lemma
}

tactic exact term {
  // Provide exact proof term
}

tactic contradiction {
  // Derive contradiction from assumptions
}

tactic by_contra {
  // Proof by contradiction
}
```

---

## Verification Domains

### Arithmetic Properties

```axiom
// GCD properties
theorem gcd_commutative: forall a: Nat, b: Nat,
  gcd(a, b) == gcd(b, a)

theorem gcd_identity: forall a: Nat,
  gcd(a, 0) == a

theorem gcd_divides: forall a: Nat, b: Nat,
  divides(gcd(a, b), a) && divides(gcd(a, b), b)

// Prime number properties
theorem prime_unique_factorization: forall n: Nat, n > 1,
  exists primes: List(Nat), product(primes) == n &&
  forall p: Nat, in(p, primes) => is_prime(p)
```

### List Properties

```axiom
// List operations
theorem length_append: forall l1: List(T), l2: List(T),
  length(l1 ++ l2) == length(l1) + length(l2)

theorem reverse_involution: forall l: List(T),
  reverse(reverse(l)) == l

theorem map_composition: forall l: List(T), f: T -> U, g: U -> V,
  map(g, map(f, l)) == map(compose(g, f), l)

theorem filter_commutative: forall l: List(T), p: T -> Bool, q: T -> Bool,
  filter(p, filter(q, l)) == filter(both(p, q), l)
```

### Program Correctness

```axiom
// Binary search correctness
theorem binary_search_correct: forall arr: Array(Int), target: Int,
  is_sorted(arr) => 
  (binary_search(arr, target) == -1 || 
   arr[binary_search(arr, target)] == target)

// Quicksort correctness
theorem quicksort_correct: forall l: List(Int),
  is_sorted(quicksort(l)) && is_permutation(quicksort(l), l)

// Merge sort stability
theorem mergesort_stable: forall l: List(T), comp: (T, T) -> Order,
  stable_sort(mergesort, l, comp)
```

---

## Invariant Checking

### Loop Invariants

```axiom
// Bubble sort with invariant proof
fun bubble_sort(mut arr: Array(Int)) -> Array(Int)
  ensures: is_sorted(arr)
{
  for i in 0..length(arr) {
    // Invariant: arr[0..i] contains i smallest elements in sorted order
    
    for j in 0..length(arr) - i - 1 {
      if arr[j] > arr[j + 1] {
        swap(arr, j, j + 1)
      }
    }
  }
  arr
}

// Loop variant (for termination)
fun compute_until_convergence(mut x: f32, epsilon: f32) -> f32
  ensures: converged(result, epsilon)
{
  // Variant: distance_to_fixpoint(x)
  // Decreases with each iteration
  
  loop {
    let old_x = x
    x = f(x)
    
    if (x - old_x).abs() < epsilon {
      break
    }
  }
  x
}
```

### Dataflow Properties

```axiom
// No use-after-free
prop no_use_after_free: forall ptr: Pointer(T), use: Access(T),
  freed(ptr) before use => !valid(use)

// No buffer overflow
prop no_buffer_overflow: forall buf: Buffer(T), idx: Nat,
  access(buf, idx) => idx < capacity(buf)

// Data race freedom
prop data_race_free: forall var: Var, access1: Access, access2: Access,
  concurrent(access1, access2) => 
  (both_reads(access1, access2) || has_lock(var))
```

---

## Decision Procedures

### SMT Solving

```axiom
// Linear arithmetic
theorem linear_satisfiable: forall x: Int, y: Int,
  exists: 2*x + 3*y == 10 && x >= 0 && y >= 0
{
  // SMT solver finds: x = 2, y = 2
  witness x = 2, y = 2
}

// Bit-vector arithmetic
theorem bv_property: forall x: BitVec(8),
  (x & 0xFF) == x

// Quantifier elimination
theorem exists_qe: forall y: Int,
  (exists x: Int, x * x == y) iff (y >= 0 && is_perfect_square(y))
```

### Constraint Solving

```axiom
// Constraint propagation
fun solve_constraints(constraints: Vec<Constraint>) -> Option<Solution> {
  let mut domains = initialize_domains(constraints)
  
  repeat {
    let old_domains = domains.clone()
    
    for constraint in constraints {
      propagate(constraint, &mut domains)
    }
    
    if domains == old_domains {
      break
    }
  }
  
  if all_singletons(domains) {
    Some(extract_solution(domains))
  } else {
    None
  }
}
```

---

## Automated Reasoning

### Forward Chaining

```axiom
// Rule-based inference
rule transitivity_of_less_than: forall a: Int, b: Int, c: Int,
  (a < b) && (b < c) => (a < c)

rule addition_preserves_order: forall a: Int, b: Int, c: Int,
  (a < b) => (a + c < b + c)

// Forward chaining engine
fun forward_chain(facts: Vec<Fact>, rules: Vec<Rule>) -> Vec<Fact> {
  mut derived = facts.clone()
  
  repeat {
    let mut new_facts = vec![]
    
    for rule in rules {
      for fact in &derived {
        if rule.matches(fact) {
          let consequence = rule.apply(fact)
          new_facts.push(consequence)
        }
      }
    }
    
    if new_facts.is_empty() {
      break
    }
    
    derived.extend(new_facts)
  }
  
  derived
}
```

### Backward Chaining

```axiom
// Goal-directed proof search
fun prove(goal: Goal, rules: Vec<Rule>, facts: Vec<Fact>) -> Option<Proof> {
  // Check if goal is in facts
  if facts.contains(&goal) {
    return Some(Proof::Axiom(goal))
  }
  
  // Try each rule
  for rule in rules {
    if let Some(subgoals) = rule.unify(&goal) {
      let mut subproofs = vec![]
      
      for subgoal in subgoals {
        if let Some(proof) = prove(subgoal, rules.clone(), facts.clone()) {
          subproofs.push(proof)
        } else {
          break
        }
      }
      
      if subproofs.len() == subgoals.len() {
        return Some(Proof::RuleApplication(rule, subproofs))
      }
    }
  }
  
  None
}
```

---

## Performance Characteristics

### Proof Search Time
- **Simple arithmetic:** <1ms
- **List properties:** 1-10ms
- **Program correctness:** 10-100ms
- **Complex theorems:** 100ms-10s

### Proof Sizes
- **Simple proofs:** <100 lines
- **Complex proofs:** 1,000+ lines
- **Verified code:** 1:10 proof-to-code ratio

---

## Next Steps

- [PERFORMANCE_BENCHMARKS.md](PERFORMANCE_BENCHMARKS.md)
- [SECURITY_MODEL.md](SECURITY_MODEL.md)
- [IDE_INTEGRATION.md](IDE_INTEGRATION.md)

---

**AXIOM Specification** - Complete formal verification language reference!
