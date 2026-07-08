//! Universal Type System
//!
//! Defines a unified type system that can represent values from any language.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{Result, UllError};

/// Universal value type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ValueType {
    Null,
    Boolean,
    Integer,
    Float,
    String,
    Bytes,
    Array,
    Object,
    Function,
    Module,
    Custom(String),
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean => write!(f, "boolean"),
            Self::Integer => write!(f, "integer"),
            Self::Float => write!(f, "float"),
            Self::String => write!(f, "string"),
            Self::Bytes => write!(f, "bytes"),
            Self::Array => write!(f, "array"),
            Self::Object => write!(f, "object"),
            Self::Function => write!(f, "function"),
            Self::Module => write!(f, "module"),
            Self::Custom(name) => write!(f, "custom({})", name),
        }
    }
}

/// Universal value that can be passed between languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Value {
    pub value_type: ValueType,
    pub data: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

impl Value {
    /// Create a null value
    pub fn null() -> Self {
        Self {
            value_type: ValueType::Null,
            data: serde_json::Value::Null,
            metadata: HashMap::new(),
        }
    }

    /// Create a boolean value
    pub fn boolean(value: bool) -> Self {
        Self {
            value_type: ValueType::Boolean,
            data: serde_json::Value::Bool(value),
            metadata: HashMap::new(),
        }
    }

    /// Create an integer value
    pub fn integer(value: i64) -> Self {
        Self {
            value_type: ValueType::Integer,
            data: serde_json::json!(value),
            metadata: HashMap::new(),
        }
    }

    /// Create a float value
    pub fn float(value: f64) -> Self {
        Self {
            value_type: ValueType::Float,
            data: serde_json::json!(value),
            metadata: HashMap::new(),
        }
    }

    /// Create a string value
    pub fn string(value: impl Into<String>) -> Self {
        Self {
            value_type: ValueType::String,
            data: serde_json::Value::String(value.into()),
            metadata: HashMap::new(),
        }
    }

    /// Create an array value
    pub fn array(values: Vec<Value>) -> Self {
        let data = serde_json::Value::Array(
            values.iter().map(|v| v.data.clone()).collect()
        );
        Self {
            value_type: ValueType::Array,
            data,
            metadata: HashMap::new(),
        }
    }

    /// Create an object value
    pub fn object(map: HashMap<String, Value>) -> Self {
        let mut obj = serde_json::json!({});
        for (k, v) in map {
            obj[k] = v.data;
        }
        Self {
            value_type: ValueType::Object,
            data: obj,
            metadata: HashMap::new(),
        }
    }

    /// Get as boolean
    pub fn as_bool(&self) -> Result<bool> {
        self.data
            .as_bool()
            .ok_or_else(|| UllError::type_conversion(
                format!("Cannot convert {} to bool", self.value_type)
            ))
    }

    /// Get as integer
    pub fn as_i64(&self) -> Result<i64> {
        self.data
            .as_i64()
            .ok_or_else(|| UllError::type_conversion(
                format!("Cannot convert {} to i64", self.value_type)
            ))
    }

    /// Get as float
    pub fn as_f64(&self) -> Result<f64> {
        self.data
            .as_f64()
            .ok_or_else(|| UllError::type_conversion(
                format!("Cannot convert {} to f64", self.value_type)
            ))
    }

    /// Get as string
    pub fn as_str(&self) -> Result<&str> {
        self.data
            .as_str()
            .ok_or_else(|| UllError::type_conversion(
                format!("Cannot convert {} to string", self.value_type)
            ))
    }

    /// Get as array
    pub fn as_array(&self) -> Result<Vec<Value>> {
        self.data
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|v| Value {
                        value_type: Self::infer_type(v),
                        data: v.clone(),
                        metadata: HashMap::new(),
                    })
                    .collect()
            })
            .ok_or_else(|| UllError::type_conversion(
                format!("Cannot convert {} to array", self.value_type)
            ))
    }

    /// Get as object
    pub fn as_object(&self) -> Result<HashMap<String, Value>> {
        self.data
            .as_object()
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            Value {
                                value_type: Self::infer_type(v),
                                data: v.clone(),
                                metadata: HashMap::new(),
                            },
                        )
                    })
                    .collect()
            })
            .ok_or_else(|| UllError::type_conversion(
                format!("Cannot convert {} to object", self.value_type)
            ))
    }

    /// Infer type from JSON value
    fn infer_type(value: &serde_json::Value) -> ValueType {
        match value {
            serde_json::Value::Null => ValueType::Null,
            serde_json::Value::Bool(_) => ValueType::Boolean,
            serde_json::Value::Number(_) => {
                if value.is_f64() {
                    ValueType::Float
                } else {
                    ValueType::Integer
                }
            }
            serde_json::Value::String(_) => ValueType::String,
            serde_json::Value::Array(_) => ValueType::Array,
            serde_json::Value::Object(_) => ValueType::Object,
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check type matches
    pub fn matches_type(&self, expected: &ValueType) -> bool {
        match (&self.value_type, expected) {
            (ValueType::Custom(a), ValueType::Custom(b)) => a == b,
            (a, b) => a == b,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.value_type, self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_creation() {
        assert_eq!(Value::null().value_type, ValueType::Null);
        assert_eq!(Value::boolean(true).value_type, ValueType::Boolean);
        assert_eq!(Value::integer(42).value_type, ValueType::Integer);
        assert_eq!(Value::float(3.14).value_type, ValueType::Float);
        assert_eq!(Value::string("hello").value_type, ValueType::String);
    }

    #[test]
    fn test_value_conversion() {
        let val = Value::integer(42);
        assert_eq!(val.as_i64().unwrap(), 42);
    }

    #[test]
    fn test_value_array() {
        let arr = Value::array(vec![
            Value::integer(1),
            Value::integer(2),
            Value::integer(3),
        ]);
        let values = arr.as_array().unwrap();
        assert_eq!(values.len(), 3);
    }
}
