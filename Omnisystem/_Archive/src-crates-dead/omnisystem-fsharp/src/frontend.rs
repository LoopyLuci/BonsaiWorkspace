use language_system::LanguageFrontend;
use core_ir::*;
use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;

pub struct F#Frontend;

impl F#Frontend {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl LanguageFrontend for F#Frontend {
    fn language_name(&self) -> &str { "F#" }
    fn file_extensions(&self) -> &[&str] { &["fs", "fsx"] }

    async fn parse(&self, _source: &str, _path: &Path) -> Result<LairModule> {
        Ok(LairModule {
            name: "f#_module".into(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            metadata: core_ir::ModuleMetadata {
                imports: vec![],
                exports: vec![],
                source_language: Some("F#".into()),
            },
        })
    }
}
