use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use arc_swap::ArcSwap;
use bonsai_skill_compiler::{CompiledSkill, SecurityReport, SkillToolDef, ToolRegistryMut};

/// Thread-safe, hot-swappable registry of compiled skills.
///
/// Reads are fully lock-free via `arc-swap`. Writes clone the current map,
/// insert/replace, and atomically swap in the new version — so readers always
/// see a consistent snapshot.
pub struct ToolRegistry {
    tools: ArcSwap<HashMap<String, CompiledSkill>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: ArcSwap::new(Arc::new(HashMap::new())) }
    }

    /// Register or replace a skill by name.
    pub fn register(&self, skill: CompiledSkill) {
        let mut map = (**self.tools.load()).clone();
        map.insert(skill.name.clone(), skill);
        self.tools.store(Arc::new(map));
    }

    /// Look up a skill by name.
    pub fn get(&self, name: &str) -> Option<CompiledSkill> {
        self.tools.load().get(name).cloned()
    }

    /// List all registered tool definitions (without WASM bytes).
    pub fn list(&self) -> Vec<SkillToolDef> {
        self.tools.load().values().map(|s| SkillToolDef {
            name: s.name.clone(),
            description: s.description.clone(),
            category: "skill".into(),
            tags: s.tags.clone(),
            requires_permissions: s.requires_permissions.clone(),
            sandbox_tier: "wasm".into(),
        }).collect()
    }

    /// Atomically replace the entire registry (used for bulk hot-reload).
    pub fn hot_swap(&self, new_map: HashMap<String, CompiledSkill>) {
        self.tools.store(Arc::new(new_map));
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::new() }
}

/// Implement the compiler's `ToolRegistryMut` so `register_compiled_skill()`
/// can be called directly with a `&ToolRegistry`.
impl ToolRegistryMut for ToolRegistry {
    fn register_wasm_tool(&self, def: SkillToolDef, wasm_bytes: Vec<u8>) -> Result<()> {
        let wasm_hash = sha2_hash(&wasm_bytes);
        let skill = CompiledSkill {
            id: format!("local/{}", def.name),
            name: def.name.clone(),
            description: def.description.clone(),
            tags: def.tags.clone(),
            wasm_bytes,
            wasm_hash,
            security_report: SecurityReport {
                passed: true,
                concerns: vec![],
                content_hash: String::new(),
            },
            requires_permissions: def.requires_permissions.clone(),
            rules: vec![],
        };
        self.register(skill);
        Ok(())
    }
}

fn sha2_hash(data: &[u8]) -> String {
    hex::encode(blake3::hash(data).as_bytes())
}
