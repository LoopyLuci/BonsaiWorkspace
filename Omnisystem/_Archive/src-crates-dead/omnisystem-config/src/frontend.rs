use async_trait::async_trait;
use language_system::LanguageFrontend;
use core_ir::{LairModule, ModuleMetadata};
use std::path::Path;

#[derive(Clone)]
pub struct ConfigFrontend;

impl ConfigFrontend {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl LanguageFrontend for ConfigFrontend {
    fn language_name(&self) -> &str { "Config" }
    fn file_extensions(&self) -> &[&str] { &["config"] }
    
    async fn parse(&self, _source: &str, file_path: &Path) -> language_system::Result<LairModule> {
        Ok(LairModule {
            name: file_path.file_stem().unwrap().to_string_lossy().to_string(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            metadata: ModuleMetadata {
                imports: vec![],
                exports: vec![],
                source_language: Some("Config".into()),
            },
        })
    }
}
