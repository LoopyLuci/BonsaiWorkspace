#[derive(Debug, Clone)]
pub enum Expr {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    Nil,
    Identifier(String),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Expr>,
}

pub fn parse(source: &str) -> anyhow::Result<Program> {
    let mut statements = Vec::new();
    for line in source.lines() {
        if let Ok(num) = line.trim().parse::<i64>() {
            statements.push(Expr::IntLiteral(num));
        }
    }
    Ok(Program { statements })
}
