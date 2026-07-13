use async_trait::async_trait;
use language_system::LanguageFrontend;
use core_ir::LairModule;
use std::path::Path;

#[derive(Clone)]
pub struct TitanFrontend;

impl TitanFrontend {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl LanguageFrontend for TitanFrontend {
    fn language_name(&self) -> &str { "Titan" }
    fn file_extensions(&self) -> &[&str] { &["titan", "ti"] }
    
    async fn parse(&self, source: &str, _path: &Path) -> language_system::Result<LairModule> {
        let program = crate::parser::parse(source).map_err(|e| language_system::FrontendError::ParseError(e.to_string()))?;
        let checker = crate::typeck::TypeChecker::new();
        checker.check(&program).map_err(|e| language_system::FrontendError::TypeError(e.to_string()))?;
        let lair = crate::lower::lower_program(&program);
        Ok(lair)
    }
}
