use crate::ast::*;

pub fn parse(source: &str) -> anyhow::Result<Program> {
    if source.trim().is_empty() {
        return Ok(Program { items: vec![] });
    }
    Ok(Program { items: vec![] })
}
