use crate::ast::*;

pub struct TypeChecker;

impl TypeChecker {
    pub fn new() -> Self { Self }
    pub fn check(&self, _program: &Program) -> anyhow::Result<()> {
        Ok(())
    }
}
