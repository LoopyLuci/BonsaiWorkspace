use language_system::LanguageFrontend;
use core_ir::*;
use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;

pub struct FORTRAN77Frontend;

impl FORTRAN77Frontend {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl LanguageFrontend for FORTRAN77Frontend {
    fn language_name(&self) -> &str { "FORTRAN 77" }
    fn file_extensions(&self) -> &[&str] { &["f", "for"] }

    async fn parse(&self, _source: &str, _path: &Path) -> Result<LairModule> {
        Ok(LairModule {
            name: "fortran 77_module".into(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            metadata: core_ir::ModuleMetadata {
                imports: vec![],
                exports: vec![],
                source_language: Some("FORTRAN 77".into()),
            },
        })
    }
}
