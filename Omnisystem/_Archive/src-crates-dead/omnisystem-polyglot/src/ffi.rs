/// OMNISYSTEM POLYGLOT: FOREIGN FUNCTION INTERFACE (FFI)
/// Enable seamless cross-language function calls and data marshaling
/// Supports all 750+ languages with unified calling convention

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// FFI Value: Universal data type that can be passed between languages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FFIValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<FFIValue>),
    Object(std::collections::HashMap<String, FFIValue>),
}

impl FFIValue {
    /// Convert FFI value to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Convert JSON to FFI value
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Convert FFI value to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Convert bytes to FFI value
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

/// FFI Function Signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFIFunctionSignature {
    /// Language ID that owns this function
    pub language_id: String,
    /// Function name
    pub function_name: String,
    /// Parameter types
    pub parameter_types: Vec<String>,
    /// Return type
    pub return_type: String,
}

/// FFI Function Binding - allows calling functions across languages
#[derive(Clone)]
pub struct FFIFunctionBinding {
    signature: FFIFunctionSignature,
    // Store function pointer as boxed closure for cross-language calls
    function: Arc<dyn Fn(Vec<FFIValue>) -> Result<FFIValue, String> + Send + Sync>,
}

impl FFIFunctionBinding {
    pub fn new<F>(
        language_id: String,
        function_name: String,
        parameter_types: Vec<String>,
        return_type: String,
        f: F,
    ) -> Self
    where
        F: Fn(Vec<FFIValue>) -> Result<FFIValue, String> + Send + Sync + 'static,
    {
        FFIFunctionBinding {
            signature: FFIFunctionSignature {
                language_id,
                function_name,
                parameter_types,
                return_type,
            },
            function: Arc::new(f),
        }
    }

    /// Call the function with arguments
    pub fn call(&self, args: Vec<FFIValue>) -> Result<FFIValue, String> {
        (self.function)(args)
    }

    pub fn signature(&self) -> &FFIFunctionSignature {
        &self.signature
    }
}

/// FFI Registry - manages all cross-language function bindings
pub struct FFIRegistry {
    functions: Arc<DashMap<String, FFIFunctionBinding>>,
    language_functions: Arc<DashMap<String, Vec<String>>>,
}

impl FFIRegistry {
    pub fn new() -> Self {
        FFIRegistry {
            functions: Arc::new(DashMap::new()),
            language_functions: Arc::new(DashMap::new()),
        }
    }

    /// Register a function for cross-language calling
    pub fn register_function(&self, binding: FFIFunctionBinding) {
        let lang_id = binding.signature.language_id.clone();
        let func_name = binding.signature.function_name.clone();
        let key = format!("{}::{}", lang_id, func_name);

        self.functions.insert(key, binding);

        // Track functions by language
        self.language_functions
            .entry(lang_id)
            .or_insert_with(Vec::new)
            .push(func_name);
    }

    /// Call a function from another language
    pub fn call_function(
        &self,
        language_id: &str,
        function_name: &str,
        args: Vec<FFIValue>,
    ) -> Result<FFIValue, String> {
        let key = format!("{}::{}", language_id, function_name);

        self.functions
            .get(&key)
            .map(|binding| binding.call(args))
            .ok_or_else(|| format!("Function not found: {}", key))?
    }

    /// Get all functions for a language
    pub fn get_language_functions(&self, language_id: &str) -> Vec<String> {
        self.language_functions
            .get(language_id)
            .map(|funcs| funcs.clone())
            .unwrap_or_default()
    }

    /// List all registered functions
    pub fn list_all_functions(&self) -> Vec<FFIFunctionSignature> {
        self.functions
            .iter()
            .map(|entry| entry.value().signature().clone())
            .collect()
    }
}

impl Default for FFIRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Data Marshaler - handles conversion between language-native types and FFI values
pub struct DataMarshaler;

impl DataMarshaler {
    /// Marshal Rust values to FFI
    pub fn marshal_rust_value(value: &impl Serialize) -> Result<FFIValue, String> {
        match serde_json::to_value(value) {
            Ok(json_value) => serde_json::from_value(json_value)
                .map_err(|e| format!("Marshaling error: {}", e)),
            Err(e) => Err(format!("Serialization error: {}", e)),
        }
    }

    /// Unmarshal FFI value to Rust type
    pub fn unmarshal_to_rust<T: for<'de> Deserialize<'de>>(
        value: &FFIValue,
    ) -> Result<T, String> {
        match serde_json::to_value(value) {
            Ok(json_value) => serde_json::from_value(json_value)
                .map_err(|e| format!("Unmarshaling error: {}", e)),
            Err(e) => Err(format!("Serialization error: {}", e)),
        }
    }

    /// Convert bytes to FFI value
    pub fn from_bytes(bytes: &[u8]) -> Result<FFIValue, String> {
        FFIValue::from_bytes(bytes).map_err(|e| e.to_string())
    }

    /// Convert FFI value to bytes
    pub fn to_bytes(value: &FFIValue) -> Result<Vec<u8>, String> {
        value.to_bytes().map_err(|e| e.to_string())
    }
}

/// Cross-Language Call Context
pub struct FFICallContext {
    pub from_language: String,
    pub to_language: String,
    pub function_name: String,
    pub arguments: Vec<FFIValue>,
}

/// FFI Statistics
#[derive(Debug, Clone)]
pub struct FFIStats {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub total_time_ms: u64,
    pub languages_integrated: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_value_conversions() {
        let value = FFIValue::String("hello".to_string());
        let json = value.to_json().unwrap();
        let recovered = FFIValue::from_json(&json).unwrap();
        assert_eq!(format!("{:?}", value), format!("{:?}", recovered));
    }

    #[test]
    fn test_ffi_registry() {
        let registry = FFIRegistry::new();

        // Register a simple function
        let binding = FFIFunctionBinding::new(
            "rust".to_string(),
            "add".to_string(),
            vec!["i64".to_string(), "i64".to_string()],
            "i64".to_string(),
            |args| {
                if args.len() != 2 {
                    return Err("Expected 2 arguments".to_string());
                }
                match (&args[0], &args[1]) {
                    (FFIValue::Integer(a), FFIValue::Integer(b)) => {
                        Ok(FFIValue::Integer(a + b))
                    }
                    _ => Err("Arguments must be integers".to_string()),
                }
            },
        );

        registry.register_function(binding);

        // Call the function
        let result = registry
            .call_function(
                "rust",
                "add",
                vec![FFIValue::Integer(5), FFIValue::Integer(3)],
            )
            .unwrap();

        assert_eq!(result, FFIValue::Integer(8));
    }

    #[test]
    fn test_ffi_data_marshaling() {
        let value = FFIValue::Array(vec![
            FFIValue::Integer(1),
            FFIValue::Integer(2),
            FFIValue::Integer(3),
        ]);

        let bytes = DataMarshaler::to_bytes(&value).unwrap();
        let recovered = DataMarshaler::from_bytes(&bytes).unwrap();

        assert_eq!(format!("{:?}", value), format!("{:?}", recovered));
    }
}
