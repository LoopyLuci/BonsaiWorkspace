//! Axiom verification kernel — Calculus of Constructions with De Bruijn indices.

pub mod kernel;

pub use kernel::{
    AxiomKernel, Term, Sort, Context, Environment, Declaration,
    ProofWitness, KernelError, KernelResult,
    lift, subst, normalize, definitionally_equal,
};
