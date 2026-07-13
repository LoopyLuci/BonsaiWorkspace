use crate::compiler::OpCode;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Nil,
}

pub struct Vm {
    code: Vec<OpCode>,
    stack: Vec<Value>,
}

impl Vm {
    pub fn new(code: Vec<OpCode>) -> Self {
        Self { code, stack: Vec::new() }
    }
    
    pub fn run(&mut self) -> anyhow::Result<Value> {
        for op in &self.code {
            match op {
                OpCode::PushInt(v) => self.stack.push(Value::Int(*v)),
                OpCode::Halt => break,
            }
        }
        Ok(self.stack.pop().unwrap_or(Value::Nil))
    }
}
