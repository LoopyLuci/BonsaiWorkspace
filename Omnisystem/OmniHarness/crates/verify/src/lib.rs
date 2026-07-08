//! Axiom verification kernel — Calculus of Constructions with De Bruijn indices.

pub mod kernel;
pub mod proof_token;
pub mod tactics;

pub use kernel::{
    definitionally_equal, lift, normalize, subst, AxiomKernel, Context, Declaration, Environment,
    KernelError, KernelResult, ProofWitness, Sort, Term,
};
pub use proof_token::VerifyToken;
pub use tactics::{Goal, ProofState, TacticEngine, TacticError, TacticResult};
