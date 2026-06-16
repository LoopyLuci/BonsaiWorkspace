# Omni-IR: Unified Intermediate Representation

Omni-IR is the common target for all Omni-languages (Titan, Sylva, Aether, Axiom).

## Structure

```
Module {
  name: String,
  exports: [Function, Type, Effect, ...],
  dependencies: [ModuleRef, ...],
  items: [FnDef, TypeDef, EffectDef, ...],
}

FnDef {
  name: String,
  signature: FnType,
  effects: EffectSet,
  body: BasicBlock[],
  capabilities: [CapabilityRef, ...],
}

FnType {
  params: [Type, ...],
  return_type: Type,
}

Type {
  // Tagged union of all possible types
  | Primitive(Primitive)
  | Struct(FieldList)
  | Enum(VariantList)
  | Function(FnType)
  | Pointer(Type, Mutability)
  | Array(Type, Size)
  | Effect(EffectType)
  | Capability(CapabilityType)
  | Reference(Type)
}

BasicBlock {
  label: String,
  instructions: Instruction[],
  terminator: Terminator,
}

Instruction {
  // Effect-aware SSA form
  | Load(target: Var, source: Value)
  | Store(target: Value, source: Value)
  | Call(target: Var, fn: Value, args: [Value, ...], effects: EffectSet)
  | BinOp(target: Var, op: BinOp, left: Value, right: Value)
  | EffectOp(target: Var, effect: Effect, args: [Value, ...])
  | CapabilityCheck(cap: Capability, resource: Value)
  | Alloc(target: Var, size: Value, effect: Alloc)
  | Free(ptr: Value, effect: Alloc)
}

EffectSet = Set<Effect>
Effect = io | alloc | net | fail | async | rand | ...

CapabilityType {
  resource: String,  // e.g., "File", "Socket", "Memory"
  rights: Set<Right>,  // read, write, execute, delegate, revoke
}
```

## Code Generation Strategy

1. **Titan** → Omni-IR (direct; Titan is the IR reference language)
2. **Sylva** → Omni-IR (via type erasure during JIT compilation)
3. **Aether** → Omni-IR (actors become function groups + message dispatch)
4. **Axiom** → Omni-IR (extraction produces Titan source, which is then IR)

## Backend Targets

1. **Native** (via Cranelift)
2. **WASM** (via Cranelift WASM backend)
3. **GPU** (via NVPTX or SPIR-V)
4. **Bytecode** (for Omni-VM)

## Optimization Passes

1. **Effect ordering** – reorder pure operations, respect effect dependencies
2. **Capability elision** – remove redundant capability checks
3. **Region allocation** – promote stack allocation for short-lived objects
4. **Inlining** – across language boundaries
5. **LLVM optimization** – standard passes
