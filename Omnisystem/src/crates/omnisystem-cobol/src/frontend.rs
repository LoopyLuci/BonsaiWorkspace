use language_system::LanguageFrontend;
use core_ir::*;
use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;

pub struct COBOLFrontend;

impl COBOLFrontend {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl LanguageFrontend for COBOLFrontend {
    fn language_name(&self) -> &str { "COBOL" }
    fn file_extensions(&self) -> &[&str] { &["cob", "cbl"] }

    async fn parse(&self, _source: &str, _path: &Path) -> Result<LairModule> {
        Ok(LairModule {
            name: "cobol_module".into(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            metadata: core_ir::ModuleMetadata {
                imports: vec![],
                exports: vec![],
                source_language: Some("COBOL".into()),
            },
        })
    }
}
