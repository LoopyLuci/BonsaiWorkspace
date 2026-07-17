use crate::ir::BuirModule;
use anyhow::Result;

pub fn serialize_to_bytes(module: &BuirModule) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(module)?)
}

pub fn deserialize_from_bytes(bytes: &[u8]) -> Result<BuirModule> {
    Ok(serde_json::from_slice(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{EffectSet, Language};

    fn sample_module() -> BuirModule {
        BuirModule {
            functions: vec![],
            types: vec![],
            globals: vec![],
            language: Language::Rust,
            source_hash: "abc123".to_string(),
            compiler_version: "1.0.0".to_string(),
        }
    }

    #[test]
    fn round_trip_preserves_module() {
        let module = sample_module();
        let bytes = serialize_to_bytes(&module).expect("serialize should succeed");
        let restored = deserialize_from_bytes(&bytes).expect("deserialize should succeed");
        assert_eq!(restored.source_hash, module.source_hash);
        assert_eq!(restored.compiler_version, module.compiler_version);
    }

    #[test]
    fn deserialize_rejects_garbage() {
        let result = deserialize_from_bytes(b"not json");
        assert!(result.is_err());
    }

    #[test]
    fn effect_set_default_is_all_false() {
        let effects = EffectSet::default();
        assert!(!effects.async_);
        assert!(!effects.unsafe_);
        assert!(!effects.io);
        assert!(!effects.alloc);
        assert!(!effects.noreturn);
    }
}
