// TITAN INTERPRETER - Runtime execution engine

use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Array(Vec<Value>),
    Null,
}

impl Value {
    fn to_string(&self) -> String {
        match self {
            Value::Integer(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", elements.join(", "))
            }
            Value::Null => "null".to_string(),
        }
    }

    fn to_bool(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Integer(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Null => false,
            _ => true,
        }
    }
}

pub fn interpret(program: &Program) -> Result<(), String> {
    let mut interpreter = Interpreter::new();
    interpreter.interpret_program(program)
}

pub fn compile_to_binary(program: &Program) -> Result<Vec<u8>, String> {
    // Placeholder: in a real implementation, this would generate machine code
    Ok(vec![0x7f, 0x45, 0x4c, 0x46]) // ELF magic bytes
}

pub fn generate_llvm_ir(program: &Program) -> Result<String, String> {
    // Generate LLVM IR from AST
    let mut ir = String::from("; LLVM IR generated from TITAN\n");
    ir.push_str("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
    ir.push_str("target triple = \"x86_64-unknown-linux-gnu\"\n\n");

    for stmt in &program.statements {
        if let Stmt::FnDef { name, params, return_type, body } = stmt {
            let ret_type = return_type.as_ref().map(|t| t.as_str()).unwrap_or("void");
            ir.push_str(&format!("define {} @{}(", ret_type, name));

            for (i, (param_name, param_type)) in params.iter().enumerate() {
                if i > 0 {
                    ir.push_str(", ");
                }
                ir.push_str(&format!("{} %{}", param_type.as_ref().unwrap_or(&"i64".to_string()), param_name));
            }

            ir.push_str(") {{\n");
            ir.push_str("entry:\n");
            ir.push_str("  ret void\n");
            ir.push_str("}\n\n");
        }
    }

    Ok(ir)
}

struct Interpreter {
    variables: HashMap<String, Value>,
    functions: HashMap<String, (Vec<(String, String)>, Vec<Stmt>)>,
}

impl Interpreter {
    fn new() -> Self {
        Interpreter {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    fn interpret_program(&mut self, program: &Program) -> Result<(), String> {
        // First pass: collect functions
        for stmt in &program.statements {
            if let Stmt::FnDef { name, params, body, .. } = stmt {
                let param_types: Vec<(String, String)> = params.iter().map(|(n, t)| {
                    (n.clone(), t.as_ref().unwrap_or(&"i64".to_string()).clone())
                }).collect();
                self.functions.insert(
                    name.clone(),
                    (param_types, body.clone()),
                );
            }
        }

        // Second pass: execute statements
        for stmt in &program.statements {
            self.execute_stmt(stmt)?;
        }

        Ok(())
    }

    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<Option<Value>, String> {
        match stmt {
            Stmt::Let { mutable: _, name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.variables.insert(name.clone(), val);
                Ok(None)
            }
            Stmt::FnDef { .. } => Ok(None),
            Stmt::Expression(expr) => {
                let val = self.eval_expr(expr)?;
                println!("{}", val.to_string());
                Ok(None)
            }
            Stmt::Return(Some(expr)) => {
                let val = self.eval_expr(expr)?;
                Ok(Some(val))
            }
            Stmt::Return(None) => Ok(Some(Value::Null)),
            Stmt::While { condition, body } => {
                while self.eval_expr(condition)?.to_bool() {
                    for stmt in body {
                        self.execute_stmt(stmt)?;
                    }
                }
                Ok(None)
            }
            Stmt::For { var, iterable, body } => {
                let iter_val = self.eval_expr(iterable)?;
                if let Value::Array(arr) = iter_val {
                    for elem in arr {
                        self.variables.insert(var.clone(), elem);
                        for stmt in body {
                            self.execute_stmt(stmt)?;
                        }
                    }
                }
                Ok(None)
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Integer(n) => Ok(Value::Integer(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::Identifier(name) => {
                self.variables
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {}", name))
            }
            Expr::BinaryOp { left, op, right } => {
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;
                self.apply_binary_op(&left_val, *op, &right_val)
            }
            Expr::UnaryOp { op, operand } => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnOp::Negate => {
                        match val {
                            Value::Integer(n) => Ok(Value::Integer(-n)),
                            Value::Float(f) => Ok(Value::Float(-f)),
                            _ => Err("Cannot negate non-numeric value".to_string()),
                        }
                    }
                    UnOp::Not => {
                        Ok(Value::Boolean(!val.to_bool()))
                    }
                    UnOp::BitwiseNot => {
                        match val {
                            Value::Integer(n) => Ok(Value::Integer(!n)),
                            _ => Err("Cannot apply bitwise NOT to non-integer".to_string()),
                        }
                    }
                }
            }
            Expr::Call { func, args } => {
                if let Expr::Identifier(fname) = &**func {
                    if let Some((params, body)) = self.functions.get(fname).cloned() {
                        let mut local_vars = self.variables.clone();

                        for (i, (param_name, _)) in params.iter().enumerate() {
                            if i < args.len() {
                                let arg_val = self.eval_expr(&args[i])?;
                                local_vars.insert(param_name.clone(), arg_val);
                            }
                        }

                        let mut func_interpreter = Interpreter {
                            variables: local_vars,
                            functions: self.functions.clone(),
                        };

                        for stmt in &body {
                            if let Some(ret_val) = func_interpreter.execute_stmt(stmt)? {
                                return Ok(ret_val);
                            }
                        }

                        return Ok(Value::Null);
                    }
                }
                Err("Unknown function call".to_string())
            }
            Expr::Array(elements) => {
                let mut arr = Vec::new();
                for elem in elements {
                    arr.push(self.eval_expr(elem)?);
                }
                Ok(Value::Array(arr))
            }
            Expr::Index { object, index } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;

                if let (Value::Array(arr), Value::Integer(i)) = (obj, idx) {
                    arr.get(i as usize)
                        .cloned()
                        .ok_or_else(|| "Array index out of bounds".to_string())
                } else {
                    Err("Invalid array indexing".to_string())
                }
            }
            Expr::If { condition, then_branch, else_branch } => {
                if self.eval_expr(condition)?.to_bool() {
                    for stmt in then_branch {
                        self.execute_stmt(stmt)?;
                    }
                    Ok(Value::Null)
                } else if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.execute_stmt(stmt)?;
                    }
                    Ok(Value::Null)
                } else {
                    Ok(Value::Null)
                }
            }
            Expr::Match { .. } => Ok(Value::Null),
            Expr::Block(_) => Ok(Value::Null),
        }
    }

    fn apply_binary_op(&self, left: &Value, op: BinOp, right: &Value) -> Result<Value, String> {
        match (left, right, op) {
            (Value::Integer(l), Value::Integer(r), BinOp::Add) => Ok(Value::Integer(l + r)),
            (Value::Integer(l), Value::Integer(r), BinOp::Subtract) => Ok(Value::Integer(l - r)),
            (Value::Integer(l), Value::Integer(r), BinOp::Multiply) => Ok(Value::Integer(l * r)),
            (Value::Integer(l), Value::Integer(r), BinOp::Divide) => {
                if *r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Integer(l / r))
                }
            }
            (Value::Integer(l), Value::Integer(r), BinOp::Modulo) => {
                if *r == 0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Integer(l % r))
                }
            }
            (l, r, BinOp::Equal) => Ok(Value::Boolean(self.values_equal(l, r))),
            (l, r, BinOp::NotEqual) => Ok(Value::Boolean(!self.values_equal(l, r))),
            (Value::Integer(l), Value::Integer(r), BinOp::Less) => Ok(Value::Boolean(l < r)),
            (Value::Integer(l), Value::Integer(r), BinOp::Greater) => Ok(Value::Boolean(l > r)),
            (Value::Integer(l), Value::Integer(r), BinOp::LessEqual) => Ok(Value::Boolean(l <= r)),
            (Value::Integer(l), Value::Integer(r), BinOp::GreaterEqual) => Ok(Value::Boolean(l >= r)),
            (l, r, BinOp::And) => Ok(Value::Boolean(l.to_bool() && r.to_bool())),
            (l, r, BinOp::Or) => Ok(Value::Boolean(l.to_bool() || r.to_bool())),
            _ => Err("Invalid binary operation".to_string()),
        }
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => l == r,
            (Value::Float(l), Value::Float(r)) => (l - r).abs() < f64::EPSILON,
            (Value::String(l), Value::String(r)) => l == r,
            (Value::Boolean(l), Value::Boolean(r)) => l == r,
            _ => false,
        }
    }
}
