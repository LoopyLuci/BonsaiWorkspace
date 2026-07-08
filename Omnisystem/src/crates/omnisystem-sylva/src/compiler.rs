#[derive(Debug, Clone)]
pub enum OpCode {
    PushInt(i64),
    Halt,
}

pub fn compile(_program: &crate::parser::Program) -> Vec<OpCode> {
    vec![OpCode::Halt]
}
