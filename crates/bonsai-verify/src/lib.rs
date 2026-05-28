//! Axiom verification kernel — Calculus of Constructions with De Bruijn indices.

pub mod kernel;
pub mod proof_token;
pub mod tactics;

pub use kernel::{
    AxiomKernel, Term, Sort, Context, Environment, Declaration,
    ProofWitness, KernelError, KernelResult,
    lift, subst, normalize, definitionally_equal,
};
pub use proof_token::VerifyToken;
pub use tactics::{TacticEngine, TacticError, TacticResult, Goal, ProofState};
