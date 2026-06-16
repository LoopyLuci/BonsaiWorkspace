# AXIOM Guide - Formal Verification

**AXIOM** is Omnisystem's formal verification language, enabling mathematical proof of program correctness.

## Core Features

### Theorem Proving
- Interactive proofs
- Tactics system
- Type theory
- Dependent types

### Model Checking
- LTL formulas
- CTL formulas
- State exploration
- Property verification

### Program Verification
- Hoare logic
- Symbolic execution
- Path conditions
- Invariant synthesis

## Common Usage

```axiom
theorem add_commutative : ∀ x y, x + y = y + x := by
    intros x y
    induction x with
    | zero => simp
    | succ n ih => simp [ih]

theorem list_append_assoc : ∀ l1 l2 l3,
    (l1 ++ l2) ++ l3 = l1 ++ (l2 ++ l3) := by
    intros
    induction l1 with
    | nil => rfl
    | cons h t ih => simp [ih]
```

## Related Documentation

- [API Reference](../05-reference/AXIOM_API.md)
- [Formal Verification](../10-advanced-topics/FORMAL_VERIFICATION.md)

---

**Status**: Production Ready | **Updated**: 2026-06-16
