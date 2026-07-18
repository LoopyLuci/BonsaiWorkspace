//! buir_cli - small demo/inspection tool for the BUIR intermediate
//! representation: builds a minimal module, hashes it, and round-trips it
//! through the byte serializer.

use buir::{
    hash_buir, hash_function, serialize_to_bytes, deserialize_from_bytes, BasicBlock, BuirFunction,
    BuirModule, BuirType, EffectSet, Instruction, Language, SsaBody, Terminator, Value,
};

fn demo_module() -> BuirModule {
    let add_fn = BuirFunction {
        name: "add".to_string(),
        signature: BuirType::Function {
            params: vec![BuirType::I32, BuirType::I32],
            returns: Box::new(BuirType::I32),
        },
        body: Some(SsaBody {
            parameters: vec![BuirType::I32, BuirType::I32],
            blocks: vec![BasicBlock {
                instructions: vec![Instruction::Add {
                    lhs: Value { id: 0, ty: BuirType::I32 },
                    rhs: Value { id: 1, ty: BuirType::I32 },
                    result: Value { id: 2, ty: BuirType::I32 },
                }],
                terminator: Terminator::Return(Some(Value { id: 2, ty: BuirType::I32 })),
            }],
        }),
        version: 1,
        effects: EffectSet::default(),
        language: Language::Rust,
        symbol_name: "buir_demo_add".to_string(),
    };

    BuirModule {
        functions: vec![add_fn],
        types: vec![],
        globals: vec![],
        language: Language::Rust,
        source_hash: "demo".to_string(),
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn main() -> anyhow::Result<()> {
    let module = demo_module();

    let module_hash = hash_buir(&module);
    println!("module hash: {module_hash}");

    let fn_hash = hash_function(&module.functions[0]);
    println!("function hash: {}", fn_hash.0.iter().map(|b| format!("{b:02x}")).collect::<String>());

    let bytes = serialize_to_bytes(&module)?;
    println!("serialized to {} bytes", bytes.len());

    let round_tripped = deserialize_from_bytes(&bytes)?;
    assert_eq!(hash_buir(&round_tripped), module_hash, "round-trip changed module hash");
    println!("round-trip OK, {} function(s) recovered", round_tripped.functions.len());

    Ok(())
}
