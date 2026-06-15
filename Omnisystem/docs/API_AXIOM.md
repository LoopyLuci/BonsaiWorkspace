# AXIOM Runtime API Reference

**Complete API reference for formal logic and theorem proving**

---

## Module Overview

The AXIOM runtime provides:
- **First-Order Logic**: Formulas, quantifiers, predicates
- **Type System**: Inference, unification, constraints
- **Theorem Proving**: Automated and interactive proof generation
- **Specifications**: Contracts with preconditions and postconditions
- **Proof Checking**: Validate proof correctness

---

## Core Types

### Formula

**First-order logic formulas**

```rust
pub enum Formula {
    Atom(String),
    Not(Box<Formula>),
    And(Box<Formula>, Box<Formula>),
    Or(Box<Formula>, Box<Formula>),
    Implies(Box<Formula>, Box<Formula>),
    Iff(Box<Formula>, Box<Formula>),
    ForAll(String, Box<Formula>),
    Exists(String, Box<Formula>),
    Predicate(String, Vec<String>),
    Equals(String, String),
    NotEquals(String, String),
}

impl Formula {
    pub fn simplify(&self) -> Formula
    pub fn is_cnf(&self) -> bool
    pub fn to_cnf(&self) -> Formula
    pub fn to_string(&self) -> String
    pub fn contains_variable(&self, var: &str) -> bool
    pub fn free_variables(&self) -> HashSet<String>
}
```

**Example:**
```rust
let atom = Formula::Atom("P(x)".to_string())
let neg = Formula::Not(Box::new(atom))
let eq = Formula::Equals("x".to_string(), "5".to_string())
let simplified = eq.simplify()
```

---

### Type System

**Static and inferred types**

```rust
pub enum Type {
    Unit,
    Bool,
    Int,
    Float,
    String,
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Function(Vec<Type>, Box<Type>),
    Generic(String),
    Custom(String),
}

impl Type {
    pub fn is_numeric(&self) -> bool
    pub fn is_comparable(&self) -> bool
    pub fn unify(&self, other: &Type) -> Result<Substitution>
}
```

**Example:**
```rust
let int_type = Type::Int
let array_type = Type::Array(Box::new(Type::String))
let func_type = Type::Function(
    vec![Type::Int, Type::String],
    Box::new(Type::Bool)
)
```

---

### Type Inference

**Automatic type inference with constraints**

```rust
pub struct TypeInference {
    constraints: Vec<Constraint>,
    substitution: Substitution,
}

impl TypeInference {
    pub fn new() -> Self
    pub fn infer(&mut self, expr: &str) -> Result<Type>
    pub fn add_constraint(&mut self, constraint: Constraint)
    pub fn solve(&mut self) -> Result<Substitution>
    pub fn get_substitution(&self) -> &Substitution
}

pub struct Constraint {
    pub lhs: Type,
    pub rhs: Type,
}

pub struct Substitution {
    mappings: HashMap<String, Type>,
}

impl Substitution {
    pub fn new() -> Self
    pub fn add(&mut self, var: String, ty: Type)
    pub fn apply(&self, ty: &Type) -> Type
    pub fn compose(&self, other: &Substitution) -> Substitution
}
```

**Example:**
```rust
let mut inf = TypeInference::new()
let t1 = inf.infer("2 + 3")?  // Type::Int
let t2 = inf.infer("fun(x) { x + 1 }")?  // Type::Function([Int], Box::new(Int))
```

---

### Theorem Prover

**Automated and interactive proof generation**

```rust
pub struct TheoremProver {
    axioms: Vec<Formula>,
    theorems: HashMap<String, Proof>,
    timeout: Duration,
    depth_limit: usize,
}

impl TheoremProver {
    pub fn new() -> Self
    pub fn with_timeout(mut self, timeout: Duration) -> Self
    pub fn with_depth_limit(mut self, limit: usize) -> Self
    pub fn add_axiom(&mut self, axiom: &str) -> Result<()>
    pub fn prove(&self, formula: &Formula) -> Result<Proof>
    pub fn prove_by_induction(
        &self,
        formula: &str,
        var: &str,
        base_case: impl Fn(&mut Self) -> Result<Proof>,
        inductive_step: impl Fn(&mut Self, &Proof) -> Result<Proof>
    ) -> Result<Proof>
    pub fn get_proof_trace(&self) -> Vec<ProofStep>
}

pub struct Proof {
    pub theorem: Formula,
    pub steps: Vec<ProofStep>,
    pub dependencies: HashMap<usize, Vec<usize>>,
}

pub struct ProofStep {
    pub num: usize,
    pub formula: Formula,
    pub justification: Justification,
}

pub enum Justification {
    Axiom,
    ModusPonens { from_1: usize, from_2: usize },
    UniversalInstantiation { from: usize, substitution: String },
    ExistentialGeneralization { from: usize, var: String },
    Assumption,
    Derived { rule: String, from: Vec<usize> },
}
```

**Example:**
```rust
let mut prover = TheoremProver::new()
    .with_timeout(Duration::from_secs(30))

prover.add_axiom("A | ¬A")?  // Law of excluded middle

let theorem = Formula::Implies(
    Box::new(Formula::Atom("P".to_string())),
    Box::new(Formula::Atom("P".to_string()))
)

let proof = prover.prove(&theorem)?
println!("Proved in {} steps", proof.steps.len())
```

---

### Specifications

**Program contracts with invariants**

```rust
pub struct Specification {
    pub preconditions: Vec<Formula>,
    pub postconditions: Vec<Formula>,
    pub invariants: Vec<Formula>,
}

impl Specification {
    pub fn new() -> Self
    pub fn add_precondition(mut self, cond: Formula) -> Self
    pub fn add_postcondition(mut self, cond: Formula) -> Self
    pub fn add_invariant(mut self, inv: Formula) -> Self
    pub fn verify(&self, prover: &TheoremProver) -> Result<bool>
}
```

**Example:**
```rust
let spec = Specification::new()
    .add_precondition(Formula::NotEquals("b".to_string(), "0".to_string()))
    .add_postcondition(Formula::Equals(
        "result * b".to_string(),
        "a".to_string()
    ))
```

---

### Proof Builder

**Interactive proof construction**

```rust
pub struct ProofBuilder {
    theorem: String,
    steps: Vec<ProofStep>,
}

impl ProofBuilder {
    pub fn new(theorem: &str) -> Self
    pub fn step(&mut self, formula: &str, justification: Justification) -> &mut Self
    pub fn build(self) -> Result<Proof>
}
```

**Example:**
```rust
let proof = ProofBuilder::new("P(x) → P(x)")
    .step("Assume P(x)", Justification::Assumption)
    .step("From assumption, P(x)", Justification::Derived {
        rule: "Identity".to_string(),
        from: vec![0]
    })
    .build()?
```

---

### Proof Checker

**Validate proof correctness**

```rust
pub struct ProofChecker {
    axioms: HashSet<String>,
    rules: Vec<ValidationRule>,
}

impl ProofChecker {
    pub fn new() -> Self
    pub fn verify(&self, proof: &Proof) -> Result<VerificationReport>
    pub fn add_axiom(&mut self, axiom: &str)
}

pub struct VerificationReport {
    pub valid: bool,
    pub num_lines: usize,
    pub num_gaps: usize,
    pub errors: Vec<ProofError>,
}

pub struct ProofError {
    pub line: usize,
    pub message: String,
}

pub enum ValidationRule {
    AxiomExists(String),
    ModusPonensValid,
    UniversalSubstitution,
    ExistentialIntro,
    ConsistencyCheck,
}
```

**Example:**
```rust
let checker = ProofChecker::new()
let report = checker.verify(&proof)?
if report.valid {
    println!("Proof valid with {} lines", report.num_lines)
} else {
    for error in report.errors {
        println!("Line {}: {}", error.line, error.message)
    }
}
```

---

## Error Types

### ProofError

**Theorem proving errors**

```rust
pub enum ProofError {
    TimeoutExceeded,
    DepthLimitReached,
    AssertionFailed(String),
    InvalidFormula(String),
    UnprovenLemma(String),
    InconsistentAxioms,
    TypeMismatch { expected: Type, got: Type },
}
```

### TypeCheckError

**Type inference and checking errors**

```rust
pub enum TypeCheckError {
    CannotUnify { t1: Type, t2: Type },
    UnboundVariable(String),
    UnboundTypeVariable(String),
    OccursCheck,
    AmbiguousType,
}
```

---

## Usage Patterns

### Basic Proving

```rust
let mut prover = TheoremProver::new()

// Define axioms
prover.add_axiom("true")?

// Prove theorem
let theorem = Formula::Atom("true".to_string())
let proof = prover.prove(&theorem)?

for step in &proof.steps {
    println!("{}: {}", step.num, step.formula)
}
```

### Type Checking Program

```rust
let mut inf = TypeInference::new()

// Infer types
let t1 = inf.infer("2 + 3")?
let t2 = inf.infer("[1, 2, 3]")?
let t3 = inf.infer("fun(x) { x }")?

// Check compatibility
let sub = t1.unify(&Type::Int)?
```

### Contract Verification

```rust
let spec = Specification::new()
    .add_precondition(Formula::NotEquals("n".to_string(), "0".to_string()))
    .add_postcondition(Formula::Equals(
        "factorial(n)".to_string(),
        "n * factorial(n-1)".to_string()
    ))
    .add_invariant(Formula::Atom("result >= 0".to_string()))

let mut prover = TheoremProver::new()
let verified = spec.verify(&prover)?
```

---

## Examples

### Simple Tautology Proof

```rust
let mut prover = TheoremProver::new()
prover.add_axiom("A | ¬A")?

let tautology = Formula::Or(
    Box::new(Formula::Atom("P".to_string())),
    Box::new(Formula::Not(Box::new(Formula::Atom("P".to_string()))))
)

let proof = prover.prove(&tautology)?
println!("✓ Tautology proved")
```

### List Length Verification

```rust
// Verify: length(append(xs, ys)) = length(xs) + length(ys)
let mut prover = TheoremProver::new()
prover.add_axiom("length([]) = 0")?
prover.add_axiom("length([x|xs]) = 1 + length(xs)")?

let theorem = "∀xs, ys. length(append(xs, ys)) = length(xs) + length(ys)"
let proof = prover.prove_by_induction(
    theorem,
    "xs",
    |prover| {
        // Base case: xs = []
        prover.prove(&Formula::Equals(
            "length(append([], ys))".to_string(),
            "length(ys)".to_string()
        ))
    },
    |prover, ih| {
        // Inductive case
        prover.prove(&Formula::Equals(
            "length(append([x|xs], ys))".to_string(),
            "1 + length(xs) + length(ys)".to_string()
        ))
    }
)?
```

---

## Testing

### Proof Tests

```rust
#[test]
fn test_tautology() {
    let mut p = TheoremProver::new()
    let f = Formula::Or(
        Box::new(Formula::Atom("P".to_string())),
        Box::new(Formula::Not(Box::new(Formula::Atom("P".to_string()))))
    )
    assert!(p.prove(&f).is_ok())
}

#[test]
fn test_type_inference() {
    let mut inf = TypeInference::new()
    let t = inf.infer("2 + 3").unwrap()
    assert_eq!(t, Type::Int)
}
```

---

## Performance Notes

- **Proof search** is exponential in worst case
- Use **timeout limits** to prevent infinite search
- **Memoization** improves performance significantly
- **Constraint propagation** reduces search space
- **Bidirectional proving** combines forward and backward search

---

## See Also
- [AXIOM_LANGUAGE_GUIDE.md](AXIOM_LANGUAGE_GUIDE.md) - Language tutorial
- [TUTORIAL_VERIFICATION.md](TUTORIAL_VERIFICATION.md) - Verification example
- [AXIOM_LANGUAGE_SPECIFICATION.md](AXIOM_LANGUAGE_SPECIFICATION.md) - Formal spec

---

**Last Updated**: 2026-06-15
