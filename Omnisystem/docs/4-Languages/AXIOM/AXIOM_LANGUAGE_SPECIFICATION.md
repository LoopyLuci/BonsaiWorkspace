# AXIOM Language Specification v1.0
## The Omnisystem Formal Verification Language

---

## 1. OVERVIEW

**AXIOM** replaces Coq, Lean, Isabelle, Z3. It provides:
- Dependent types: Types depend on values
- Proof terms: Proofs are first-class values
- Formal verification: Mathematically guaranteed correctness
- SMT integration: Automatic theorem proving
- Runtime assertion checking: Proofs enforced at runtime

---

## 2. DEPENDENT TYPES

### 2.1 Basic Dependent Types

```axiom
// Type dependent on value
type Vec(n: nat) = union {
    Nil if n == 0,
    Cons(head: i32, tail: Vec(n - 1)) if n > 0
}

// Type-safe vector operations
fn vec_head(n: nat, v: Vec(n + 1)) -> i32 {
    match v {
        Cons(h, _) => h
    }
}

// Refined integer type
type Positive = {x: i32 | x > 0}
type NonNegative = {x: i32 | x >= 0}

fn abs_positive(x: i32) -> Positive {
    if x > 0 { x } else { -x }
}
```

### 2.2 Proofs as Values

```axiom
// Proof of a property
proof add_commutative: forall(a: i32, b: i32), a + b == b + a {
    by_induction(a) {
        case base(0):
            // 0 + b == b + 0
            rewrite(add_zero_right(b))
            rfl
        case step(n, ih):
            // (n + 1) + b == b + (n + 1)
            rewrite(add_succ_left(n, b))
            rewrite(add_succ_right(b, n))
            congruence(ih)
    }
}

// Use proof
fn commutative_add_5_3() -> (5 + 3 == 3 + 5) {
    return add_commutative(5, 3)
}
```

### 2.3 Theorem Declaration

```axiom
theorem list_append_assoc<T>(a: List<T>, b: List<T>, c: List<T>)
    -> append(append(a, b), c) == append(a, append(b, c))
{
    by_induction(a) {
        case base:
            // append(append([], b), c) == append([], append(b, c))
            simplify()
            rfl
        case step(h, t, ih):
            // append(append(h::t, b), c) == append(h::t, append(b, c))
            unfold(append)
            congruence(ih)
    }
}
```

---

## 3. REFINEMENT TYPES

### 3.1 Value-Dependent Types

```axiom
// Array with bounds guarantee
type BoundedArray(n: nat) = {
    data: [i32],
    len: nat,
    proof: len <= n
}

fn create_bounded(capacity: nat) -> BoundedArray(capacity) {
    return {
        data: [],
        len: 0,
        proof: zero_le_any(capacity)
    }
}

// Function with precondition
fn divide(a: i32, b: {x: i32 | x != 0}) -> i32 {
    return a / b
}

// Call with proof
let result = divide(10, {x: 2, proof: two_ne_zero})
```

### 3.2 Refinement Subtypes

```axiom
// Subtype relationship (automatically proved)
let x: {n: i32 | n > 5} = {7, by_arithmetic}

// Widening (subtype to supertype)
let y: {n: i32 | n > 0} = x  // Automatically widened

// Narrowing requires proof
let z: {n: i32 | n > 10} = narrow(x, proof: x_gt_10)
```

---

## 4. FORMAL SPECIFICATIONS

### 4.1 Contract Specification

```axiom
fn binary_search<T: Ordered>(
    arr: BoundedArray<T>,
    target: T
) -> Option<nat>
    requires: is_sorted(arr.data, arr.len)
    ensures: match result {
        Some(idx) => idx < arr.len && arr.data[idx] == target,
        None => forall(i in 0..arr.len), arr.data[i] != target
    }
{
    let mut left: nat = 0
    let mut right: nat = arr.len
    
    loop {
        if left >= right {
            return None
        }
        
        let mid = (left + right) / 2
        let cmp = compare(arr.data[mid], target)
        
        match cmp {
            Less => left = mid + 1,
            Greater => right = mid,
            Equal => return Some(mid)
        }
    }
}
```

### 4.2 Invariant Specification

```axiom
struct BSTNode<T> {
    value: T,
    left: Option<Box<BSTNode<T>>>,
    right: Option<Box<BSTNode<T>>>,
    
    invariant: is_bst_balanced(self) &&
               forall_left(x, x < value) &&
               forall_right(x, x > value)
}

fn insert_bst(tree: &mut BSTNode<i32>, value: i32) -> ()
    ensures: tree.invariant  // Invariant maintained
{
    if value < tree.value {
        match &mut tree.left {
            Some(left) => insert_bst(left, value),
            None => tree.left = Some(Box::new(BSTNode { value, ... }))
        }
    } else if value > tree.value {
        match &mut tree.right {
            Some(right) => insert_bst(right, value),
            None => tree.right = Some(Box::new(BSTNode { value, ... }))
        }
    }
}
```

---

## 5. PROOF TACTICS

### 5.1 Common Tactics

```axiom
proof example: forall(n: nat), n + 0 == n {
    by_induction(n) {
        case base:
            simp()           // Simplify using definitions
            rfl              // Reflexivity: x = x
        
        case step(n, ih):
            rewrite(add_succ_left(n, 0))  // Rewrite LHS
            congruence(ih)   // Apply inductive hypothesis
    }
}

proof equality_example: 2 + 2 == 4 {
    norm_num()  // Numeric normalization
}

proof logic_example: forall(p: prop, q: prop), p && q -> q {
    intro(p, q, h)  // Introduce variables and hypotheses
    exact(h.right)  // Provide exact proof term
}
```

### 5.2 Automated Reasoning

```axiom
proof auto_example: forall(x: i32, y: i32, z: i32),
    x + y == y + x && y + z == z + y -> x + z == z + x
{
    omega()  // Linear arithmetic solver
}

proof smt_example: forall(x: i32, y: i32, z: i32),
    x * y + z == x * y + z
{
    smt()   // SMT solver (Z3, etc.)
}
```

---

## 6. ASSERTION CHECKING

### 6.1 Runtime Assertions

```axiom
@runtime_check
fn factorial(n: {x: i32 | x >= 0}) -> {x: i32 | x > 0} {
    if n == 0 {
        return 1
    }
    let f = factorial(n - 1)
    let result = n * f
    
    // This assertion is proven at compile time, checked at runtime
    assert(result > 0, "factorial result always positive")
    return result
}
```

### 6.2 Assume Directives

```axiom
// Use assume for external invariants (from C libraries, etc.)
@assume
extern_fn get_current_time() -> {x: i32 | x >= 0}

fn process_time() -> void {
    let time = get_current_time()  // Assumed to be >= 0
    do_processing(time)
}
```

---

## 7. TYPE CLASS CONSTRAINTS

### 7.1 Ordered Type Class

```axiom
type class Ordered(T) {
    fn compare(a: T, b: T) -> Comparison
    
    proof refl: forall(a: T), compare(a, a) == Equal
    proof trans: forall(a: T, b: T, c: T),
        compare(a, b) == Less && compare(b, c) == Less ->
        compare(a, c) == Less
}

impl Ordered(i32) {
    fn compare(a: i32, b: i32) -> Comparison {
        if a < b { Less } else if a > b { Greater } else { Equal }
    }
}
```

### 7.2 Numeric Type Class

```axiom
type class Numeric(T) {
    fn add(a: T, b: T) -> T
    fn mul(a: T, b: T) -> T
    
    proof add_assoc: forall(a: T, b: T, c: T),
        add(add(a, b), c) == add(a, add(b, c))
    
    proof mul_comm: forall(a: T, b: T),
        mul(a, b) == mul(b, a)
}
```

---

## 8. EXAMPLE: VERIFIED QUICKSORT

```axiom
theorem quicksort_correct<T: Ordered>(arr: [T]) -> [T]
    ensures: is_sorted(result) && is_permutation(arr, result)
{
    fn partition(arr: [T], pivot: T) -> ([T], [T])
        ensures: {
            let (less, greater) = result
            is_partitioned(less, greater, pivot)
        }
    {
        // Implementation...
    }
    
    fn quicksort_helper(arr: [T]) -> [T]
        ensures: is_sorted(result) && is_permutation(arr, result)
    {
        match arr {
            [] => [],
            [pivot] => [pivot],
            arr => {
                let (less, greater) = partition(arr, arr[0])
                let sorted_less = quicksort_helper(less)
                let sorted_greater = quicksort_helper(greater)
                
                return append(append(sorted_less, [arr[0]]), sorted_greater)
                    : is_sorted(result) by proof_is_sorted(...)
                    : is_permutation(arr, result) by proof_permutation(...)
            }
        }
    }
    
    return quicksort_helper(arr)
}
```

---

## 9. COMPILATION

```
AXIOM Code
    ↓
[Lexer/Parser] → Proof AST
    ↓
[Type Checker] → Verified Proof Terms
    ↓
[SMT Solver] → Automated Verification
    ↓
[Eraser] → Runtime-Checkable Code
    ↓
[Code Generator] → LLVM IR + Assertions
    ↓
Native Executable (with runtime verification)
```

---

This specification enables AXIOM to provide mathematical proof of program correctness while generating efficient executable code.
