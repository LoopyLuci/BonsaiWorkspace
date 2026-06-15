// Type Marshaling Module
// Converts between Sylva types and language-specific types

use omnisystem_sylva_core::module::SylvaModule;
use omnisystem_sylva_core::types::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type Marshaling Module - converts types between languages
pub struct TypeMarshalingModule {
    name: String,
    version: String,
    type_mappings: HashMap<String, LanguageTypeMapping>,
}

/// Mapping of Sylva types to language-specific types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageTypeMapping {
    pub language: String,
    pub mappings: HashMap<String, String>,
}

/// Type conversion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub success: bool,
    pub source_type: String,
    pub target_type: String,
    pub converted_value: Option<String>,
    pub error: Option<String>,
}

impl TypeMarshalingModule {
    pub fn new() -> Self {
        Self {
            name: "type-marshaling".to_string(),
            version: "1.0.0".to_string(),
            type_mappings: Self::build_default_mappings(),
        }
    }

    /// Build default type mappings for all languages
    fn build_default_mappings() -> HashMap<String, LanguageTypeMapping> {
        let mut mappings = HashMap::new();

        // Python type mapping
        let mut python_map = HashMap::new();
        python_map.insert("Bool".to_string(), "bool".to_string());
        python_map.insert("I32".to_string(), "int".to_string());
        python_map.insert("I64".to_string(), "int".to_string());
        python_map.insert("F64".to_string(), "float".to_string());
        python_map.insert("String".to_string(), "str".to_string());
        python_map.insert("Array".to_string(), "list".to_string());
        mappings.insert(
            "python".to_string(),
            LanguageTypeMapping {
                language: "python".to_string(),
                mappings: python_map,
            },
        );

        // Go type mapping
        let mut go_map = HashMap::new();
        go_map.insert("Bool".to_string(), "bool".to_string());
        go_map.insert("I32".to_string(), "int32".to_string());
        go_map.insert("I64".to_string(), "int64".to_string());
        go_map.insert("F64".to_string(), "float64".to_string());
        go_map.insert("String".to_string(), "string".to_string());
        go_map.insert("Array".to_string(), "[]interface{}".to_string());
        mappings.insert(
            "go".to_string(),
            LanguageTypeMapping {
                language: "go".to_string(),
                mappings: go_map,
            },
        );

        // JavaScript type mapping
        let mut js_map = HashMap::new();
        js_map.insert("Bool".to_string(), "boolean".to_string());
        js_map.insert("I32".to_string(), "number".to_string());
        js_map.insert("I64".to_string(), "BigInt".to_string());
        js_map.insert("F64".to_string(), "number".to_string());
        js_map.insert("String".to_string(), "string".to_string());
        js_map.insert("Array".to_string(), "Array".to_string());
        mappings.insert(
            "javascript".to_string(),
            LanguageTypeMapping {
                language: "javascript".to_string(),
                mappings: js_map,
            },
        );

        // Rust type mapping
        let mut rust_map = HashMap::new();
        rust_map.insert("Bool".to_string(), "bool".to_string());
        rust_map.insert("I32".to_string(), "i32".to_string());
        rust_map.insert("I64".to_string(), "i64".to_string());
        rust_map.insert("F64".to_string(), "f64".to_string());
        rust_map.insert("String".to_string(), "String".to_string());
        rust_map.insert("Array".to_string(), "Vec<T>".to_string());
        mappings.insert(
            "rust".to_string(),
            LanguageTypeMapping {
                language: "rust".to_string(),
                mappings: rust_map,
            },
        );

        // Java type mapping
        let mut java_map = HashMap::new();
        java_map.insert("Bool".to_string(), "boolean".to_string());
        java_map.insert("I32".to_string(), "int".to_string());
        java_map.insert("I64".to_string(), "long".to_string());
        java_map.insert("F64".to_string(), "double".to_string());
        java_map.insert("String".to_string(), "String".to_string());
        java_map.insert("Array".to_string(), "List".to_string());
        mappings.insert(
            "java".to_string(),
            LanguageTypeMapping {
                language: "java".to_string(),
                mappings: java_map,
            },
        );

        mappings
    }

    /// Marshal Sylva value to language-specific representation
    pub fn marshal(&self, language: &str, value: &Value) -> ConversionResult {
        let type_str = format!("{:?}", value.type_of());

        match self.type_mappings.get(language) {
            Some(mapping) => match mapping.mappings.get(&type_str) {
                Some(target_type) => ConversionResult {
                    success: true,
                    source_type: type_str.clone(),
                    target_type: target_type.clone(),
                    converted_value: Some(format!("{:?}", value)),
                    error: None,
                },
                None => ConversionResult {
                    success: false,
                    source_type: type_str.clone(),
                    target_type: "unknown".to_string(),
                    converted_value: None,
                    error: Some(format!(
                        "No mapping for type {} in language {}",
                        type_str, language
                    )),
                },
            },
            None => ConversionResult {
                success: false,
                source_type: type_str,
                target_type: "unknown".to_string(),
                converted_value: None,
                error: Some(format!("Language not supported: {}", language)),
            },
        }
    }

    /// Unmarshal language-specific value to Sylva value
    pub fn unmarshal(&self, language: &str, type_name: &str, data: &str) -> ConversionResult {
        match self.type_mappings.get(language) {
            Some(mapping) => {
                let sylva_type = mapping
                    .mappings
                    .iter()
                    .find(|(_, v)| v == &type_name)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                ConversionResult {
                    success: true,
                    source_type: type_name.to_string(),
                    target_type: sylva_type,
                    converted_value: Some(data.to_string()),
                    error: None,
                }
            }
            None => ConversionResult {
                success: false,
                source_type: type_name.to_string(),
                target_type: "unknown".to_string(),
                converted_value: None,
                error: Some(format!("Language not supported: {}", language)),
            },
        }
    }

    /// Get type mapping for a language
    pub fn get_mapping(&self, language: &str) -> Option<&LanguageTypeMapping> {
        self.type_mappings.get(language)
    }
}

impl Default for TypeMarshalingModule {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SylvaModule for TypeMarshalingModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn init(&mut self, _config: &omnisystem_sylva_core::module::SylvaModuleConfig) -> anyhow::Result<()> {
        tracing::info!("Initializing Type Marshaling module with {} language mappings",
                      self.type_mappings.len());
        Ok(())
    }

    async fn main(&self) -> anyhow::Result<()> {
        tracing::info!("Type Marshaling module running");
        Ok(())
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        tracing::info!("Shutting down Type Marshaling module");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marshal_bool() {
        let module = TypeMarshalingModule::new();
        let val = Value::Bool(true);
        let result = module.marshal("python", &val);

        assert!(result.success);
        assert_eq!(result.target_type, "bool");
    }

    #[test]
    fn test_marshal_string() {
        let module = TypeMarshalingModule::new();
        let val = Value::String("hello".to_string());
        let result = module.marshal("go", &val);

        assert!(result.success);
        assert_eq!(result.target_type, "string");
    }

    #[test]
    fn test_unsupported_language() {
        let module = TypeMarshalingModule::new();
        let val = Value::Bool(true);
        let result = module.marshal("unknown_lang", &val);

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_unmarshal() {
        let module = TypeMarshalingModule::new();
        let result = module.unmarshal("python", "bool", "true");

        assert!(result.success);
    }
}
