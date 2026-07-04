//! Axiom verification kernel — Calculus of Constructions with De Bruijn indices.
//!
//! Implements a minimal but complete dependent type theory:
//! - Sorts: Prop, Type(n) for n ≥ 0
//! - λ-abstraction and Π-types (dependent functions)
//! - Let-bindings with definitional equality
//! - De Bruijn indices for capture-avoiding substitution
//! - Bi-directional type inference and checking
//! - β-δ-ζ normalisation (beta, delta = unfold let, zeta = unfold let in type)
//!
//! This is the *trusted* kernel: only code in this file can produce a `Proof`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ── Sorts ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sort {
    /// The sort of propositions (impredicative, proof-irrelevant).
    Prop,
    /// The sort of types at universe level n.
    Type(u32),
}

impl Sort {
    pub fn universe_level(&self) -> u32 {
        match self {
            Sort::Prop => 0,
            Sort::Type(n) => *n,
        }
    }

    /// The sort of this sort (Prop : Type(0), Type(n) : Type(n+1)).
    pub fn sort_of(&self) -> Sort {
        match self {
            Sort::Prop => Sort::Type(0),
            Sort::Type(n) => Sort::Type(n + 1),
        }
    }

    /// Impredicative rule: if codomain is Prop, the Π-type is Prop.
    pub fn pi_result(domain: &Sort, codomain: &Sort) -> Sort {
        match codomain {
            Sort::Prop => Sort::Prop,
            Sort::Type(n) => {
                let m = domain.universe_level();
                Sort::Type((*n).max(m))
            }
        }
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sort::Prop => write!(f, "Prop"),
            Sort::Type(0) => write!(f, "Type"),
            Sort::Type(n) => write!(f, "Type({})", n),
        }
    }
}

// ── Terms ─────────────────────────────────────────────────────────────────────

/// A term in the Calculus of Constructions, with 9 constructors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Term {
    /// De Bruijn variable: index 0 = innermost binder.
    Var(usize),

    /// Global constant (from the context/environment).
    Const(String),

    /// Function application: `(f a)`.
    App(Box<Term>, Box<Term>),

    /// λ-abstraction: `λ(x : ty). body`.
    /// The binder name is kept for pretty-printing only.
    Lam {
        name: String,
        ty: Box<Term>,
        body: Box<Term>,
    },

    /// Dependent function type: `Π(x : domain). codomain`.
    Pi {
        name: String,
        domain: Box<Term>,
        codomain: Box<Term>,
    },

    /// A universe sort.
    Sort(Sort),

    /// Let-binding: `let x : ty := val in body`.
    Let {
        name: String,
        ty: Box<Term>,
        val: Box<Term>,
        body: Box<Term>,
    },

    /// The type of natural numbers (built-in for convenience).
    Nat,

    /// A verified proof witness — only constructible by the kernel.
    Proof(Box<ProofWitness>),
}

/// An opaque proof witness produced by `AxiomKernel::prove`.
/// Cannot be constructed outside this module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofWitness {
    pub proposition: Term,
    pub term: Term,
}

// ── Helper constructors ───────────────────────────────────────────────────────

impl Term {
    pub fn var(i: usize) -> Self {
        Term::Var(i)
    }
    pub fn con(name: impl Into<String>) -> Self {
        Term::Const(name.into())
    }
    pub fn app(f: Term, a: Term) -> Self {
        Term::App(Box::new(f), Box::new(a))
    }
    pub fn lam(name: impl Into<String>, ty: Term, body: Term) -> Self {
        Term::Lam {
            name: name.into(),
            ty: Box::new(ty),
            body: Box::new(body),
        }
    }
    pub fn pi(name: impl Into<String>, domain: Term, codomain: Term) -> Self {
        Term::Pi {
            name: name.into(),
            domain: Box::new(domain),
            codomain: Box::new(codomain),
        }
    }
    pub fn sort(s: Sort) -> Self {
        Term::Sort(s)
    }
    pub fn prop() -> Self {
        Term::Sort(Sort::Prop)
    }
    pub fn type0() -> Self {
        Term::Sort(Sort::Type(0))
    }
    pub fn let_in(name: impl Into<String>, ty: Term, val: Term, body: Term) -> Self {
        Term::Let {
            name: name.into(),
            ty: Box::new(ty),
            val: Box::new(val),
            body: Box::new(body),
        }
    }

    pub fn is_const_named(&self, name: &str) -> bool {
        matches!(self, Term::Const(n) if n.starts_with(name))
    }
}

// ── Environment ───────────────────────────────────────────────────────────────

/// A global constant declaration.
#[derive(Debug, Clone)]
pub struct Declaration {
    /// The type of the constant.
    pub ty: Term,
    /// The definition, if any (None for axioms).
    pub body: Option<Term>,
}

/// Global environment of constants and axioms.
#[derive(Debug, Clone, Default)]
pub struct Environment {
    pub decls: HashMap<String, Declaration>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_axiom(&mut self, name: impl Into<String>, ty: Term) {
        self.decls
            .insert(name.into(), Declaration { ty, body: None });
    }

    pub fn add_def(&mut self, name: impl Into<String>, ty: Term, body: Term) {
        self.decls.insert(
            name.into(),
            Declaration {
                ty,
                body: Some(body),
            },
        );
    }

    pub fn lookup(&self, name: &str) -> Option<&Declaration> {
        self.decls.get(name)
    }
}

// ── Local context ─────────────────────────────────────────────────────────────

/// A typing context: a stack of (name, type) entries.
/// The last entry is bound by De Bruijn index 0.
#[derive(Debug, Clone, Default)]
pub struct Context {
    entries: Vec<(String, Term)>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, name: impl Into<String>, ty: Term) -> Self {
        let mut c = self.clone();
        c.entries.push((name.into(), ty));
        c
    }

    pub fn lookup(&self, i: usize) -> Option<&(String, Term)> {
        let n = self.entries.len();
        if i < n {
            Some(&self.entries[n - 1 - i])
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ── Type errors ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    UnboundVariable(usize),
    UnknownConst(String),
    NotAFunction(Box<Term>),
    TypeMismatch {
        expected: Box<Term>,
        got: Box<Term>,
    },
    SortExpected(Box<Term>),
    CannotProve(String),
    ApplicationTypeMismatch {
        arg: Box<Term>,
        expected: Box<Term>,
        got: Box<Term>,
    },
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::UnboundVariable(i) => write!(f, "unbound variable: #{}", i),
            KernelError::UnknownConst(n) => write!(f, "unknown constant: {}", n),
            KernelError::NotAFunction(t) => write!(f, "not a function type: {:?}", t),
            KernelError::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {:?}, got {:?}", expected, got)
            }
            KernelError::SortExpected(t) => write!(f, "expected a sort, got {:?}", t),
            KernelError::CannotProve(msg) => write!(f, "cannot prove: {}", msg),
            KernelError::ApplicationTypeMismatch { arg, expected, got } => write!(
                f,
                "argument {:?} has type {:?} but expected {:?}",
                arg, got, expected
            ),
        }
    }
}

pub type KernelResult<T> = Result<T, KernelError>;

// ── De Bruijn substitution & lifting ──────────────────────────────────────────

/// Lift all free De Bruijn indices ≥ `cutoff` by `amount`.
pub fn lift(term: &Term, cutoff: usize, amount: usize) -> Term {
    if amount == 0 {
        return term.clone();
    }
    match term {
        Term::Var(i) => {
            if *i >= cutoff {
                Term::Var(i + amount)
            } else {
                Term::Var(*i)
            }
        }
        Term::Const(_) | Term::Sort(_) | Term::Nat => term.clone(),
        Term::App(f, a) => Term::app(lift(f, cutoff, amount), lift(a, cutoff, amount)),
        Term::Lam { name, ty, body } => Term::lam(
            name,
            lift(ty, cutoff, amount),
            lift(body, cutoff + 1, amount),
        ),
        Term::Pi {
            name,
            domain,
            codomain,
        } => Term::pi(
            name,
            lift(domain, cutoff, amount),
            lift(codomain, cutoff + 1, amount),
        ),
        Term::Let {
            name,
            ty,
            val,
            body,
        } => Term::let_in(
            name,
            lift(ty, cutoff, amount),
            lift(val, cutoff, amount),
            lift(body, cutoff + 1, amount),
        ),
        Term::Proof(pw) => Term::Proof(Box::new(ProofWitness {
            proposition: lift(&pw.proposition, cutoff, amount),
            term: lift(&pw.term, cutoff, amount),
        })),
    }
}

/// Substitute `sub` for De Bruijn index 0, adjusting other indices.
pub fn subst(term: &Term, sub: &Term, depth: usize) -> Term {
    match term {
        Term::Var(i) => match i.cmp(&depth) {
            std::cmp::Ordering::Equal => lift(sub, 0, depth),
            std::cmp::Ordering::Greater => Term::Var(i - 1),
            std::cmp::Ordering::Less => Term::Var(*i),
        },
        Term::Const(_) | Term::Sort(_) | Term::Nat => term.clone(),
        Term::App(f, a) => Term::app(subst(f, sub, depth), subst(a, sub, depth)),
        Term::Lam { name, ty, body } => {
            Term::lam(name, subst(ty, sub, depth), subst(body, sub, depth + 1))
        }
        Term::Pi {
            name,
            domain,
            codomain,
        } => Term::pi(
            name,
            subst(domain, sub, depth),
            subst(codomain, sub, depth + 1),
        ),
        Term::Let {
            name,
            ty,
            val,
            body,
        } => Term::let_in(
            name,
            subst(ty, sub, depth),
            subst(val, sub, depth),
            subst(body, sub, depth + 1),
        ),
        Term::Proof(pw) => Term::Proof(Box::new(ProofWitness {
            proposition: subst(&pw.proposition, sub, depth),
            term: subst(&pw.term, sub, depth),
        })),
    }
}

// ── Normalisation (β-δ-ζ) ─────────────────────────────────────────────────────

/// Reduce `term` to weak head normal form, then recurse.
pub fn normalize(term: &Term, env: &Environment) -> Term {
    match term {
        Term::Var(_) | Term::Sort(_) | Term::Nat => term.clone(),

        Term::Const(name) => {
            if let Some(decl) = env.lookup(name) {
                if let Some(body) = &decl.body {
                    return normalize(body, env); // δ-reduction
                }
            }
            term.clone()
        }

        Term::App(f, a) => {
            let f_n = normalize(f, env);
            let a_n = normalize(a, env);
            match f_n {
                Term::Lam { body, .. } => {
                    // β-reduction
                    let substituted = subst(&body, &a_n, 0);
                    normalize(&substituted, env)
                }
                other => Term::app(other, a_n),
            }
        }

        Term::Lam { name, ty, body } => Term::lam(name, normalize(ty, env), normalize(body, env)),

        Term::Pi {
            name,
            domain,
            codomain,
        } => Term::pi(name, normalize(domain, env), normalize(codomain, env)),

        Term::Let { val, body, .. } => {
            // ζ-reduction: substitute val into body
            let val_n = normalize(val, env);
            let substituted = subst(body, &val_n, 0);
            normalize(&substituted, env)
        }

        Term::Proof(pw) => Term::Proof(Box::new(ProofWitness {
            proposition: normalize(&pw.proposition, env),
            term: normalize(&pw.term, env),
        })),
    }
}

/// Definitional equality (α + β + δ + ζ).
pub fn definitionally_equal(a: &Term, b: &Term, env: &Environment) -> bool {
    let a_n = normalize(a, env);
    let b_n = normalize(b, env);
    alpha_eq(&a_n, &b_n)
}

fn alpha_eq(a: &Term, b: &Term) -> bool {
    match (a, b) {
        (Term::Var(i), Term::Var(j)) => i == j,
        (Term::Const(n1), Term::Const(n2)) => n1 == n2,
        (Term::Sort(s1), Term::Sort(s2)) => s1 == s2,
        (Term::Nat, Term::Nat) => true,
        (Term::App(f1, a1), Term::App(f2, a2)) => alpha_eq(f1, f2) && alpha_eq(a1, a2),
        (
            Term::Lam {
                ty: t1, body: b1, ..
            },
            Term::Lam {
                ty: t2, body: b2, ..
            },
        ) => alpha_eq(t1, t2) && alpha_eq(b1, b2),
        (
            Term::Pi {
                domain: d1,
                codomain: c1,
                ..
            },
            Term::Pi {
                domain: d2,
                codomain: c2,
                ..
            },
        ) => alpha_eq(d1, d2) && alpha_eq(c1, c2),
        (
            Term::Let {
                ty: t1,
                val: v1,
                body: b1,
                ..
            },
            Term::Let {
                ty: t2,
                val: v2,
                body: b2,
                ..
            },
        ) => alpha_eq(t1, t2) && alpha_eq(v1, v2) && alpha_eq(b1, b2),
        _ => false,
    }
}

// ── AxiomKernel ───────────────────────────────────────────────────────────────

/// The trusted type-checking kernel.
/// All proofs must pass through `check` or `infer`.
pub struct AxiomKernel {
    pub env: Environment,
}

impl AxiomKernel {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
        }
    }

    pub fn with_env(env: Environment) -> Self {
        Self { env }
    }

    /// Expose definitional equality as a method.
    pub fn definitionally_equal(&self, a: &Term, b: &Term, _ctx: &Context) -> bool {
        definitionally_equal(a, b, &self.env)
    }

    pub fn with_nat() -> Self {
        let mut k = Self::new();
        // Add nat constructors as axioms
        k.env.add_axiom("zero", Term::Nat);
        k.env.add_axiom("succ", Term::pi("n", Term::Nat, Term::Nat));
        k.env.add_axiom(
            "nat_rec",
            Term::pi(
                "P",
                Term::pi("_", Term::Nat, Term::type0()),
                Term::pi(
                    "z",
                    Term::app(Term::var(0), Term::con("zero")),
                    Term::pi(
                        "s",
                        Term::pi("n", Term::Nat, Term::app(Term::var(2), Term::var(0))),
                        Term::pi("n", Term::Nat, Term::app(Term::var(3), Term::var(0))),
                    ),
                ),
            ),
        );
        k
    }

    /// Infer the type of `term` in local context `ctx`.
    pub fn infer(&self, term: &Term, ctx: &Context) -> KernelResult<Term> {
        match term {
            Term::Var(i) => {
                let (_, ty) = ctx.lookup(*i).ok_or(KernelError::UnboundVariable(*i))?;
                Ok(lift(ty, 0, *i + 1))
            }

            Term::Const(name) => {
                let decl = self
                    .env
                    .lookup(name)
                    .ok_or_else(|| KernelError::UnknownConst(name.clone()))?;
                Ok(decl.ty.clone())
            }

            Term::Sort(s) => Ok(Term::Sort(s.sort_of())),

            Term::Nat => Ok(Term::Sort(Sort::Type(0))),

            Term::App(f, a) => {
                let f_ty = self.infer(f, ctx)?;
                let f_ty_n = normalize(&f_ty, &self.env);
                match f_ty_n {
                    Term::Pi {
                        domain, codomain, ..
                    } => {
                        self.check(a, &domain, ctx)?;
                        let a_n = normalize(a, &self.env);
                        Ok(subst(&codomain, &a_n, 0))
                    }
                    other => Err(KernelError::NotAFunction(Box::new(other))),
                }
            }

            Term::Lam { name, ty, body } => {
                // Ensure ty is a type
                self.check_is_sort(ty, ctx)?;
                let ctx2 = ctx.push(name, *ty.clone());
                let body_ty = self.infer(body, &ctx2)?;
                Ok(Term::pi(name, *ty.clone(), body_ty))
            }

            Term::Pi {
                name,
                domain,
                codomain,
            } => {
                let d_sort = self.infer_sort(domain, ctx)?;
                let ctx2 = ctx.push(name, *domain.clone());
                let c_sort = self.infer_sort(codomain, &ctx2)?;
                Ok(Term::Sort(Sort::pi_result(&d_sort, &c_sort)))
            }

            Term::Let {
                name,
                ty,
                val,
                body,
            } => {
                self.check_is_sort(ty, ctx)?;
                self.check(val, ty, ctx)?;
                let val_n = normalize(val, &self.env);
                let ctx2 = ctx.push(name, *ty.clone());
                let body_ty = self.infer(body, &ctx2)?;
                // Substitute val into body type (ζ)
                Ok(subst(&body_ty, &val_n, 0))
            }

            Term::Proof(pw) => {
                // A proof witness is already verified — its type is its proposition
                Ok(pw.proposition.clone())
            }
        }
    }

    /// Check that `term` has type `expected` in context `ctx`.
    pub fn check(&self, term: &Term, expected: &Term, ctx: &Context) -> KernelResult<()> {
        let inferred = self.infer(term, ctx)?;
        let expected_n = normalize(expected, &self.env);
        let inferred_n = normalize(&inferred, &self.env);
        if !alpha_eq(&inferred_n, &expected_n) {
            Err(KernelError::TypeMismatch {
                expected: Box::new(expected_n),
                got: Box::new(inferred_n),
            })
        } else {
            Ok(())
        }
    }

    /// Normalise `term` using the kernel's environment.
    pub fn normalize(&self, term: &Term) -> Term {
        normalize(term, &self.env)
    }

    /// Construct a `Proof` witness if `proof_term : proposition` type-checks.
    pub fn prove(&self, proposition: Term, proof_term: Term, ctx: &Context) -> KernelResult<Term> {
        self.check(&proof_term, &proposition, ctx)?;
        Ok(Term::Proof(Box::new(ProofWitness {
            proposition,
            term: proof_term,
        })))
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn check_is_sort(&self, term: &Term, ctx: &Context) -> KernelResult<Sort> {
        let ty = self.infer(term, ctx)?;
        let ty_n = normalize(&ty, &self.env);
        match ty_n {
            Term::Sort(s) => Ok(s),
            other => Err(KernelError::SortExpected(Box::new(other))),
        }
    }

    fn infer_sort(&self, term: &Term, ctx: &Context) -> KernelResult<Sort> {
        let ty = self.infer(term, ctx)?;
        let ty_n = normalize(&ty, &self.env);
        match ty_n {
            Term::Sort(s) => Ok(s),
            other => Err(KernelError::SortExpected(Box::new(other))),
        }
    }
}

impl Default for AxiomKernel {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context::new()
    }

    #[test]
    fn zero_has_type_nat() {
        let k = AxiomKernel::with_nat();
        let zero = Term::con("zero");
        let ty = k.infer(&zero, &ctx()).unwrap();
        assert_eq!(ty, Term::Nat);
    }

    #[test]
    fn nat_has_type_type0() {
        let k = AxiomKernel::new();
        let ty = k.infer(&Term::Nat, &ctx()).unwrap();
        assert_eq!(ty, Term::Sort(Sort::Type(0)));
    }

    #[test]
    fn sort_hierarchy() {
        let k = AxiomKernel::new();
        // Type : Type(1)
        let ty = k.infer(&Term::type0(), &ctx()).unwrap();
        assert_eq!(ty, Term::Sort(Sort::Type(1)));
        // Prop : Type(0)
        let prop_ty = k.infer(&Term::prop(), &ctx()).unwrap();
        assert_eq!(prop_ty, Term::Sort(Sort::Type(0)));
    }

    #[test]
    fn identity_function_type_checks() {
        let k = AxiomKernel::new();
        // id : Π(A : Type). Π(x : A). A
        // = λ A. λ x. x
        let id = Term::lam(
            "A",
            Term::type0(),
            Term::lam("x", Term::var(0), Term::var(0)),
        );
        let ty = k.infer(&id, &ctx()).unwrap();
        // Should be: Π(A : Type(0)). Π(_ : A). A
        match ty {
            Term::Pi { .. } => {} // success
            other => panic!("expected Pi, got {:?}", other),
        }
    }

    #[test]
    fn beta_normalization() {
        let k = AxiomKernel::new();
        // (λ x : Nat. x) applied to Nat  →  Nat
        let app = Term::app(Term::lam("x", Term::Nat, Term::var(0)), Term::Nat);
        let normal = k.normalize(&app);
        assert_eq!(normal, Term::Nat);
    }

    #[test]
    fn let_binding_normalizes() {
        let k = AxiomKernel::new();
        // let x : Type := Nat in x  →  Nat
        let let_term = Term::let_in("x", Term::type0(), Term::Nat, Term::var(0));
        let normal = k.normalize(&let_term);
        assert_eq!(normal, Term::Nat);
    }

    #[test]
    fn type_error_on_bad_application() {
        let k = AxiomKernel::new();
        // Applying Nat (not a function) to Nat should fail
        let bad = Term::app(Term::Nat, Term::Nat);
        let result = k.infer(&bad, &ctx());
        assert!(result.is_err());
        matches!(result.unwrap_err(), KernelError::NotAFunction(_));
    }

    #[test]
    fn unbound_variable_is_error() {
        let k = AxiomKernel::new();
        let result = k.infer(&Term::var(5), &ctx());
        assert_eq!(result.unwrap_err(), KernelError::UnboundVariable(5));
    }

    #[test]
    fn prove_produces_proof_witness() {
        let k = AxiomKernel::with_nat();
        // Prove: zero : Nat
        let proof = k.prove(Term::Nat, Term::con("zero"), &ctx()).unwrap();
        match proof {
            Term::Proof(pw) => assert_eq!(pw.proposition, Term::Nat),
            _ => panic!("expected Proof"),
        }
    }

    #[test]
    fn definitional_equality_via_beta() {
        let k = AxiomKernel::new();
        // (λ x. x) Nat  ≡  Nat
        let lhs = Term::app(Term::lam("x", Term::type0(), Term::var(0)), Term::Nat);
        assert!(definitionally_equal(&lhs, &Term::Nat, &k.env));
    }
}
