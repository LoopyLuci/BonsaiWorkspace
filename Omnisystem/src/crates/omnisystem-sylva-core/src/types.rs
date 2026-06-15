// Sylva Type System - unified types across all implementations

use serde::{Deserialize, Serialize};

/// Sylva type system - maps to equivalent types in all 750+ languages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    // Primitives
    Null,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    String,

    // Collections
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),

    // Objects
    Struct(String), // struct name
    Enum(String),   // enum name

    // Advanced
    Handle(String), // opaque handle to resource
    Future(Box<Type>), // async result type
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>), // Ok, Err
    Custom, // arbitrary JSON/custom values

    // Functions
    Function(Vec<Type>, Box<Type>), // params, return
}

/// Runtime value in Sylva
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Array(Vec<Value>),
    Map(std::collections::BTreeMap<String, Value>),
    Handle(String),
    Custom(serde_json::Value),
}

impl Value {
    pub fn type_of(&self) -> Type {
        match self {
            Value::Null => Type::Null,
            Value::Bool(_) => Type::Bool,
            Value::I8(_) => Type::I8,
            Value::I16(_) => Type::I16,
            Value::I32(_) => Type::I32,
            Value::I64(_) => Type::I64,
            Value::U8(_) => Type::U8,
            Value::U16(_) => Type::U16,
            Value::U32(_) => Type::U32,
            Value::U64(_) => Type::U64,
            Value::F32(_) => Type::F32,
            Value::F64(_) => Type::F64,
            Value::String(_) => Type::String,
            Value::Array(_) => Type::Array(Box::new(Type::Custom)),
            Value::Map(_) => Type::Map(Box::new(Type::String), Box::new(Type::Custom)),
            Value::Handle(_) => Type::Handle("handle".to_string()),
            Value::Custom(_) => Type::Custom,
        }
    }

    pub fn as_bool(&self) -> anyhow::Result<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(anyhow::anyhow!("Expected bool, got {:?}", self.type_of())),
        }
    }

    pub fn as_i64(&self) -> anyhow::Result<i64> {
        match self {
            Value::I64(i) => Ok(*i),
            Value::I32(i) => Ok(*i as i64),
            Value::I16(i) => Ok(*i as i64),
            Value::I8(i) => Ok(*i as i64),
            _ => Err(anyhow::anyhow!("Expected integer, got {:?}", self.type_of())),
        }
    }

    pub fn as_string(&self) -> anyhow::Result<String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            _ => Err(anyhow::anyhow!("Expected string, got {:?}", self.type_of())),
        }
    }

    pub fn as_array(&self) -> anyhow::Result<Vec<Value>> {
        match self {
            Value::Array(arr) => Ok(arr.clone()),
            _ => Err(anyhow::anyhow!("Expected array, got {:?}", self.type_of())),
        }
    }
}

// Custom type for type system compatibility
#[derive(Debug, Clone, Copy)]
pub struct Custom;

impl Type {
    pub fn from_json(val: &serde_json::Value) -> Self {
        match val {
            serde_json::Value::Null => Type::Null,
            serde_json::Value::Bool(_) => Type::Bool,
            serde_json::Value::Number(_) => Type::I64,
            serde_json::Value::String(_) => Type::String,
            serde_json::Value::Array(_) => Type::Array(Box::new(Type::Custom)),
            serde_json::Value::Object(_) => Type::Map(Box::new(Type::String), Box::new(Type::Custom)),
        }
    }

    pub fn is_compatible(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Null, Type::Null) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::I32, Type::I32) => true,
            (Type::I64, Type::I64) => true,
            (Type::String, Type::String) => true,
            (Type::Array(a), Type::Array(b)) => a.is_compatible(b),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_of() {
        assert_eq!(Value::Bool(true).type_of(), Type::Bool);
        assert_eq!(Value::I64(42).type_of(), Type::I64);
        assert_eq!(Value::String("hello".to_string()).type_of(), Type::String);
    }

    #[test]
    fn test_value_conversions() {
        let v = Value::Bool(true);
        assert!(v.as_bool().is_ok());

        let v = Value::I64(42);
        assert_eq!(v.as_i64().unwrap(), 42);

        let v = Value::String("test".to_string());
        assert_eq!(v.as_string().unwrap(), "test");
    }

    #[test]
    fn test_type_compatibility() {
        assert!(Type::Bool.is_compatible(&Type::Bool));
        assert!(!Type::Bool.is_compatible(&Type::I32));
    }
}
