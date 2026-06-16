// SYLVA INTERPRETER

use crate::parser::{Program, Stmt, Expr};

pub fn interpret(program: &Program) -> Result<(), String> {
    for stmt in &program.statements {
        match stmt {
            Stmt::Assignment { name, value } => {
                println!(\"Assigned {} = {:?}\", name, value);
            },
            Stmt::Expression(expr) => {
                println!(\"Result: {:?}\", expr);
            }
        }
    }
    Ok(())
}
