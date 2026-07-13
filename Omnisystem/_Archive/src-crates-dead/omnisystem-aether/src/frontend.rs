use async_trait::async_trait;
use language_system::LanguageFrontend;
use core_ir::{LairModule, ModuleMetadata};
use std::path::Path;

#[derive(Clone)]
pub struct AetherFrontend;

impl AetherFrontend {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl LanguageFrontend for AetherFrontend {
    fn language_name(&self) -> &str { "Aether" }
    fn file_extensions(&self) -> &[&str] { &["aether", "ae"] }

    async fn parse(&self, _source: &str, _path: &Path) -> language_system::Result<LairModule> {
        Ok(LairModule {
            name: "aether_module".into(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            metadata: ModuleMetadata {
                imports: vec![],
                exports: vec![],
                source_language: Some("Aether".into()),
            },
        })
    }
}
