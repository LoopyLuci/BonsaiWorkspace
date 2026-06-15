# AXIOM Language Guide - Formal Verification

**Verify program correctness through automated theorem proving and formal logic**

---

## Table of Contents
1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [First-Order Logic](#first-order-logic)
4. [Type System](#type-system)
5. [Type Inference](#type-inference)
6. [Theorem Proving](#theorem-proving)
7. [Specifications](#specifications)
8. [Proof Checking](#proof-checking)
9. [Advanced Topics](#advanced-topics)

---

## Introduction

AXIOM is a formal verification language for:
- **Correctness Proofs**: Mathematically verify program behavior
- **Type Safety**: Advanced type inference and checking
- **Theorem Proving**: Automated proof generation
- **Specification**: Preconditions, postconditions, invariants

### Quick Facts
- **Logic**: First-order logic with quantifiers
- **Types**: Unit, Bool, Int, Float, String, Array, Function
- **Inference**: Full type unification and constraint solving
- **Proofs**: Automated generation with user guidance
- **Performance**: O(n³) type inference with memoization

---

## Getting Started

### Your First Proof

```axiom
// simple.ax
use axiom::logic::*
use axiom::proof::*

fun main() {
    // Define a simple theorem
    let theorem = Formula::Equals(
        "2 + 2",
        "4"
    )
    
    // Create prover
    let prover = TheoremProver::new()
    
    // Prove theorem
    match prover.prove(&theorem) {
        Ok(proof) => {
            println!("✓ Theorem proved!")
            println!("Steps: {}", proof.steps.len())
        },
        Err(e) => {
            println!("✗ Proof failed: {:?}", e)
        }
    }
}
```

### Running the Prover

```bash
omnisystem verify simple.ax
# ✓ Theorem proved!
# Steps: 3
```

---

## First-Order Logic

### Formulas

```axiom
use axiom::logic::*

// Atomic formulas
let atom1 = Formula::Atom("P(x)")
let atom2 = Formula::Atom("Q(y)")

// Negation
let neg = Formula::Not(Box::new(atom1))

// Conjunction (AND)
let and = Formula::And(
    Box::new(atom1),
    Box::new(atom2)
)

// Disjunction (OR)
let or = Formula::Or(
    Box::new(atom1),
    Box::new(atom2)
)

// Implication (→)
let imp = Formula::Implies(
    Box::new(atom1),
    Box::new(atom2)
)

// Biconditional (↔)
let iff = Formula::Iff(
    Box::new(atom1),
    Box::new(atom2)
)
```

### Quantifiers

```axiom
// Universal quantifier (∀)
let forall = Formula::ForAll(
    "x".to_string(),
    Box::new(Formula::Atom("P(x)"))
)

// Existential quantifier (∃)
let exists = Formula::Exists(
    "y".to_string(),
    Box::new(Formula::Atom("Q(y)"))
)

// Multiple quantifiers
let complex = Formula::ForAll(
    "x".to_string(),
    Box::new(Formula::Exists(
        "y".to_string(),
        Box::new(Formula::Atom("Loves(x, y)"))
    ))
)
```

### Predicates and Equality

```axiom
// Predicate
let pred = Formula::Predicate("Even", vec!["n".to_string()])

// Equality
let eq = Formula::Equals("x".to_string(), "5".to_string())

// Inequality
let neq = Formula::NotEquals("x".to_string(), "0".to_string())

// Simplification
let simplified = eq.simplify()
```

---

## Type System

### Basic Types

```axiom
use axiom::types::*

// Primitive types
let unit = Type::Unit
let bool_type = Type::Bool
let int_type = Type::Int
let float_type = Type::Float
let string_type = Type::String

// Composite types
let array_type = Type::Array(Box::new(Type::Int))
let tuple_type = Type::Tuple(vec![
    Type::Int,
    Type::String
])

// Function type: Int -> String -> Bool
let func_type = Type::Function(
    vec![Type::Int, Type::String],
    Box::new(Type::Bool)
)
```

### Custom Types

```axiom
// Define custom type
type Color {
    Red,
    Green,
    Blue,
}

// Define record type
type Person {
    name: String,
    age: Int,
    email: String,
}

// Polymorphic type
type Box<T> {
    value: T,
}

type List<T> {
    head: T,
    tail: List<T>,
}
```

### Generic Types

```axiom
// Generic function
fun id<T>(x: T) -> T {
    x
}

// Generic type constraint
fun first<T>(xs: List<T>) -> T {
    xs.head
}

// Multiple type parameters
fun pair<A, B>(a: A, b: B) -> (A, B) {
    (a, b)
}
```

---

## Type Inference

### Inference Examples

```axiom
use axiom::inference::*

// Create type inferencer
let mut inferencer = TypeInference::new()

// Infer type of expression
let expr1 = "2 + 3"
let type1 = inferencer.infer(expr1)?
// type1: Int

// Infer function type
let expr2 = "fun(x) { x + 1 }"
let type2 = inferencer.infer(expr2)?
// type2: Int -> Int

// Infer with constraints
let expr3 = "if true { 1 } else { 2 }"
let type3 = inferencer.infer(expr3)?
// type3: Int
```

### Type Unification

```axiom
// Unify types
let mut subst = Substitution::new()

// X = Int
subst.add("X".to_string(), Type::Int)

// Y = String -> Bool
subst.add("Y".to_string(), 
    Type::Function(
        vec![Type::String],
        Box::new(Type::Bool)
    )
)

// Apply substitution
let result = subst.apply(&var_x)
```

### Constraint Solving

```axiom
// Collect constraints
let mut constraints = vec![]
constraints.push((var_x, Type::Int))
constraints.push((var_y, Type::String))

// Check consistency
if are_consistent(&constraints) {
    println!("Constraints satisfiable")
} else {
    println!("Contradiction found")
}
```

---

## Theorem Proving

### Automated Proving

```axiom
use axiom::proof::*

// Create theorem prover
let prover = TheoremProver::new()
    .with_timeout(Duration::from_secs(30))
    .with_depth_limit(100)

// Add axioms (basic truths)
prover.add_axiom("A | ¬A")  // Law of excluded middle
prover.add_axiom("(A → B) ∧ A → B")  // Modus ponens

// Prove theorem
let theorem = "P(x) → P(x)"
match prover.prove(&formula_from_string(theorem)?) {
    Ok(proof) => {
        println!("✓ Proved in {} steps", proof.steps.len())
        for step in &proof.steps {
            println!("  {}: {}", step.num, step.formula)
        }
    },
    Err(e) => println!("✗ Could not prove: {:?}", e)
}
```

### Proof Structure

```axiom
use axiom::proof::*

type Proof {
    theorem: Formula,
    steps: Vec<ProofStep>,
    dependencies: HashMap<usize, Vec<usize>>,
}

type ProofStep {
    num: usize,
    formula: Formula,
    justification: Justification,
}

enum Justification {
    Axiom,
    ModusPonens { from_1: usize, from_2: usize },
    UniversalInstantiation { from: usize, substitution: Substitution },
    ExistentialGeneralization { from: usize, var: String },
    Assumption,
    Derived { rule: String, from: Vec<usize> },
}
```

### Interactive Proving

```axiom
// Manual proof construction
let proof = ProofBuilder::new("∀x. P(x) → P(x)")
    .step("Assume P(x)", Justification::Assumption)
    .step("From assumption, P(x)", Justification::Derived { 
        rule: "Identity",
        from: vec![1]
    })
    .step("Therefore ∀x. P(x) → P(x)", Justification::Derived {
        rule: "Universal generalization",
        from: vec![2]
    })
    .build()?
```

---

## Specifications

### Preconditions and Postconditions

```axiom
use axiom::spec::*

type Specification {
    preconditions: Vec<Formula>,
    postconditions: Vec<Formula>,
    invariants: Vec<Formula>,
}

fun divide(a: Int, b: Int) -> Int
    where {
        // Precondition: divisor must be non-zero
        precondition: b != 0,
        postcondition: result * b + remainder == a,
        invariant: b > 0
    }
{
    a / b
}

// Function contracts
fun sorted_insert(xs: List<Int>, x: Int) -> List<Int>
    where {
        precondition: is_sorted(xs),
        postcondition: is_sorted(result) && contains(result, x),
        invariant: len(result) == len(xs) + 1
    }
{
    // Implementation
}
```

### Invariants

```axiom
// Class invariant
type BankAccount {
    balance: Int,
    
    invariant {
        balance >= 0,
        balance <= MAX_BALANCE
    }
}

// Loop invariant
fun sum_to_n(n: Int) -> Int {
    let mut i = 0
    let mut sum = 0
    
    while i < n {
        invariant {
            sum == i * (i + 1) / 2,  // Sum formula
            i >= 0,
            i <= n
        }
        
        i += 1
        sum += i
    }
    
    sum
}
```

### Termination Proofs

```axiom
// Prove termination
fun gcd(a: Int, b: Int) -> Int
    where {
        variant: b,  // Variant: value that strictly decreases
        decreases: b > 0  // Proof that variant decreases
    }
{
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}
```

---

## Proof Checking

### Checking Proofs

```axiom
use axiom::check::*

// Create proof checker
let checker = ProofChecker::new()

// Load proof
let proof = parse_proof_file("proof.ax")?

// Check validity
match checker.verify(&proof) {
    Ok(report) => {
        println!("✓ Proof valid!");
        println!("Lines: {}", report.num_lines);
        println!("Gaps: {}", report.num_gaps);
    },
    Err(errors) => {
        println!("✗ Proof invalid:");
        for error in errors {
            println!("  Line {}: {}", error.line, error.message)
        }
    }
}
```

### Proof Validation Rules

```axiom
// Each proof step must satisfy rules
enum ValidationRule {
    AxiomExists(String),  // Axiom must be in knowledge base
    ModusPonensValid,     // Both premises and conclusion must exist
    UniversalSubstitution, // Substitution must be consistent
    ExistentialIntro,     // Variable must not occur free
    ConsistencyCheck,     // No contradictions
}
```

---

## Advanced Topics

### Inductive Proofs

```axiom
// Prove by induction
fun sum_formula(n: Int) -> Int {
    requires: n >= 0
    
    // Base case: P(0)
    if n == 0 {
        return 0  // 0 = 0 * (0 + 1) / 2
    }
    
    // Inductive step: assume P(k), prove P(k+1)
    let k_sum = sum_formula(n - 1)
    let result = k_sum + n
    
    // Verify: result = n * (n + 1) / 2
    assert_eq!(result, n * (n + 1) / 2)
    
    result
}
```

### Structural Proof

```axiom
// Proof by structural induction on lists
fun list_length<T>(xs: List<T>) -> Int {
    match xs {
        List::Nil => 0,
        List::Cons(_, tail) => 1 + list_length(tail)
    }
}

// Verified property
property len_concat<T> {
    ∀xs: List<T>, ys: List<T>.
    length(concat(xs, ys)) = length(xs) + length(ys)
}
```

### Temporal Logic

```axiom
use axiom::temporal::*

// Eventually property: eventually P holds
let eventually_p = Formula::Eventually(
    Box::new(Formula::Atom("P"))
)

// Always property: P holds at all future times
let always_p = Formula::Always(
    Box::new(Formula::Atom("P"))
)

// Until property: P holds until Q
let until = Formula::Until(
    Box::new(Formula::Atom("P")),
    Box::new(Formula::Atom("Q"))
)
```

---

## Complete Example: List Operations Verification

```axiom
use axiom::logic::*
use axiom::proof::*
use axiom::spec::*

// Define list type
type List<T> {
    Nil,
    Cons(T, Box<List<T>>),
}

// Length function
fun length<T>(xs: List<T>) -> Int {
    match xs {
        List::Nil => 0,
        List::Cons(_, tail) => 1 + length(*tail),
    }
}

// Append function
fun append<T>(xs: List<T>, ys: List<T>) -> List<T> 
    where {
        postcondition: length(result) == length(xs) + length(ys)
    }
{
    match xs {
        List::Nil => ys,
        List::Cons(x, tail) => {
            List::Cons(x, Box::new(append(*tail, ys)))
        }
    }
}

// Verified properties
property append_length<T> {
    ∀xs: List<T>, ys: List<T>.
    length(append(xs, ys)) = length(xs) + length(ys)
}

property append_assoc<T> {
    ∀xs: List<T>, ys: List<T>, zs: List<T>.
    append(append(xs, ys), zs) = append(xs, append(ys, zs))
}

// Proof of append_length
fun prove_append_length() -> Proof {
    let prover = TheoremProver::new()
    
    // Add axioms
    prover.add_axiom("length(Nil) = 0")
    prover.add_axiom("length(Cons(x, xs)) = 1 + length(xs)")
    
    // Prove by induction
    let theorem = "∀xs, ys. length(append(xs, ys)) = length(xs) + length(ys)"
    
    prover.prove_by_induction(
        theorem,
        "xs",
        |prover| {
            // Base case: xs = Nil
            prover.prove("length(append(Nil, ys)) = length(Nil) + length(ys)")
                .unwrap()
        },
        |prover, ih| {
            // Inductive case: assume IH for xs, prove for Cons(x, xs)
            prover.prove("length(append(Cons(x, xs), ys)) = length(Cons(x, xs)) + length(ys)")
                .unwrap()
        }
    )
}

fun main() -> Result<()> {
    let proof = prove_append_length()
    
    println!("✓ Proved: {}", proof.theorem);
    println!("  Steps: {}", proof.steps.len());
    
    Ok(())
}
```

---

## Best Practices

✅ **DO**
- Start with simple theorems
- Use axioms sparingly
- Test properties on small inputs
- Document proof strategies
- Use type-driven proving
- Verify base cases thoroughly
- Check postcondition completeness

❌ **DON'T**
- Assume unproven lemmas
- Use circular reasoning
- Over-constrain specifications
- Mix proof styles inconsistently
- Ignore edge cases
- Leave proof gaps
- Prove unprovable statements

---

## Performance Tips

1. **Memoize proof steps** to avoid redundant work
2. **Use bidirectional proving** (forward and backward)
3. **Constrain search space** with depth limits
4. **Leverage lemmas** to break down proofs
5. **Structure types well** for better inference

---

## Debugging

### Proof Failures

```axiom
// Enable debug output
let prover = TheoremProver::new()
    .with_debug(true)

// Get proof trace
let trace = prover.get_proof_trace()
for step in trace {
    println!("{}: {} (via {})", 
        step.num, 
        step.formula, 
        step.rule)
}
```

---

## See Also
- [API_AXIOM.md](API_AXIOM.md) - Complete API reference
- [TUTORIAL_VERIFICATION.md](TUTORIAL_VERIFICATION.md) - Verification example
- [AXIOM_LANGUAGE_SPECIFICATION.md](AXIOM_LANGUAGE_SPECIFICATION.md) - Formal spec

---

**Next**: [TUTORIAL_VERIFICATION.md](TUTORIAL_VERIFICATION.md) - Verify real programs
