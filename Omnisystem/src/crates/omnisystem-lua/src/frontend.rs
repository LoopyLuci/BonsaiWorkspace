use language_system::LanguageFrontend;
use core_ir::*;
use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;

pub struct LuaFrontend;

impl LuaFrontend {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl LanguageFrontend for LuaFrontend {
    fn language_name(&self) -> &str { "Lua" }
    fn file_extensions(&self) -> &[&str] { &["lua"] }

    async fn parse(&self, _source: &str, _path: &Path) -> Result<LairModule> {
        Ok(LairModule {
            name: "lua_module".into(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            metadata: core_ir::ModuleMetadata {
                imports: vec![],
                exports: vec![],
                source_language: Some("Lua".into()),
            },
        })
    }
}
