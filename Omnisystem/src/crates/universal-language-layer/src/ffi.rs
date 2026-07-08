//! Foreign Function Interface (FFI) Module
//!
//! Enables calling functions across language boundaries with type safety.

use crate::error::{Result, UllError};
use crate::types::Value;
use crate::language::Language;
use std::collections::HashMap;
use uuid::Uuid;

/// Function signature for FFI calls
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub language: Language,
    pub parameters: Vec<Parameter>,
    pub return_type: String,
    pub is_async: bool,
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
}

/// FFI function handle
#[derive(Debug, Clone)]
pub struct FfiHandle {
    pub id: String,
    pub signature: FunctionSignature,
    pub pointer: *const libc::c_void,
}

unsafe impl Send for FfiHandle {}
unsafe impl Sync for FfiHandle {}

/// FFI call request
#[derive(Debug, Clone)]
pub struct FfiCall {
    pub call_id: String,
    pub function_id: String,
    pub arguments: HashMap<String, Value>,
    pub timeout_ms: u64,
}

impl FfiCall {
    /// Create new FFI call
    pub fn new(function_id: impl Into<String>, arguments: HashMap<String, Value>) -> Self {
        Self {
            call_id: Uuid::new_v4().to_string(),
            function_id: function_id.into(),
            arguments,
            timeout_ms: 30000, // 30s default timeout
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// FFI call result
#[derive(Debug, Clone)]
pub struct FfiResult {
    pub call_id: String,
    pub result: Value,
    pub duration_ms: u64,
}

/// FFI registry for function management
pub struct FfiRegistry {
    functions: HashMap<String, FfiHandle>,
}

impl FfiRegistry {
    /// Create new FFI registry
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    /// Register a function
    pub fn register(&mut self, signature: FunctionSignature, pointer: *const libc::c_void) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let handle = FfiHandle {
            id: id.clone(),
            signature,
            pointer,
        };
        self.functions.insert(id.clone(), handle);
        Ok(id)
    }

    /// Unregister a function
    pub fn unregister(&mut self, id: &str) -> Result<()> {
        self.functions.remove(id)
            .ok_or_else(|| UllError::FunctionNotFound(id.to_string()))?;
        Ok(())
    }

    /// Get function handle
    pub fn get(&self, id: &str) -> Result<FfiHandle> {
        self.functions
            .get(id)
            .cloned()
            .ok_or_else(|| UllError::FunctionNotFound(id.to_string()))
    }

    /// List all registered functions
    pub fn list(&self) -> Vec<FfiHandle> {
        self.functions.values().cloned().collect()
    }

    /// Find function by name and language
    pub fn find(&self, name: &str, language: Language) -> Option<FfiHandle> {
        self.functions
            .values()
            .find(|h| h.signature.name == name && h.signature.language == language)
            .cloned()
    }
}

impl Default for FfiRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// FFI call executor trait
pub trait FfiExecutor: Send + Sync {
    /// Execute an FFI call
    fn execute(&self, call: FfiCall) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FfiResult>> + Send + '_>>;

    /// Check function exists
    fn function_exists(&self, id: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send + '_>>;

    /// Get function signature
    fn get_signature(&self, id: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FunctionSignature>> + Send + '_>>;
}

/// Type conversion utilities
pub mod conversions {
    use super::*;

    /// Convert Rust value to FFI-safe representation
    pub fn rust_to_ffi(value: &Value) -> Result<*const libc::c_void> {
        // This is a simplified version - real implementation would handle all types
        match &value.data {
            serde_json::Value::Null => Ok(std::ptr::null()),
            serde_json::Value::Bool(b) => Ok(*b as *const libc::c_void),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i as *const libc::c_void)
                } else if let Some(f) = n.as_f64() {
                    Ok(f.to_bits() as *const libc::c_void)
                } else {
                    Err(UllError::ffi("Cannot convert number"))
                }
            }
            serde_json::Value::String(s) => {
                Ok(s.as_ptr() as *const libc::c_void)
            }
            _ => Err(UllError::ffi("Cannot convert complex type to FFI pointer")),
        }
    }

    /// Convert FFI pointer back to Rust value
    pub fn ffi_to_rust(ptr: *const libc::c_void, expected_type: &str) -> Result<Value> {
        if ptr.is_null() {
            return Ok(Value::null());
        }

        match expected_type {
            "bool" => {
                let val = ptr as i64 != 0;
                Ok(Value::boolean(val))
            }
            "i64" => {
                let val = ptr as i64;
                Ok(Value::integer(val))
            }
            "f64" => {
                let val = f64::from_bits(ptr as u64);
                Ok(Value::float(val))
            }
            "string" => {
                unsafe {
                    let s = std::ffi::CStr::from_ptr(ptr as *const i8)
                        .to_str()
                        .map_err(|_| UllError::ffi("Invalid UTF-8 in string"))?;
                    Ok(Value::string(s))
                }
            }
            _ => Err(UllError::type_conversion(
                format!("Unknown type: {}", expected_type)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_call_creation() {
        let mut args = HashMap::new();
        args.insert("x".to_string(), Value::integer(42));

        let call = FfiCall::new("my_func", args);
        assert!(!call.call_id.is_empty());
        assert_eq!(call.function_id, "my_func");
    }

    #[test]
    fn test_ffi_registry() {
        let mut registry = FfiRegistry::new();

        let sig = FunctionSignature {
            name: "test_func".to_string(),
            language: Language::Rust,
            parameters: vec![],
            return_type: "i64".to_string(),
            is_async: false,
        };

        let id = registry.register(sig.clone(), std::ptr::null()).unwrap();
        assert!(registry.get(&id).is_ok());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_ffi_conversions() {
        let val = Value::integer(42);
        let ptr = conversions::rust_to_ffi(&val).unwrap();
        let recovered = conversions::ffi_to_rust(ptr, "i64").unwrap();
        assert_eq!(recovered.as_i64().unwrap(), 42);
    }
}
