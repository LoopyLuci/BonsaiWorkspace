// FFI Bridge Module
// Provides C FFI interface for calling Omnisystem from any language

use omnisystem_sylva_core::module::SylvaModule;
use omnisystem_sylva_core::types::{Type, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FFI Bridge Module - enables calling Omnisystem from any language via C
pub struct FFIBridgeModule {
    name: String,
    version: String,
    registered_functions: HashMap<String, FFIFunction>,
}

/// FFI Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFIFunction {
    pub name: String,
    pub module: String,
    pub operation: String,
    pub param_types: Vec<Type>,
    pub return_type: Type,
}

/// FFI Handle - opaque pointer to resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FFIHandle(u64);

impl FFIHandle {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// FFI Call - request from C to Omnisystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFICall {
    pub function: String,
    pub args: Vec<FFIValue>,
}

/// FFI Value - marshaled value for FFI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FFIValue {
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    F64(f64),
    String(String),
    Handle(FFIHandle),
    Array(Vec<FFIValue>),
}

impl FFIValue {
    pub fn from_value(val: &Value) -> Self {
        match val {
            Value::Null => FFIValue::Null,
            Value::Bool(b) => FFIValue::Bool(*b),
            Value::I64(i) => FFIValue::I64(*i),
            Value::U64(u) => FFIValue::U64(*u),
            Value::F64(f) => FFIValue::F64(*f),
            Value::String(s) => FFIValue::String(s.clone()),
            Value::Handle(h) => FFIValue::Handle(FFIHandle(h.parse().unwrap_or(0))),
            Value::Array(arr) => {
                FFIValue::Array(arr.iter().map(FFIValue::from_value).collect())
            }
            _ => FFIValue::Null,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            FFIValue::Null => Value::Null,
            FFIValue::Bool(b) => Value::Bool(*b),
            FFIValue::I32(i) => Value::I32(*i),
            FFIValue::I64(i) => Value::I64(*i),
            FFIValue::U32(u) => Value::U32(*u),
            FFIValue::U64(u) => Value::U64(*u),
            FFIValue::F64(f) => Value::F64(*f),
            FFIValue::String(s) => Value::String(s.clone()),
            FFIValue::Handle(h) => Value::Handle(h.0.to_string()),
            FFIValue::Array(arr) => {
                Value::Array(arr.iter().map(FFIValue::to_value).collect())
            }
        }
    }
}

/// FFI Response - result from Omnisystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFIResponse {
    pub success: bool,
    pub result: FFIValue,
    pub error: Option<String>,
}

impl FFIBridgeModule {
    pub fn new() -> Self {
        Self {
            name: "ffi-bridge".to_string(),
            version: "1.0.0".to_string(),
            registered_functions: HashMap::new(),
        }
    }

    /// Register an FFI function
    pub fn register_function(&mut self, func: FFIFunction) {
        self.registered_functions.insert(func.name.clone(), func);
    }

    /// Call an FFI function
    pub async fn call(&self, call: FFICall) -> FFIResponse {
        match self.registered_functions.get(&call.function) {
            Some(func) => {
                tracing::info!("FFI call: {} from module {}", func.name, func.module);

                // In real implementation, dispatch to actual function
                FFIResponse {
                    success: true,
                    result: FFIValue::Null,
                    error: None,
                }
            }
            None => FFIResponse {
                success: false,
                result: FFIValue::Null,
                error: Some(format!("Function not found: {}", call.function)),
            },
        }
    }

    /// List all registered functions
    pub fn list_functions(&self) -> Vec<&FFIFunction> {
        self.registered_functions.values().collect()
    }

    /// Get function by name
    pub fn get_function(&self, name: &str) -> Option<&FFIFunction> {
        self.registered_functions.get(name)
    }
}

impl Default for FFIBridgeModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SylvaModule for FFIBridgeModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn init(&mut self, _config: &omnisystem_sylva_core::module::SylvaModuleConfig) -> anyhow::Result<()> {
        tracing::info!("Initializing FFI Bridge module");

        // Register standard functions
        self.register_function(FFIFunction {
            name: "ping".to_string(),
            module: "core".to_string(),
            operation: "ping".to_string(),
            param_types: vec![],
            return_type: Type::Bool,
        });

        Ok(())
    }

    async fn main(&self) -> anyhow::Result<()> {
        tracing::info!("FFI Bridge module running");
        Ok(())
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        tracing::info!("Shutting down FFI Bridge module");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_handle() {
        let handle = FFIHandle::new(12345);
        assert_eq!(handle.as_u64(), 12345);
    }

    #[test]
    fn test_ffi_value_conversion() {
        let val = Value::Bool(true);
        let ffi_val = FFIValue::from_value(&val);
        assert_eq!(ffi_val, FFIValue::Bool(true));

        let back = ffi_val.to_value();
        assert_eq!(back, val);
    }

    #[test]
    fn test_register_function() {
        let mut module = FFIBridgeModule::new();
        let func = FFIFunction {
            name: "test".to_string(),
            module: "test".to_string(),
            operation: "test".to_string(),
            param_types: vec![Type::I64],
            return_type: Type::Bool,
        };

        module.register_function(func.clone());
        assert!(module.get_function("test").is_some());
    }

    #[tokio::test]
    async fn test_ffi_call() {
        let module = FFIBridgeModule::new();
        let call = FFICall {
            function: "nonexistent".to_string(),
            args: vec![],
        };

        let response = module.call(call).await;
        assert!(!response.success);
        assert!(response.error.is_some());
    }
}
