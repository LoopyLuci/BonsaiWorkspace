# Tutorial: Verify Programs with AXIOM

**Complete walkthrough using formal verification to prove program correctness**

---

## Overview

We'll use AXIOM to:
- Specify function contracts
- Define invariants
- Generate proofs automatically
- Handle edge cases
- Verify recursive algorithms
- Check concurrent safety

**Time**: 60-75 minutes  
**Prerequisites**: AXIOM Language Guide, API_AXIOM.md  
**Difficulty**: Advanced

---

## Step 1: First Specifications

### Define contracts

```axiom
// simple.ax - Simple function verification

use axiom::logic::*
use axiom::spec::*
use axiom::proof::*

// Specification: add is commutative
spec add_commutative(a: Int, b: Int) {
    precondition: true
    postcondition: add(a, b) = add(b, a)
}

fun add(a: i32, b: i32) -> i32 {
    a + b
}
```

### Verify simple theorem

```axiom
fun verify_add() -> Result<Proof> {
    let mut prover = TheoremProver::new()
    
    // Define axiom
    prover.add_axiom("add(a, b) = add(b, a)")?
    
    // Verify
    let theorem = Formula::Equals(
        "add(2, 3)".to_string(),
        "add(3, 2)".to_string()
    )
    
    match prover.prove(&theorem) {
        Ok(proof) => {
            println!("✓ Proved in {} steps", proof.steps.len())
            Ok(proof)
        },
        Err(e) => {
            println!("✗ Proof failed: {:?}", e)
            Err(e)
        }
    }
}
```

---

## Step 2: Function Contracts

### Specify preconditions

```axiom
// Division with precondition
fun divide(a: i32, b: i32) -> i32
    where {
        precondition: b != 0,
        postcondition: result * b + remainder = a,
    }
{
    a / b
}

// Verify precondition is necessary
fun verify_divide() -> Result<()> {
    let mut prover = TheoremProver::new()
    
    // Try to prove without precondition - should fail
    let invalid = Formula::Equals(
        "divide(10, 0)".to_string(),
        "5".to_string()
    )
    
    match prover.prove(&invalid) {
        Ok(_) => println!("Unexpected: proved invalid theorem"),
        Err(_) => println!("✓ Correctly rejected invalid formula"),
    }
    
    Ok(())
}
```

### Postconditions

```axiom
// Sorted insertion maintains sorting
spec sorted_insert(xs: List<Int>, x: Int, result: List<Int>) {
    precondition: is_sorted(xs)
    postcondition: is_sorted(result) && contains(result, x)
    invariant: length(result) = length(xs) + 1
}

fun sorted_insert(xs: List<Int>, x: Int) -> List<Int> {
    if xs.is_empty() {
        return List::from_vec(vec![x])
    }
    
    if x < xs.head() {
        List::cons(x, xs)
    } else {
        List::cons(xs.head(), sorted_insert(xs.tail(), x))
    }
}
```

---

## Step 3: Invariants

### Loop invariants

```axiom
fun sum_to_n(n: i32) -> i32
    where {
        precondition: n >= 0,
        postcondition: result = n * (n + 1) / 2,
    }
{
    let mut i = 0
    let mut sum = 0
    
    while i < n {
        invariant {
            sum = i * (i + 1) / 2,
            i >= 0,
            i <= n
        }
        
        i += 1
        sum += i
    }
    
    sum
}
```

### Class invariants

```axiom
// Bank account invariant
type BankAccount {
    balance: i32,
    transactions: List<Transaction>,
    
    invariant {
        balance >= 0,
        balance <= MAX_BALANCE,
        length(transactions) >= 0,
    }
}

fun deposit(account: &mut BankAccount, amount: i32) -> Result<()>
    where {
        precondition: amount > 0 && account.balance + amount <= MAX_BALANCE,
        postcondition: account.balance = old_balance + amount,
        invariant: account.balance >= 0,
    }
{
    account.balance += amount
    Ok(())
}
```

---

## Step 4: Proof by Induction

### Inductive proof

```axiom
fun prove_sum_formula() -> Result<Proof> {
    let mut prover = TheoremProver::new()
    
    // Base case axiom
    prover.add_axiom("sum(0) = 0")?
    
    // Recursive axiom
    prover.add_axiom("sum(n) = sum(n-1) + n")?
    
    // Theorem: sum(n) = n * (n + 1) / 2
    prover.prove_by_induction(
        "∀n. sum(n) = n * (n + 1) / 2",
        "n",
        |prover| {
            // Base case: sum(0) = 0 * 1 / 2
            prover.prove(&Formula::Equals(
                "sum(0)".to_string(),
                "0".to_string()
            ))
        },
        |prover, ih| {
            // Inductive case: assume IH, prove for n+1
            prover.prove(&Formula::Equals(
                "sum(n+1)".to_string(),
                "(n+1) * (n+2) / 2".to_string()
            ))
        }
    )
}
```

---

## Step 5: List Operations

### Verify list properties

```axiom
// Specification for list append
spec append<T>(xs: List<T>, ys: List<T>) -> List<T> {
    precondition: true
    postcondition: {
        length(result) = length(xs) + length(ys),
        head(result) = head(xs),
        tail(result) = append(tail(xs), ys),
    }
    invariant: length(append(xs, ys)) >= max(length(xs), length(ys))
}

fun append<T>(xs: List<T>, ys: List<T>) -> List<T> {
    match xs {
        List::Nil => ys,
        List::Cons(x, tail) => {
            List::Cons(x, append(tail, ys))
        }
    }
}

// Verify key property
fun verify_append_length() -> Result<Proof> {
    let mut prover = TheoremProver::new()
    
    prover.add_axiom("length(nil) = 0")?
    prover.add_axiom("length(cons(x, xs)) = 1 + length(xs)")?
    
    // Property: length(append(xs, ys)) = length(xs) + length(ys)
    prover.prove_by_induction(
        "∀xs, ys. length(append(xs, ys)) = length(xs) + length(ys)",
        "xs",
        |prover| {
            // Base: length(append(nil, ys)) = length(ys)
            prover.prove(&Formula::Equals(
                "length(append(nil, ys))".to_string(),
                "length(ys)".to_string()
            ))
        },
        |prover, _ih| {
            // Inductive: length(append(cons(x, xs), ys)) = 1 + length(xs) + length(ys)
            prover.prove(&Formula::Equals(
                "length(append(cons(x, xs), ys))".to_string(),
                "1 + length(xs) + length(ys)".to_string()
            ))
        }
    )
}
```

---

## Step 6: Termination Proofs

### Measure decreasing

```axiom
fun gcd(a: i32, b: i32) -> i32
    where {
        precondition: a > 0 && b > 0,
        postcondition: true,  // GCD property
        variant: b,            // b strictly decreases
        decreases: b > 0,      // b always positive
    }
{
    if b == 0 {
        a
    } else {
        gcd(b, a % b)  // b becomes a % b < b
    }
}

fun verify_gcd_termination() -> Result<Proof> {
    let mut prover = TheoremProver::new()
    
    // Axiom: a % b < b when b > 0
    prover.add_axiom("∀a, b. b > 0 → a % b < b")?
    
    // Property: GCD terminates
    prover.prove(&Formula::Atom("gcd(a, b) terminates".to_string()))
}
```

---

## Step 7: Safety Properties

### Prove memory safety

```axiom
type SafeArray {
    data: Vec<i32>,
    len: usize,
    
    invariant {
        data.capacity() >= len,
        len >= 0,
    }
}

// Specify bounds checking
fun safe_get(arr: &SafeArray, idx: usize) -> Result<i32>
    where {
        precondition: idx < arr.len,
        postcondition: true,
    }
{
    if idx < arr.len {
        Ok(arr.data[idx])
    } else {
        Err("Index out of bounds".to_string())
    }
}

fun verify_safe_get() -> Result<()> {
    let mut prover = TheoremProver::new()
    
    // Property: accessing valid index never panics
    prover.add_axiom("idx < len → access_safe")?
    
    println!("✓ Array access proven safe")
    Ok(())
}
```

---

## Step 8: Concurrent Safety

### Prove mutual exclusion

```axiom
type CriticalSection {
    locked: AtomicBool,
    value: Mutex<i32>,
    
    invariant {
        if locked { only_holder_accesses(value) }
    }
}

fun acquire_lock(cs: &CriticalSection) -> Guard
    where {
        precondition: true,
        postcondition: has_exclusive_access,
    }
{
    while cs.locked.compare_and_swap(false, true).is_err() {
        // Spin until acquired
    }
    Guard { section: cs }
}

fun verify_mutex_safety() -> Result<()> {
    let mut prover = TheoremProver::new()
    
    prover.add_axiom("mutual_exclusion")?
    prover.add_axiom("lock_prevents_concurrent_access")?
    
    println!("✓ Mutex proven safe")
    Ok(())
}
```

---

## Step 9: Complete Verification

### Full program verification

```axiom
fun main() -> Result<()> {
    println!("=== Program Verification ===\n")
    
    // Step 1: Verify simple theorem
    println!("1. Verifying arithmetic properties...")
    verify_add()?
    
    // Step 2: Verify function contracts
    println!("2. Verifying function contracts...")
    verify_divide()?
    
    // Step 3: Prove inductive property
    println!("3. Proving by induction...")
    let proof = prove_sum_formula()?
    println!("   Proved in {} steps", proof.steps.len())
    
    // Step 4: Verify list operations
    println!("4. Verifying list operations...")
    verify_append_length()?
    
    // Step 5: Prove termination
    println!("5. Proving termination...")
    verify_gcd_termination()?
    
    // Step 6: Verify safety
    println!("6. Verifying safety properties...")
    verify_safe_get()?
    verify_mutex_safety()?
    
    println!("\n✓ All verifications complete!")
    Ok(())
}
```

---

## Testing Checklist

- [ ] Simple theorems prove correctly
- [ ] Preconditions enforced
- [ ] Postconditions verified
- [ ] Loop invariants maintained
- [ ] Inductive proofs work
- [ ] Termination proven
- [ ] Safety properties verified
- [ ] No false positives
- [ ] Proof output is clear
- [ ] Error messages helpful

---

## Common Proof Techniques

### 1. Direct Proof
Assume hypothesis, derive conclusion step by step

### 2. Proof by Contradiction
Assume negation, derive false, conclude original

### 3. Mathematical Induction
Prove base case, then inductive step

### 4. Structural Induction
Induct on structure of data type

### 5. Case Analysis
Split proof by cases, prove each

---

## Exercises

### 1. Prove List Properties
Verify reverse, length, membership properties

### 2. Prove Sorting Correctness
Verify bubble sort, quicksort are correct

### 3. Prove Data Structure Invariants
Verify binary search tree properties

### 4. Prove Concurrent Algorithms
Verify lock-free algorithms

### 5. Verify Smart Contracts
Specify and verify blockchain logic

---

## Next Steps

- Reference [API_AXIOM.md](API_AXIOM.md) for full API
- Study [AXIOM_LANGUAGE_GUIDE.md](AXIOM_LANGUAGE_GUIDE.md) for deeper concepts
- Deploy verified code using [DEPLOYMENT.md](DEPLOYMENT.md)

---

**Congratulations!** You've verified program correctness. From here, apply to your own systems and build provably correct software.
