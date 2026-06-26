# AXIOM Compiler Architecture v1.0
## Formal Verification Compilation System

---

## 1. PIPELINE

```
AXIOM Source Code
    ↓
[Lexer/Parser] → Proof Term AST
    ↓
[Type Checker] → Dependent Type Checking
    ↓
[Proof Verifier] → Check Proof Correctness
    ↓
[SMT Encoding] → SMT-LIB2 Formula
    ↓
[SMT Solver] → Discharge Verification Goals
    ↓
[Erase Proofs] → Runtime-Executable Code
    ↓
[Code Generator] → LLVM IR with Assertions
    ↓
[Runtime Integration] → Executable with Verification
```

---

## 2. DEPENDENT TYPE CHECKING

### 2.1 Type Unification with Values

```
fn unify_dependent_types(
    type1: DependentType,
    type2: DependentType,
    value_env: Environment
) -> Result<Substitution, Error> {
    // Handle dependent type unification
    // type1 = Vec(n), type2 = Vec(m)
    // Must prove n == m
    
    if type1 is DependentType {
        base1 = type1.base
        indices1 = type1.indices
        
        if type2 is DependentType {
            base2 = type2.base
            indices2 = type2.indices
            
            if base1 != base2 {
                return Error("Base types don't match")
            }
            
            // Unify index expressions
            for (idx1, idx2) in zip(indices1, indices2) {
                if !semantically_equal(idx1, idx2, value_env) {
                    return Error("Index mismatch: {} vs {}", idx1, idx2)
                }
            }
        }
    }
    
    return Ok(Substitution::empty())
}
```

---

## 3. PROOF VERIFICATION

### 3.1 Proof Term Checking

```
fn verify_proof(
    proof_term: ProofTerm,
    goal: Proposition,
    context: ProofContext
) -> Result<(), Error> {
    // Type-check the proof term
    proof_type = infer_proof_type(proof_term, context)
    
    // Check proof type matches goal
    if !unify(proof_type, goal, context) {
        return Error("Proof type doesn't match goal: {} vs {}", proof_type, goal)
    }
    
    // Recursively verify sub-proofs
    for sub_proof in proof_term.sub_proofs() {
        sub_goal = extract_sub_goal(proof_term, sub_proof)
        verify_proof(sub_proof, sub_goal, context)?
    }
    
    return Ok(())
}

fn verify_inductive_proof(
    proof: InductiveProof,
    prop: Proposition
) -> Result<(), Error> {
    // Verify base case
    base_prop = instantiate_at_zero(prop)
    verify_proof(proof.base_case, base_prop, empty_context())?
    
    // Verify inductive step
    let n = fresh_variable("n")
    let ih = add_assumption(empty_context(), prop.with_n(n))
    
    step_prop = instantiate_at_succ(prop, n)
    verify_proof(proof.step_case, step_prop, ih)?
    
    return Ok(())
}
```

---

## 4. SMT ENCODING

### 4.1 Translate to SMT-LIB2

```
fn encode_goal_to_smt(goal: Proposition, context: Environment) -> string {
    formula = "(set-logic QF_LIA)\n"  // Quantifier-free linear arithmetic
    
    // Declare constants
    for binding in context {
        formula += "(declare-const {} {})\n".format(
            binding.name,
            encode_type(binding.type)
        )
    }
    
    // Assert assumptions
    for assumption in context.assumptions {
        formula += "(assert {})\n".format(encode_formula(assumption))
    }
    
    // Assert goal (negated for unsatisfiability check)
    formula += "(assert (not {}))\n".format(encode_formula(goal))
    
    formula += "(check-sat)\n"
    formula += "(get-model)\n"
    
    return formula
}

fn encode_formula(prop: Proposition) -> string {
    match prop {
        Equality(a, b) => return "(= {} {})".format(encode_expr(a), encode_expr(b)),
        BinaryOp("+", a, b) => return "(+ {} {})".format(encode_expr(a), encode_expr(b)),
        BinaryOp("<", a, b) => return "(< {} {})".format(encode_expr(a), encode_expr(b)),
        And(p, q) => return "(and {} {})".format(encode_formula(p), encode_formula(q)),
        Or(p, q) => return "(or {} {})".format(encode_formula(p), encode_formula(q)),
        Implies(p, q) => return "(=> {} {})".format(encode_formula(p), encode_formula(q)),
        Not(p) => return "(not {})".format(encode_formula(p)),
        Forall(vars, body) => {
            quantified = "(forall (({}))\n {})".format(
                vars.map(encode_var_decl).join(" "),
                encode_formula(body)
            )
            return quantified
        }
    }
}
```

---

## 5. PROOF ERASURE

### 5.1 Remove Proof Terms

```
fn erase_proofs(program: TypedProgram) -> RuntimeProgram {
    erased = RuntimeProgram {}
    
    for fn_decl in program.functions {
        // Remove proof parameters
        params = fn_decl.parameters
            .filter(|p| !p.type.is_proof_type())
        
        // Erase proof terms from body
        erased_body = erase_proofs_from_block(fn_decl.body)
        
        // Keep runtime assertions for unproven properties
        assertions = extract_unproven_assertions(fn_decl)
        erased_body.prepend_assertions(assertions)
        
        erased.add_function(FunctionDecl {
            name: fn_decl.name,
            parameters: params,
            return_type: fn_decl.return_type,
            body: erased_body
        })
    }
    
    return erased
}

fn extract_unproven_assertions(fn_decl: FunctionDecl) -> [Assertion] {
    assertions = []
    
    for precond in fn_decl.preconditions {
        if !proven_in_context(precond, fn_decl.context) {
            // Generate runtime assertion
            assertions.push(Assertion {
                condition: precond,
                error_message: "Precondition violation: {}".format(precond)
            })
        }
    }
    
    for postcond in fn_decl.postconditions {
        if !always_holds(postcond, fn_decl.body) {
            assertions.push(Assertion {
                condition: postcond,
                error_message: "Postcondition violation: {}".format(postcond)
            })
        }
    }
    
    return assertions
}
```

---

## 6. CODE GENERATION

### 6.1 Generate Executable Code

```
fn generate_code(program: RuntimeProgram) -> LLVMModule {
    module = LLVMModule::new()
    
    // Generate functions with assertions
    for fn_decl in program.functions {
        fn = module.create_function(fn_decl.name)
        builder = IRBuilder(fn)
        
        // Generate assertion checks for preconditions
        for precond in fn_decl.preconditions {
            condition = generate_expr(precond, builder)
            
            fail_block = fn.append_block("precond_failed")
            pass_block = fn.append_block("precond_passed")
            
            builder.create_cond_branch(condition, pass_block, fail_block)
            
            builder.set_insertion_point(fail_block)
            builder.create_call("abort", [])
            
            builder.set_insertion_point(pass_block)
        }
        
        // Generate function body
        generate_block(fn_decl.body, builder, module)
        
        // Generate assertion checks for postconditions
        for postcond in fn_decl.postconditions {
            condition = generate_expr(postcond, builder)
            
            fail_block = fn.append_block("postcond_failed")
            pass_block = fn.append_block("postcond_passed")
            
            builder.create_cond_branch(condition, pass_block, fail_block)
            
            builder.set_insertion_point(fail_block)
            builder.create_call("abort", [])
            
            builder.set_insertion_point(pass_block)
        }
    }
    
    return module
}
```

---

## 7. EXAMPLE COMPILATION

```
AXIOM Function:
────────────────
fn divide(a: i32, b: {x: i32 | x != 0}) -> i32 {
    return a / b
}

Step 1: Parse & Type Check ✓
Step 2: Extract Proof Goals
  - Goal: b != 0

Step 3: Encode to SMT-LIB2:
  (declare-const a Int)
  (declare-const b Int)
  (assert (not (= b 0)))
  (assert (not (not (= b 0))))  ; Negated goal
  (check-sat)

Step 4: SMT Solver (Z3) → unsat ✓
  Proof verified!

Step 5: Erase Proofs
  fn divide(a: i32, b: i32) -> i32 {
    assert(b != 0, "Refinement violated: b must be != 0")
    return a / b
  }

Step 6: Generate LLVM IR:
  define i32 @divide(i32 %a, i32 %b) {
    %cond = icmp ne i32 %b, 0
    br i1 %cond, label %pass, label %fail
    
    fail:
      call void @abort()
      unreachable
    
    pass:
      %result = sdiv i32 %a, %b
      ret i32 %result
  }

Result: Executable with runtime verification
```

---

This architecture enables AXIOM to provide formal proof while generating practical runtime-verified executables.
