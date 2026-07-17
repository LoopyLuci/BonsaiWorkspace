use crate::ir::{BuirModule, BuirFunction};

pub fn hash_buir(module: &BuirModule) -> String {
    let json = serde_json::to_vec(module).expect("BUIR serialization failed");
    let hash = blake3::hash(&json);
    hash.to_hex().to_string()
}

pub fn hash_function(function: &BuirFunction) -> FunctionHash {
    let json = serde_json::to_vec(function).expect("Function serialization failed");
    let hash = blake3::hash(&json);
    FunctionHash(*hash.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FunctionHash(pub [u8; 32]);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BuirFunction, BuirModule, BuirType, EffectSet, Language};

    fn sample_function(name: &str) -> BuirFunction {
        BuirFunction {
            name: name.to_string(),
            signature: BuirType::Function { params: vec![], returns: Box::new(BuirType::Void) },
            body: None,
            version: 1,
            effects: EffectSet::default(),
            language: Language::Rust,
            symbol_name: name.to_string(),
        }
    }

    fn sample_module(fn_name: &str) -> BuirModule {
        BuirModule {
            functions: vec![sample_function(fn_name)],
            types: vec![],
            globals: vec![],
            language: Language::Rust,
            source_hash: "abc".to_string(),
            compiler_version: "0.0.0".to_string(),
        }
    }

    #[test]
    fn hash_buir_is_deterministic() {
        let a = sample_module("foo");
        let b = sample_module("foo");
        assert_eq!(hash_buir(&a), hash_buir(&b));
    }

    #[test]
    fn hash_buir_changes_with_content() {
        let a = sample_module("foo");
        let b = sample_module("bar");
        assert_ne!(hash_buir(&a), hash_buir(&b));
    }

    #[test]
    fn hash_function_is_deterministic_and_32_bytes() {
        let f = sample_function("foo");
        let h1 = hash_function(&f);
        let h2 = hash_function(&f);
        assert_eq!(h1, h2);
        assert_eq!(h1.0.len(), 32);
    }
}
