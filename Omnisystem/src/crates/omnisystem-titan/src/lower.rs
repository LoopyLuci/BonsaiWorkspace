use crate::ast::*;
use core_ir::{LairModule, ModuleMetadata};

pub fn lower_program(_program: &Program) -> LairModule {
    LairModule {
        name: "titan_module".into(),
        functions: vec![],
        types: vec![],
        constants: vec![],
        metadata: ModuleMetadata {
            imports: vec![],
            exports: vec![],
            source_language: Some("Titan".into()),
        },
    }
}
