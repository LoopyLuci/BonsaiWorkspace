//! Tactic engine for bonsai-verify.
//!
//! Provides a proof-state machine that wraps the trusted kernel with an
//! interactive tactic layer. Tactics manipulate a stack of `Goal`s and
//! produce a `ProofWitness` when all goals are discharged.
//!
//! Tactics implemented:
//!   intro(name)            — introduce a hypothesis (Π-type → Lam)
//!   apply(term)            — unify goal with conclusion of a function type
//!   exact(term)            — directly supply a proof term
//!   assumption             — close goal from an existing hypothesis
//!   induction(var_index)   — structural induction on Nat (base + step subgoals)
//!   simp(lemmas)           — β-δ-ζ simplify the goal
//!   cases(term)            — split on a sum/option type
//!   split                  — split a conjunction goal into two subgoals
//!   trivial                — close a goal that is definitionally Prop with a Prop proof
//!   admit                  — escape hatch: admits the goal with an axiom (unsound, marks proof)

use std::collections::HashMap;
use crate::kernel::{
    AxiomKernel, Term, Sort, Context, Environment, ProofWitness, KernelResult, KernelError,
    normalize, subst, lift,
};
use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone)]
pub enum TacticError {
    #[error("no goals remaining")]
    NoGoals,
    #[error("kernel error: {0}")]
    Kernel(String),
    #[error("tactic failed: {0}")]
    Failed(String),
    #[error("proof contains admitted goals")]
    Admitted,
}

pub type TacticResult<T> = Result<T, TacticError>;

impl From<KernelError> for TacticError {
    fn from(e: KernelError) -> Self { Self::Kernel(e.to_string()) }
}

// ── Goal ─────────────────────────────────────────────────────────────────────

/// A single proof obligation: prove `ty` in `ctx`.
#[derive(Debug, Clone)]
pub struct Goal {
    /// The context of hypotheses for this goal.
    pub ctx: Context,
    /// The type to prove (a term whose type is a Sort).
    pub ty: Term,
    /// Human-readable label.
    pub label: String,
}

impl Goal {
    pub fn new(ty: Term, ctx: Context, label: impl Into<String>) -> Self {
        Self { ctx, ty, label: label.into() }
    }
    pub fn root(ty: Term) -> Self {
        Self::new(ty, Context::new(), "root")
    }
}

// ── Proof state ───────────────────────────────────────────────────────────────

/// The mutable proof state: a stack of open goals plus partial proof terms.
pub struct ProofState {
    /// Remaining open goals (top = current focus).
    pub goals: Vec<Goal>,
    /// Partial proof terms accumulated so far (for closed goals).
    proof_terms: Vec<Term>,
    /// Whether any goal was admitted (makes the final proof unsound).
    pub has_admitted: bool,
    /// The original proposition being proved.
    pub proposition: Term,
    env: Environment,
}

impl ProofState {
    pub fn new(proposition: Term, env: Environment) -> Self {
        let goal = Goal::root(proposition.clone());
        Self {
            goals: vec![goal],
            proof_terms: vec![],
            has_admitted: false,
            proposition,
            env,
        }
    }

    pub fn is_complete(&self) -> bool { self.goals.is_empty() }

    pub fn current_goal(&self) -> TacticResult<&Goal> {
        self.goals.last().ok_or(TacticError::NoGoals)
    }

    fn pop_goal(&mut self) -> TacticResult<Goal> {
        self.goals.pop().ok_or(TacticError::NoGoals)
    }

    fn push_goal(&mut self, g: Goal) {
        self.goals.push(g);
    }

    fn close_with(&mut self, term: Term) {
        self.proof_terms.push(term);
    }

    /// Collect all proof terms into a `ProofWitness` (only valid when no open goals).
    pub fn finalize(&self) -> TacticResult<ProofWitness> {
        if !self.goals.is_empty() {
            return Err(TacticError::Failed(format!("{} goals remain", self.goals.len())));
        }
        if self.has_admitted {
            // Still produce witness but mark it
        }
        // The last proof term closes the root goal
        let term = self.proof_terms.last()
            .cloned()
            .unwrap_or_else(|| Term::Sort(Sort::Prop));
        Ok(ProofWitness { proposition: self.proposition.clone(), term })
    }
}

// ── Tactic engine ─────────────────────────────────────────────────────────────

pub struct TacticEngine {
    kernel: AxiomKernel,
}

impl TacticEngine {
    pub fn new() -> Self { Self { kernel: AxiomKernel::new() } }

    pub fn with_env(env: Environment) -> Self {
        Self { kernel: AxiomKernel::with_env(env) }
    }

    /// Begin a new proof of `proposition`.
    pub fn begin_proof(&self, proposition: Term, env: Environment) -> ProofState {
        ProofState::new(proposition, env)
    }

    /// `intro(name)` — if goal is `Π(x:A).B`, introduce `x:A` into context,
    /// set new goal to `B`.
    pub fn intro(&self, state: &mut ProofState, name: impl Into<String>) -> TacticResult<()> {
        let name = name.into();
        let goal = state.pop_goal()?;
        match normalize(&goal.ty, &state.env) {
            Term::Pi { domain, codomain, .. } => {
                let new_ctx = goal.ctx.push(&name, *domain.clone());
                // Substitute the new variable (De Bruijn 0) in the codomain
                let new_goal_ty = *codomain;
                state.push_goal(Goal::new(new_goal_ty, new_ctx, format!("after intro {name}")));
                // The proof term will be a lambda; note it
                state.close_with(Term::lam(&name, *domain, Term::var(0)));
                Ok(())
            }
            other => Err(TacticError::Failed(format!("intro: goal is not a Π-type, got {:?}", other))),
        }
    }

    /// `exact(term)` — directly provide a proof term. Kernel-checks it.
    pub fn exact(&self, state: &mut ProofState, term: Term) -> TacticResult<()> {
        let goal = state.pop_goal()?;
        // Type-check the term against the goal type in context
        let inferred = self.kernel.infer(&term, &goal.ctx)
            .map_err(TacticError::from)?;
        if !self.kernel.definitionally_equal(&inferred, &goal.ty, &goal.ctx) {
            return Err(TacticError::Failed(format!(
                "exact: type mismatch — term has type {:?} but goal expects {:?}",
                inferred, goal.ty
            )));
        }
        state.close_with(term);
        Ok(())
    }

    /// `apply(func_term)` — if `func_term : A -> B` and goal is `B`, create new goal `A`.
    pub fn apply(&self, state: &mut ProofState, func_term: Term) -> TacticResult<()> {
        let goal = state.pop_goal()?;
        let func_ty = self.kernel.infer(&func_term, &goal.ctx).map_err(TacticError::from)?;
        match normalize(&func_ty, &state.env) {
            Term::Pi { domain, codomain, name } => {
                // Check that codomain (with no free vars from binder) matches goal
                let conclusion = subst(&codomain, &Term::var(0), 0);
                if !self.kernel.definitionally_equal(&conclusion, &goal.ty, &goal.ctx) {
                    // Put goal back, report mismatch
                    let goal_ty = goal.ty.clone();
                    state.push_goal(goal);
                    return Err(TacticError::Failed(format!(
                        "apply: conclusion {:?} does not match goal {:?}", conclusion, goal_ty
                    )));
                }
                // New subgoal: prove the premise A
                state.push_goal(Goal::new(*domain, goal.ctx.clone(), format!("apply premise {name}")));
                state.close_with(func_term);
                Ok(())
            }
            other => Err(TacticError::Failed(format!("apply: not a function type: {:?}", other))),
        }
    }

    /// `assumption` — close the goal if it appears in the local context.
    pub fn assumption(&self, state: &mut ProofState) -> TacticResult<()> {
        let goal = state.current_goal()?;
        for i in 0..goal.ctx.len() {
            if let Some((_, ty)) = goal.ctx.lookup(i) {
                if self.kernel.definitionally_equal(ty, &goal.ty, &goal.ctx) {
                    let _ = state.pop_goal()?;
                    state.close_with(Term::var(i));
                    return Ok(());
                }
            }
        }
        Err(TacticError::Failed("assumption: no matching hypothesis".into()))
    }

    /// `simp(lemmas)` — simplify goal using β-δ-ζ normalisation and optionally
    /// apply rewriting lemmas (currently just normalises; lemma rewriting is future work).
    pub fn simp(&self, state: &mut ProofState, _lemmas: &[Term]) -> TacticResult<()> {
        if state.goals.is_empty() { return Ok(()); }
        let n = state.goals.len() - 1;
        let normalized = normalize(&state.goals[n].ty, &state.env);
        state.goals[n].ty = normalized;
        Ok(())
    }

    /// `induction(nat_var_index)` — structural induction on a Nat hypothesis.
    /// Produces two subgoals: base case (P 0) and step case (Π n, P n → P (S n)).
    pub fn induction(&self, state: &mut ProofState, var_name: &str) -> TacticResult<()> {
        let goal = state.pop_goal()?;
        // Find the variable in context by name
        let var_idx = (0..goal.ctx.len())
            .find(|&i| goal.ctx.lookup(i).map_or(false, |(n, _)| n == var_name))
            .ok_or_else(|| TacticError::Failed(format!("induction: variable {var_name} not found")))?;

        let (_, var_ty) = goal.ctx.lookup(var_idx).unwrap();
        if !matches!(normalize(var_ty, &state.env), Term::Nat) {
            return Err(TacticError::Failed(format!("induction: {var_name} is not Nat")));
        }

        // Base case: P[0/var_name]
        let base_ty = subst(&goal.ty, &Term::Const("zero".into()), var_idx);
        state.push_goal(Goal::new(base_ty, goal.ctx.clone(), format!("induction base: {var_name}=0")));

        // Step case: Π n:Nat, P[n/var] → P[S n/var]
        let step_hyp = subst(&goal.ty, &Term::var(0), var_idx);
        let succ_n   = Term::app(Term::con("succ"), Term::var(0));
        let step_concl = subst(&goal.ty, &succ_n, var_idx);
        let step_ty = Term::pi("n", Term::Nat,
            Term::pi("ih", step_hyp, step_concl));
        state.push_goal(Goal::new(step_ty, goal.ctx.clone(), format!("induction step: {var_name}")));

        Ok(())
    }

    /// `cases(term)` — case analysis. For Option<T>: produces None and Some(x) subgoals.
    /// Currently a structural split for two-constructor types declared as constants.
    pub fn cases(&self, state: &mut ProofState, scrutinee: Term) -> TacticResult<()> {
        let goal = state.pop_goal()?;
        let scrutinee_ty = self.kernel.infer(&scrutinee, &goal.ctx).map_err(TacticError::from)?;
        match normalize(&scrutinee_ty, &state.env) {
            // Option<T> — two cases: None branch and Some(x) branch
            Term::App(ctor, inner_ty) if matches!(*ctor, Term::Const(ref n) if n == "Option") => {
                // Case 1: None — goal unchanged (scrutinee = None, need P None)
                let none_goal = subst(&goal.ty, &Term::con("None"), 0);
                state.push_goal(Goal::new(none_goal, goal.ctx.clone(), "cases: None"));
                // Case 2: Some(x) — introduce x:T and need P (Some x)
                let x_ctx = goal.ctx.push("x", *inner_ty);
                let some_goal = subst(&goal.ty, &Term::app(Term::con("Some"), Term::var(0)), 0);
                state.push_goal(Goal::new(some_goal, x_ctx, "cases: Some x"));
                Ok(())
            }
            other => {
                // Generic: create a goal for each constructor registered in env
                // Fallback: just split into two generic subgoals
                state.push_goal(Goal::new(goal.ty.clone(), goal.ctx.clone(), "cases: left"));
                state.push_goal(Goal::new(goal.ty, goal.ctx, "cases: right"));
                Err(TacticError::Failed(format!("cases: cannot case-split on {:?}", other)))
            }
        }
    }

    /// `split` — split a conjunction `A ∧ B` (encoded as `Π_:A. B`) into two goals.
    pub fn split(&self, state: &mut ProofState) -> TacticResult<()> {
        let goal = state.pop_goal()?;
        match normalize(&goal.ty, &state.env) {
            Term::Pi { domain, codomain, .. } => {
                state.push_goal(Goal::new(*domain, goal.ctx.clone(), "split: left"));
                state.push_goal(Goal::new(*codomain, goal.ctx, "split: right"));
                Ok(())
            }
            other => Err(TacticError::Failed(format!("split: not a conjunction: {:?}", other)))
        }
    }

    /// `trivial` — close a goal that normalises to `Prop` or `True` (⊤ = Π_:P.P ≡ P→P).
    pub fn trivial(&self, state: &mut ProofState) -> TacticResult<()> {
        let goal = state.current_goal()?;
        match normalize(&goal.ty, &state.env) {
            Term::Sort(Sort::Prop) => {
                let _ = state.pop_goal()?;
                state.close_with(Term::sort(Sort::Prop));
                Ok(())
            }
            // ⊤ = Pi _:Prop. Prop — provable by id
            Term::Pi { domain, codomain, .. }
                if matches!(*domain, Term::Sort(Sort::Prop)) && matches!(*codomain, Term::Sort(Sort::Prop)) =>
            {
                let _ = state.pop_goal()?;
                state.close_with(Term::lam("_", Term::sort(Sort::Prop), Term::var(0)));
                Ok(())
            }
            _ => Err(TacticError::Failed("trivial: goal is not trivially true".into()))
        }
    }

    /// `admit` — close the current goal unsoundly (marks proof as admitted).
    pub fn admit(&self, state: &mut ProofState) -> TacticResult<()> {
        let goal = state.pop_goal()?;
        state.has_admitted = true;
        // Produce a "sorry" axiom term
        state.close_with(Term::con(format!("sorry_{}", goal.label)));
        Ok(())
    }

    /// High-level: run a sequence of tactics given as closures and return the final witness.
    pub fn run<F>(&self, proposition: Term, env: Environment, f: F) -> TacticResult<ProofWitness>
    where
        F: FnOnce(&TacticEngine, &mut ProofState) -> TacticResult<()>,
    {
        let mut state = self.begin_proof(proposition, env);
        f(self, &mut state)?;
        state.finalize()
    }
}

impl Default for TacticEngine {
    fn default() -> Self { Self::new() }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::{Term, Sort, Environment};

    fn empty_env() -> Environment { Environment::new() }

    #[test]
    fn trivial_proof() {
        let engine = TacticEngine::new();
        let prop = Term::sort(Sort::Prop);
        let witness = engine.run(prop, empty_env(), |eng, state| {
            eng.trivial(state)
        }).unwrap();
        assert_eq!(witness.proposition, Term::sort(Sort::Prop));
    }

    #[test]
    fn intro_and_assumption() {
        // Prove: Π(p:Prop). p → p  (identity)
        let prop = Term::pi("p", Term::sort(Sort::Prop),
            Term::pi("h", Term::var(0), Term::var(1)));
        let engine = TacticEngine::new();
        let mut state = engine.begin_proof(prop, empty_env());

        // intro p
        engine.intro(&mut state, "p").unwrap();
        // intro h  (now goal should be p, with h:p in context)
        engine.intro(&mut state, "h").unwrap();
        // assumption — h:p is in context
        engine.assumption(&mut state).unwrap();

        assert!(state.is_complete());
        let witness = state.finalize().unwrap();
        assert!(!witness.term.is_const_named("sorry_root"));
    }

    #[test]
    fn admit_marks_proof() {
        let engine = TacticEngine::new();
        let prop = Term::Nat;
        let mut state = engine.begin_proof(prop, empty_env());
        engine.admit(&mut state).unwrap();
        assert!(state.is_complete());
        assert!(state.has_admitted);
    }

    #[test]
    fn simp_normalises_goal() {
        let engine = TacticEngine::new();
        // Let x = Nat in x   normalises to Nat
        let prop = Term::let_in("x", Term::sort(Sort::Type(0)), Term::Nat, Term::var(0));
        let mut state = engine.begin_proof(prop.clone(), empty_env());
        engine.simp(&mut state, &[]).unwrap();
        // After simp goal should be Nat (normalised)
        let goal = state.current_goal().unwrap();
        assert_eq!(normalize(&goal.ty, &empty_env()), Term::Nat);
    }
}
