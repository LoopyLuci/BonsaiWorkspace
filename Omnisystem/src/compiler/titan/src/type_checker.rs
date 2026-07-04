// TITAN TYPE CHECKER - Semantic analysis and type checking

use crate::ast::*;
use std::collections::HashMap;

pub fn check(program: &Program) -> Result<(), String> {
    let mut checker = TypeChecker::new();
    checker.check_program(program)
}

struct TypeChecker {
    functions: HashMap<String, FunctionType>,
    variables: HashMap<String, String>,
}

struct FunctionType {
    params: Vec<String>,
    return_type: String,
}

impl TypeChecker {
    fn new() -> Self {
        TypeChecker {
            functions: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    fn check_program(&mut self, program: &Program) -> Result<(), String> {
        // First pass: collect function definitions
        for stmt in &program.statements {
            if let Stmt::FnDef { name, params, return_type, .. } = stmt {
                self.functions.insert(
                    name.clone(),
                    FunctionType {
                        params: params.iter().map(|(_, t)| t.clone().unwrap_or_default()).collect(),
                        return_type: return_type.clone().unwrap_or_else(|| "void".to_string()),
                    },
                );
            }
        }

        // Second pass: type check statements
        for stmt in &program.statements {
            self.check_statement(stmt)?;
        }

        Ok(())
    }

    fn check_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let { name, type_hint, value, .. } => {
                let _value_type = self.infer_type(value)?;
                self.variables.insert(name.clone(), type_hint.clone().unwrap_or_default());
                Ok(())
            }
            Stmt::FnDef { body, .. } => {
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                Ok(())
            }
            Stmt::Expression(expr) => {
                self.infer_type(expr)?;
                Ok(())
            }
            Stmt::Return(_) => Ok(()),
            Stmt::While { condition, body, .. } => {
                self.infer_type(condition)?;
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                Ok(())
            }
            Stmt::For { var, iterable, body, .. } => {
                self.infer_type(iterable)?;
                self.variables.insert(var.clone(), "i64".to_string());
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                Ok(())
            }
        }
    }

    fn infer_type(&self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Integer(_) => Ok("i64".to_string()),
            Expr::Float(_) => Ok("f64".to_string()),
            Expr::String(_) => Ok("String".to_string()),
            Expr::Boolean(_) => Ok("bool".to_string()),
            Expr::Identifier(name) => {
                self.variables
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {}", name))
            }
            Expr::BinaryOp { left, right, op } => {
                let _left_type = self.infer_type(left)?;
                let _right_type = self.infer_type(right)?;
                match op {
                    BinOp::Add | BinOp::Subtract | BinOp::Multiply | BinOp::Divide | BinOp::Modulo => {
                        Ok("i64".to_string())
                    }
                    BinOp::Equal | BinOp::NotEqual | BinOp::Less | BinOp::Greater
                    | BinOp::LessEqual | BinOp::GreaterEqual | BinOp::And | BinOp::Or => {
                        Ok("bool".to_string())
                    }
                    _ => Ok("i64".to_string()),
                }
            }
            Expr::UnaryOp { operand, .. } => {
                self.infer_type(operand)
            }
            Expr::Call { .. } => Ok("i64".to_string()),
            Expr::Array(_) => Ok("Array".to_string()),
            Expr::Index { .. } => Ok("i64".to_string()),
            Expr::If { then_branch, .. } => {
                if let Some(Stmt::Expression(expr)) = then_branch.last() {
                    self.infer_type(expr)
                } else {
                    Ok("void".to_string())
                }
            }
            Expr::Match { .. } => Ok("i64".to_string()),
            Expr::Block(_) => Ok("void".to_string()),
        }
    }
}
