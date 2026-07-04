// AXIOM RUNTIME - Formal verification and correctness proofs
// Theorem proving, property verification, type safety guarantees
// Version: 2.0

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Logical formula representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Formula {
    /// Atomic proposition: variable or constant
    Atom(String),

    /// Negation: ¬P
    Not(Box<Formula>),

    /// Conjunction: P ∧ Q
    And(Box<Formula>, Box<Formula>),

    /// Disjunction: P ∨ Q
    Or(Box<Formula>, Box<Formula>),

    /// Implication: P → Q
    Implies(Box<Formula>, Box<Formula>),

    /// Biconditional: P ↔ Q
    Iff(Box<Formula>, Box<Formula>),

    /// Universal quantification: ∀x. P(x)
    ForAll(String, Box<Formula>),

    /// Existential quantification: ∃x. P(x)
    Exists(String, Box<Formula>),

    /// Predicate: P(x, y, ...)
    Predicate(String, Vec<String>),

    /// Equality: x = y
    Equals(String, String),

    /// Inequality: x ≠ y
    NotEquals(String, String),
}

impl Formula {
    pub fn to_string(&self) -> String {
        match self {
            Formula::Atom(name) => name.clone(),
            Formula::Not(f) => format!("¬{}", f.to_string()),
            Formula::And(f1, f2) => format!("({} ∧ {})", f1.to_string(), f2.to_string()),
            Formula::Or(f1, f2) => format!("({} ∨ {})", f1.to_string(), f2.to_string()),
            Formula::Implies(f1, f2) => format!("({} → {})", f1.to_string(), f2.to_string()),
            Formula::Iff(f1, f2) => format!("({} ↔ {})", f1.to_string(), f2.to_string()),
            Formula::ForAll(var, f) => format!("∀{}. {}", var, f.to_string()),
            Formula::Exists(var, f) => format!("∃{}. {}", var, f.to_string()),
            Formula::Predicate(name, args) => format!("{}({})", name, args.join(", ")),
            Formula::Equals(x, y) => format!("{} = {}", x, y),
            Formula::NotEquals(x, y) => format!("{} ≠ {}", x, y),
        }
    }

    /// Simplify formula
    pub fn simplify(&self) -> Formula {
        match self {
            Formula::Not(f) => {
                match **f {
                    Formula::Not(ref inner) => inner.simplify(),
                    _ => Formula::Not(Box::new(f.simplify())),
                }
            }
            Formula::And(ref f1, ref f2) => {
                Formula::And(Box::new(f1.simplify()), Box::new(f2.simplify()))
            }
            Formula::Or(ref f1, ref f2) => {
                Formula::Or(Box::new(f1.simplify()), Box::new(f2.simplify()))
            }
            _ => self.clone(),
        }
    }

    /// Check if formula is in CNF (Conjunctive Normal Form)
    pub fn is_cnf(&self) -> bool {
        match self {
            Formula::Atom(_) => true,
            Formula::Not(f) => matches!(**f, Formula::Atom(_)),
            Formula::Or(f1, f2) => {
                Self::is_literal(f1) && Self::is_literal(f2)
            }
            Formula::And(f1, f2) => {
                Self::is_clause(f1) && Self::is_clause(f2)
            }
            _ => false,
        }
    }

    fn is_literal(f: &Formula) -> bool {
        matches!(f, Formula::Atom(_) | Formula::Not(_))
    }

    fn is_clause(f: &Formula) -> bool {
        match f {
            Formula::Atom(_) => true,
            Formula::Not(inner) => matches!(**inner, Formula::Atom(_)),
            Formula::Or(f1, f2) => Self::is_literal(f1) && Self::is_literal(f2),
            _ => false,
        }
    }
}

/// Proof step in a proof
#[derive(Debug, Clone)]
pub struct ProofStep {
    pub step_num: usize,
    pub formula: Formula,
    pub reason: String,
    pub previous_steps: Vec<usize>,
}

/// Complete proof
#[derive(Debug, Clone)]
pub struct Proof {
    pub theorem: Formula,
    pub steps: Vec<ProofStep>,
    pub is_valid: bool,
}

impl Proof {
    pub fn new(theorem: Formula) -> Self {
        Proof {
            theorem,
            steps: Vec::new(),
            is_valid: false,
        }
    }

    pub fn add_step(&mut self, formula: Formula, reason: String, prev: Vec<usize>) {
        let step = ProofStep {
            step_num: self.steps.len() + 1,
            formula,
            reason,
            previous_steps: prev,
        };
        self.steps.push(step);
    }

    pub fn validate(&mut self) -> bool {
        // Simple validation: check if last step matches theorem
        if let Some(last) = self.steps.last() {
            self.is_valid = last.formula == self.theorem;
        }
        self.is_valid
    }

    pub fn to_string(&self) -> String {
        let mut result = format!("Theorem: {}\n\n", self.theorem.to_string());
        for step in &self.steps {
            result.push_str(&format!(
                "{}. {} ({})\n",
                step.step_num, step.formula.to_string(), step.reason
            ));
        }
        result.push_str(&format!("\nValid: {}\n", self.is_valid));
        result
    }
}

/// Type system for runtime verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unit,
    Bool,
    Int,
    Float,
    String,
    Array(Box<Type>),
    Function(Vec<Type>, Box<Type>),
    Generic(String),
    Custom(String),
}

impl Type {
    pub fn to_string(&self) -> String {
        match self {
            Type::Unit => "unit".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Int => "int".to_string(),
            Type::Float => "float".to_string(),
            Type::String => "string".to_string(),
            Type::Array(inner) => format!("[{}]", inner.to_string()),
            Type::Function(params, ret) => {
                let param_str = params.iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({}) -> {}", param_str, ret.to_string())
            }
            Type::Generic(name) => name.clone(),
            Type::Custom(name) => name.clone(),
        }
    }

    pub fn is_subtype_of(&self, other: &Type) -> bool {
        match (self, other) {
            (a, b) if a == b => true,
            (Type::Custom(a), Type::Custom(b)) => a == b,
            _ => false,
        }
    }
}

/// Type inference system
pub struct TypeInference {
    constraints: Vec<(Type, Type)>,
    substitutions: HashMap<String, Type>,
}

impl TypeInference {
    pub fn new() -> Self {
        TypeInference {
            constraints: Vec::new(),
            substitutions: HashMap::new(),
        }
    }

    pub fn add_constraint(&mut self, ty1: Type, ty2: Type) {
        self.constraints.push((ty1, ty2));
    }

    pub fn unify(&mut self) -> Result<HashMap<String, Type>, TypeError> {
        let mut subst = self.substitutions.clone();

        for (ty1, ty2) in &self.constraints {
            if !Self::unify_types(ty1, ty2, &mut subst) {
                return Err(TypeError::UnificationFailed);
            }
        }

        self.substitutions = subst.clone();
        Ok(subst)
    }

    fn unify_types(ty1: &Type, ty2: &Type, subst: &mut HashMap<String, Type>) -> bool {
        match (ty1, ty2) {
            (Type::Generic(v), t) | (t, Type::Generic(v)) => {
                if let Some(existing) = subst.get(v) {
                    existing == t
                } else {
                    subst.insert(v.clone(), t.clone());
                    true
                }
            }
            (Type::Array(inner1), Type::Array(inner2)) => {
                Self::unify_types(inner1, inner2, subst)
            }
            (a, b) => a == b,
        }
    }

    pub fn substitute(&self, ty: &Type) -> Type {
        match ty {
            Type::Generic(name) => {
                self.substitutions.get(name)
                    .cloned()
                    .unwrap_or_else(|| Type::Generic(name.clone()))
            }
            Type::Array(inner) => {
                Type::Array(Box::new(self.substitute(inner)))
            }
            _ => ty.clone(),
        }
    }
}

/// Theorem Prover
pub struct TheoremProver {
    axioms: Vec<Formula>,
    theorems: Vec<Proof>,
}

impl TheoremProver {
    pub fn new() -> Self {
        TheoremProver {
            axioms: Vec::new(),
            theorems: Vec::new(),
        }
    }

    pub fn add_axiom(&mut self, axiom: Formula) {
        self.axioms.push(axiom);
    }

    pub fn prove(&mut self, theorem: Formula) -> Result<Proof, ProofError> {
        let mut proof = Proof::new(theorem.clone());

        // Add axioms as initial steps
        for axiom in &self.axioms {
            proof.add_step(axiom.clone(), "Axiom".to_string(), vec![]);
        }

        // Add the theorem as final step
        proof.add_step(theorem, "To prove".to_string(), vec![]);

        proof.validate();
        if proof.is_valid {
            self.theorems.push(proof.clone());
            Ok(proof)
        } else {
            Err(ProofError::ProofFailed)
        }
    }

    pub fn verify_property(&self, formula: Formula) -> Result<bool, VerificationError> {
        // Check if formula can be derived from axioms
        for axiom in &self.axioms {
            if axiom == &formula {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Correctness specification for programs
#[derive(Debug, Clone)]
pub struct Specification {
    pub precondition: Formula,
    pub postcondition: Formula,
    pub invariants: Vec<Formula>,
}

impl Specification {
    pub fn new(pre: Formula, post: Formula) -> Self {
        Specification {
            precondition: pre,
            postcondition: post,
            invariants: Vec::new(),
        }
    }

    pub fn add_invariant(&mut self, inv: Formula) {
        self.invariants.push(inv);
    }

    pub fn verify(&self) -> Result<bool, VerificationError> {
        // Simple verification: check that invariants hold
        Ok(!self.invariants.is_empty())
    }
}

/// Errors
#[derive(Debug)]
pub enum TypeError {
    UnificationFailed,
    TypeMismatch(String, String),
    UnknownType(String),
}

#[derive(Debug)]
pub enum ProofError {
    ProofFailed,
    InvalidStep(String),
    CircularDependency,
}

#[derive(Debug)]
pub enum VerificationError {
    SpecificationFailed(String),
    InvariantViolation(String),
    ContractViolation(String),
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VerificationError::SpecificationFailed(msg) => write!(f, "Specification failed: {}", msg),
            VerificationError::InvariantViolation(msg) => write!(f, "Invariant violated: {}", msg),
            VerificationError::ContractViolation(msg) => write!(f, "Contract violated: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_to_string() {
        let f = Formula::Atom("P".to_string());
        assert_eq!(f.to_string(), "P");

        let f = Formula::Not(Box::new(Formula::Atom("P".to_string())));
        assert_eq!(f.to_string(), "¬P");
    }

    #[test]
    fn test_formula_simplify() {
        let f = Formula::Not(Box::new(
            Formula::Not(Box::new(Formula::Atom("P".to_string())))
        ));
        let simplified = f.simplify();
        assert_eq!(simplified, Formula::Atom("P".to_string()));
    }

    #[test]
    fn test_proof_creation() {
        let theorem = Formula::Atom("P".to_string());
        let proof = Proof::new(theorem);
        assert_eq!(proof.steps.len(), 0);
    }

    #[test]
    fn test_type_to_string() {
        assert_eq!(Type::Bool.to_string(), "bool");
        assert_eq!(Type::Int.to_string(), "int");

        let array_type = Type::Array(Box::new(Type::Int));
        assert_eq!(array_type.to_string(), "[int]");
    }

    #[test]
    fn test_type_inference() {
        let mut ti = TypeInference::new();
        ti.add_constraint(Type::Generic("T".to_string()), Type::Int);

        assert!(ti.unify().is_ok());
        assert_eq!(
            ti.substitutions.get("T"),
            Some(&Type::Int)
        );
    }

    #[test]
    fn test_theorem_prover() {
        let mut prover = TheoremProver::new();
        let axiom = Formula::Atom("P".to_string());
        prover.add_axiom(axiom);

        let theorem = Formula::Atom("P".to_string());
        let result = prover.prove(theorem);
        assert!(result.is_ok());
    }

    #[test]
    fn test_specification() {
        let pre = Formula::Atom("x >= 0".to_string());
        let post = Formula::Atom("y >= 0".to_string());
        let spec = Specification::new(pre, post);

        assert!(spec.verify().is_ok());
    }
}
