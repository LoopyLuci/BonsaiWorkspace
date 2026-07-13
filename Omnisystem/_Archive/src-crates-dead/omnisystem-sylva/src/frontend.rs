use async_trait::async_trait;
use language_system::LanguageFrontend;
use core_ir::{LairModule, ModuleMetadata};
use std::path::Path;

#[derive(Clone)]
pub struct SylvaFrontend;

impl SylvaFrontend {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl LanguageFrontend for SylvaFrontend {
    fn language_name(&self) -> &str { "Sylva" }
    fn file_extensions(&self) -> &[&str] { &["syl", "sylva"] }
    
    async fn parse(&self, source: &str, _path: &Path) -> language_system::Result<LairModule> {
        let program = crate::parser::parse(source).map_err(|e| language_system::FrontendError::ParseError(e.to_string()))?;
        let _bytecode = crate::compiler::compile(&program);
        
        Ok(LairModule {
            name: "sylva_module".into(),
            functions: vec![],
            types: vec![],
            constants: vec![],
            metadata: ModuleMetadata {
                imports: vec![],
                exports: vec![],
                source_language: Some("Sylva".into()),
            },
        })
    }
}
