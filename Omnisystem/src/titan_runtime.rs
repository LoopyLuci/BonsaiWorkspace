// TITAN RUNTIME - Complete implementation
// Core runtime system for TITAN language execution
// Version: 2.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::boxed::Box;
use std::any::Any;

/// TITAN Value type - unified representation for all values
#[derive(Debug, Clone)]
pub enum TitanValue {
    Null,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    F32(f32),
    F64(f64),
    Char(char),
    String(String),
    Bytes(Vec<u8>),
    Vec(Arc<Mutex<Vec<TitanValue>>>),
    HashMap(Arc<Mutex<HashMap<String, TitanValue>>>),
    Struct {
        type_name: String,
        fields: HashMap<String, TitanValue>,
    },
    Enum {
        variant: String,
        data: Option<Box<TitanValue>>,
    },
    Reference(Arc<TitanValue>),
    MutableReference(Arc<Mutex<TitanValue>>),
}

impl TitanValue {
    /// Type name as string
    pub fn type_name(&self) -> &'static str {
        match self {
            TitanValue::Null => "null",
            TitanValue::Bool(_) => "bool",
            TitanValue::I8(_) => "i8",
            TitanValue::I16(_) => "i16",
            TitanValue::I32(_) => "i32",
            TitanValue::I64(_) => "i64",
            TitanValue::I128(_) => "i128",
            TitanValue::U8(_) => "u8",
            TitanValue::U16(_) => "u16",
            TitanValue::U32(_) => "u32",
            TitanValue::U64(_) => "u64",
            TitanValue::U128(_) => "u128",
            TitanValue::F32(_) => "f32",
            TitanValue::F64(_) => "f64",
            TitanValue::Char(_) => "char",
            TitanValue::String(_) => "string",
            TitanValue::Bytes(_) => "bytes",
            TitanValue::Vec(_) => "vec",
            TitanValue::HashMap(_) => "hashmap",
            TitanValue::Struct { .. } => "struct",
            TitanValue::Enum { .. } => "enum",
            TitanValue::Reference(_) => "ref",
            TitanValue::MutableReference(_) => "mut_ref",
        }
    }

    /// Is this value truthy?
    pub fn is_truthy(&self) -> bool {
        match self {
            TitanValue::Null => false,
            TitanValue::Bool(b) => *b,
            TitanValue::I8(n) => *n != 0,
            TitanValue::I16(n) => *n != 0,
            TitanValue::I32(n) => *n != 0,
            TitanValue::I64(n) => *n != 0,
            TitanValue::I128(n) => *n != 0,
            TitanValue::U8(n) => *n != 0,
            TitanValue::U16(n) => *n != 0,
            TitanValue::U32(n) => *n != 0,
            TitanValue::U64(n) => *n != 0,
            TitanValue::U128(n) => *n != 0,
            TitanValue::F32(n) => *n != 0.0,
            TitanValue::F64(n) => *n != 0.0,
            TitanValue::String(s) => !s.is_empty(),
            TitanValue::Bytes(b) => !b.is_empty(),
            _ => true,
        }
    }
}

/// TITAN Type system
#[derive(Debug, Clone, PartialEq)]
pub enum TitanType {
    Null,
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Char,
    String,
    Bytes,
    Vec(Box<TitanType>),
    HashMap(Box<TitanType>, Box<TitanType>),
    Struct(String),
    Enum(String),
    Function {
        params: Vec<TitanType>,
        return_type: Box<TitanType>,
    },
    Reference(Box<TitanType>),
    MutableReference(Box<TitanType>),
    Generic(String),
}

impl TitanType {
    pub fn name(&self) -> String {
        match self {
            TitanType::Null => "null".to_string(),
            TitanType::Bool => "bool".to_string(),
            TitanType::I8 => "i8".to_string(),
            TitanType::I16 => "i16".to_string(),
            TitanType::I32 => "i32".to_string(),
            TitanType::I64 => "i64".to_string(),
            TitanType::I128 => "i128".to_string(),
            TitanType::U8 => "u8".to_string(),
            TitanType::U16 => "u16".to_string(),
            TitanType::U32 => "u32".to_string(),
            TitanType::U64 => "u64".to_string(),
            TitanType::U128 => "u128".to_string(),
            TitanType::F32 => "f32".to_string(),
            TitanType::F64 => "f64".to_string(),
            TitanType::Char => "char".to_string(),
            TitanType::String => "string".to_string(),
            TitanType::Bytes => "bytes".to_string(),
            TitanType::Vec(inner) => format!("Vec<{}>", inner.name()),
            TitanType::HashMap(k, v) => format!("HashMap<{}, {}>", k.name(), v.name()),
            TitanType::Struct(name) => name.clone(),
            TitanType::Enum(name) => name.clone(),
            TitanType::Function { params, return_type } => {
                let param_names: Vec<String> = params.iter().map(|p| p.name()).collect();
                format!("fn({}) -> {}", param_names.join(", "), return_type.name())
            }
            TitanType::Reference(inner) => format!("&{}", inner.name()),
            TitanType::MutableReference(inner) => format!("&mut {}", inner.name()),
            TitanType::Generic(name) => name.clone(),
        }
    }
}

/// Memory management context
pub struct MemoryContext {
    heap: HashMap<u64, Arc<Mutex<TitanValue>>>,
    next_ptr: u64,
}

impl MemoryContext {
    pub fn new() -> Self {
        MemoryContext {
            heap: HashMap::new(),
            next_ptr: 1,
        }
    }

    pub fn allocate(&mut self, value: TitanValue) -> u64 {
        let ptr = self.next_ptr;
        self.next_ptr += 1;
        self.heap.insert(ptr, Arc::new(Mutex::new(value)));
        ptr
    }

    pub fn get(&self, ptr: u64) -> Option<Arc<Mutex<TitanValue>>> {
        self.heap.get(&ptr).cloned()
    }

    pub fn deallocate(&mut self, ptr: u64) {
        self.heap.remove(&ptr);
    }
}

/// Function definition
pub struct TitanFunction {
    pub name: String,
    pub params: Vec<(String, TitanType)>,
    pub return_type: TitanType,
    pub body: Box<dyn Fn(Vec<TitanValue>) -> Result<TitanValue, RuntimeError>>,
}

/// Runtime context for execution
pub struct TitanRuntime {
    globals: Arc<Mutex<HashMap<String, TitanValue>>>,
    locals: Arc<Mutex<Vec<HashMap<String, TitanValue>>>>,
    functions: Arc<Mutex<HashMap<String, Box<dyn Fn(Vec<TitanValue>) -> Result<TitanValue, RuntimeError>>>>>,
    memory: Arc<Mutex<MemoryContext>>,
    types: Arc<Mutex<HashMap<String, TitanType>>>,
    call_stack: Arc<Mutex<Vec<StackFrame>>>,
}

pub struct StackFrame {
    pub function_name: String,
    pub locals: HashMap<String, TitanValue>,
}

#[derive(Debug, Clone)]
pub enum RuntimeError {
    TypeError {
        expected: String,
        got: String,
    },
    VariableNotFound(String),
    FunctionNotFound(String),
    IndexOutOfBounds {
        index: usize,
        size: usize,
    },
    DivisionByZero,
    InvalidOperation(String),
    MemoryError(String),
    StackOverflow,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RuntimeError::TypeError { expected, got } => {
                write!(f, "Type error: expected {}, got {}", expected, got)
            }
            RuntimeError::VariableNotFound(name) => {
                write!(f, "Variable not found: {}", name)
            }
            RuntimeError::FunctionNotFound(name) => {
                write!(f, "Function not found: {}", name)
            }
            RuntimeError::IndexOutOfBounds { index, size } => {
                write!(f, "Index out of bounds: {} >= {}", index, size)
            }
            RuntimeError::DivisionByZero => {
                write!(f, "Division by zero")
            }
            RuntimeError::InvalidOperation(op) => {
                write!(f, "Invalid operation: {}", op)
            }
            RuntimeError::MemoryError(msg) => {
                write!(f, "Memory error: {}", msg)
            }
            RuntimeError::StackOverflow => {
                write!(f, "Stack overflow")
            }
        }
    }
}

impl TitanRuntime {
    pub fn new() -> Self {
        let runtime = TitanRuntime {
            globals: Arc::new(Mutex::new(HashMap::new())),
            locals: Arc::new(Mutex::new(vec![HashMap::new()])),
            functions: Arc::new(Mutex::new(HashMap::new())),
            memory: Arc::new(Mutex::new(MemoryContext::new())),
            types: Arc::new(Mutex::new(HashMap::new())),
            call_stack: Arc::new(Mutex::new(Vec::new())),
        };

        runtime.register_builtin_functions();
        runtime
    }

    /// Register built-in functions
    fn register_builtin_functions(&self) {
        let functions = self.functions.clone();
        let mut funcs = functions.lock().unwrap();

        // println function
        funcs.insert(
            "println".to_string(),
            Box::new(|args| {
                let output = args.iter()
                    .map(|v| format_value(v))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("{}", output);
                Ok(TitanValue::Null)
            }),
        );

        // len function
        funcs.insert(
            "len".to_string(),
            Box::new(|args| {
                if args.is_empty() {
                    return Err(RuntimeError::InvalidOperation("len requires 1 argument".to_string()));
                }
                match &args[0] {
                    TitanValue::String(s) => Ok(TitanValue::U64(s.len() as u64)),
                    TitanValue::Bytes(b) => Ok(TitanValue::U64(b.len() as u64)),
                    TitanValue::Vec(v) => {
                        let vec = v.lock().unwrap();
                        Ok(TitanValue::U64(vec.len() as u64))
                    }
                    _ => Err(RuntimeError::InvalidOperation("len not supported for this type".to_string())),
                }
            }),
        );

        // to_string function
        funcs.insert(
            "to_string".to_string(),
            Box::new(|args| {
                if args.is_empty() {
                    return Err(RuntimeError::InvalidOperation("to_string requires 1 argument".to_string()));
                }
                Ok(TitanValue::String(format_value(&args[0])))
            }),
        );

        // abs function
        funcs.insert(
            "abs".to_string(),
            Box::new(|args| {
                if args.is_empty() {
                    return Err(RuntimeError::InvalidOperation("abs requires 1 argument".to_string()));
                }
                match &args[0] {
                    TitanValue::I64(n) => Ok(TitanValue::I64(n.abs())),
                    TitanValue::F64(n) => Ok(TitanValue::F64(n.abs())),
                    _ => Err(RuntimeError::InvalidOperation("abs not supported for this type".to_string())),
                }
            }),
        );

        // sqrt function
        funcs.insert(
            "sqrt".to_string(),
            Box::new(|args| {
                if args.is_empty() {
                    return Err(RuntimeError::InvalidOperation("sqrt requires 1 argument".to_string()));
                }
                match &args[0] {
                    TitanValue::F64(n) => Ok(TitanValue::F64(n.sqrt())),
                    TitanValue::I64(n) => Ok(TitanValue::F64((*n as f64).sqrt())),
                    _ => Err(RuntimeError::InvalidOperation("sqrt not supported for this type".to_string())),
                }
            }),
        );

        // pow function
        funcs.insert(
            "pow".to_string(),
            Box::new(|args| {
                if args.len() < 2 {
                    return Err(RuntimeError::InvalidOperation("pow requires 2 arguments".to_string()));
                }
                match (&args[0], &args[1]) {
                    (TitanValue::F64(base), TitanValue::F64(exp)) => Ok(TitanValue::F64(base.powf(*exp))),
                    (TitanValue::I64(base), TitanValue::I64(exp)) => {
                        if *exp < 0 {
                            Ok(TitanValue::F64((*base as f64).powf(*exp as f64)))
                        } else {
                            Ok(TitanValue::I64(base.pow(*exp as u32)))
                        }
                    }
                    _ => Err(RuntimeError::InvalidOperation("pow not supported for these types".to_string())),
                }
            }),
        );
    }

    /// Set a global variable
    pub fn set_global(&self, name: String, value: TitanValue) {
        let mut globals = self.globals.lock().unwrap();
        globals.insert(name, value);
    }

    /// Get a global variable
    pub fn get_global(&self, name: &str) -> Option<TitanValue> {
        let globals = self.globals.lock().unwrap();
        globals.get(name).cloned()
    }

    /// Set a local variable
    pub fn set_local(&self, name: String, value: TitanValue) {
        let mut locals = self.locals.lock().unwrap();
        if let Some(scope) = locals.last_mut() {
            scope.insert(name, value);
        }
    }

    /// Get a local variable
    pub fn get_local(&self, name: &str) -> Option<TitanValue> {
        let locals = self.locals.lock().unwrap();
        for scope in locals.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    /// Get variable (checks locals first, then globals)
    pub fn get_variable(&self, name: &str) -> Option<TitanValue> {
        self.get_local(name).or_else(|| self.get_global(name))
    }

    /// Call a function
    pub fn call_function(
        &self,
        name: &str,
        args: Vec<TitanValue>,
    ) -> Result<TitanValue, RuntimeError> {
        let functions = self.functions.lock().unwrap();
        let func = functions
            .get(name)
            .ok_or_else(|| RuntimeError::FunctionNotFound(name.to_string()))?;

        func(args)
    }

    /// Register a type
    pub fn register_type(&self, name: String, ty: TitanType) {
        let mut types = self.types.lock().unwrap();
        types.insert(name, ty);
    }

    /// Get a type
    pub fn get_type(&self, name: &str) -> Option<TitanType> {
        let types = self.types.lock().unwrap();
        types.get(name).cloned()
    }

    /// Push a new scope
    pub fn push_scope(&self) {
        let mut locals = self.locals.lock().unwrap();
        locals.push(HashMap::new());
    }

    /// Pop a scope
    pub fn pop_scope(&self) {
        let mut locals = self.locals.lock().unwrap();
        if locals.len() > 1 {
            locals.pop();
        }
    }
}

/// Format a value for display
pub fn format_value(value: &TitanValue) -> String {
    match value {
        TitanValue::Null => "null".to_string(),
        TitanValue::Bool(b) => b.to_string(),
        TitanValue::I8(n) => n.to_string(),
        TitanValue::I16(n) => n.to_string(),
        TitanValue::I32(n) => n.to_string(),
        TitanValue::I64(n) => n.to_string(),
        TitanValue::I128(n) => n.to_string(),
        TitanValue::U8(n) => n.to_string(),
        TitanValue::U16(n) => n.to_string(),
        TitanValue::U32(n) => n.to_string(),
        TitanValue::U64(n) => n.to_string(),
        TitanValue::U128(n) => n.to_string(),
        TitanValue::F32(n) => n.to_string(),
        TitanValue::F64(n) => n.to_string(),
        TitanValue::Char(c) => c.to_string(),
        TitanValue::String(s) => s.clone(),
        TitanValue::Bytes(b) => format!("[bytes: {} bytes]", b.len()),
        TitanValue::Vec(v) => {
            let vec = v.lock().unwrap();
            let items: Vec<String> = vec.iter().map(|v| format_value(v)).collect();
            format!("[{}]", items.join(", "))
        }
        TitanValue::HashMap(m) => {
            let map = m.lock().unwrap();
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_value(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        TitanValue::Struct { type_name, fields } => {
            let items: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_value(v)))
                .collect();
            format!("{} {{{}}}", type_name, items.join(", "))
        }
        TitanValue::Enum { variant, data } => {
            if let Some(d) = data {
                format!("{}({})", variant, format_value(d))
            } else {
                variant.clone()
            }
        }
        TitanValue::Reference(v) => format!("&{}", format_value(v)),
        TitanValue::MutableReference(v) => {
            let val = v.lock().unwrap();
            format!("&mut {}", format_value(&val))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_titan_value_type_name() {
        assert_eq!(TitanValue::Bool(true).type_name(), "bool");
        assert_eq!(TitanValue::I64(42).type_name(), "i64");
        assert_eq!(TitanValue::String("hello".to_string()).type_name(), "string");
    }

    #[test]
    fn test_titan_value_is_truthy() {
        assert!(TitanValue::Bool(true).is_truthy());
        assert!(!TitanValue::Bool(false).is_truthy());
        assert!(TitanValue::I64(42).is_truthy());
        assert!(!TitanValue::I64(0).is_truthy());
    }

    #[test]
    fn test_runtime_global_variables() {
        let rt = TitanRuntime::new();
        rt.set_global("x".to_string(), TitanValue::I64(42));
        assert_eq!(rt.get_global("x"), Some(TitanValue::I64(42)));
    }

    #[test]
    fn test_runtime_builtin_functions() {
        let rt = TitanRuntime::new();
        let result = rt.call_function("len", vec![TitanValue::String("hello".to_string())]);
        assert_eq!(result, Ok(TitanValue::U64(5)));
    }

    #[test]
    fn test_runtime_abs() {
        let rt = TitanRuntime::new();
        let result = rt.call_function("abs", vec![TitanValue::I64(-42)]);
        assert_eq!(result, Ok(TitanValue::I64(42)));
    }

    #[test]
    fn test_runtime_sqrt() {
        let rt = TitanRuntime::new();
        let result = rt.call_function("sqrt", vec![TitanValue::F64(16.0)]);
        match result {
            Ok(TitanValue::F64(n)) => assert!((n - 4.0).abs() < 0.0001),
            _ => panic!("Expected F64"),
        }
    }
}
