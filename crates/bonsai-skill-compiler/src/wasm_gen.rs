use anyhow::Result;
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection,
    ImportSection, Module, TypeSection, ValType,
};

use crate::{extractor::Rule, parser::SkillMetadata};

/// Generate a minimal WASM module that exports `invoke(ptr: i32, len: i32) -> i32`.
///
/// Imports `bonsai_log(ptr, len)` from `"env"`.  The body calls the log import
/// with the incoming pointer/length and returns 0 (success).  Real rule logic
/// replaces this skeleton in later compiler stages.
pub fn generate_wasm_skeleton(metadata: &SkillMetadata, rules: &[Rule]) -> Result<Vec<u8>> {
    let mut module = Module::new();

    // ── Type section ──────────────────────────────────────────────────────────
    // Type 0: (i32, i32) -> ()   — bonsai_log
    // Type 1: (i32, i32) -> i32  — invoke
    let mut types = TypeSection::new();
    types.function([ValType::I32, ValType::I32], []);
    types.function([ValType::I32, ValType::I32], [ValType::I32]);
    module.section(&types);

    // ── Import: bonsai_log (type 0) from "env" ────────────────────────────────
    let mut imports = ImportSection::new();
    imports.import("env", "bonsai_log", EntityType::Function(0));
    module.section(&imports);

    // ── Function: invoke (type 1) at index 1 ─────────────────────────────────
    let mut funcs = FunctionSection::new();
    funcs.function(1);
    module.section(&funcs);

    // ── Export: "invoke" → func 1 ────────────────────────────────────────────
    let mut exports = ExportSection::new();
    exports.export("invoke", ExportKind::Func, 1);
    module.section(&exports);

    // ── Code: forward (ptr, len) to bonsai_log, return 0 ─────────────────────
    let mut code = CodeSection::new();
    let mut func = Function::new([]); // no extra locals
    func.instruction(&wasm_encoder::Instruction::LocalGet(0));
    func.instruction(&wasm_encoder::Instruction::LocalGet(1));
    func.instruction(&wasm_encoder::Instruction::Call(0)); // bonsai_log
    func.instruction(&wasm_encoder::Instruction::I32Const(0));
    func.instruction(&wasm_encoder::Instruction::End);
    code.function(&func);
    module.section(&code);

    let wasm = module.finish();

    tracing::debug!(
        skill = %metadata.name,
        rules = rules.len(),
        wasm_bytes = wasm.len(),
        "generated WASM skeleton"
    );

    Ok(wasm)
}
