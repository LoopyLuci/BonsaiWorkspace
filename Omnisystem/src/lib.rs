// OMNISYSTEM - Complete implementation
// Core runtime and framework modules
// Version: 2.0

pub mod titan_runtime;
pub mod sylva_runtime;
pub mod omni_format;
pub mod omni_query_language;

pub use titan_runtime::{TitanRuntime, TitanValue, TitanType, RuntimeError};
pub use sylva_runtime::{Tensor, TensorError, Dense, Layer, activations, loss_functions};
pub use omni_format::{OmniDocument, OmniValue, OmniHeader, OmniError};
pub use omni_query_language::{OqlParser, QueryExecutor, QueryResult};

/// Omnisystem version info
pub const VERSION: &str = "2.0.0";
pub const BUILD_DATE: &str = env!("CARGO_PKG_VERSION");

/// Initialize Omnisystem runtime environment
pub fn init() -> TitanRuntime {
    let runtime = TitanRuntime::new();
    println!("Omnisystem v{} initialized", VERSION);
    runtime
}

/// Create a new OMNI document
pub fn create_omni_document() -> OmniDocument {
    OmniDocument::new()
}

/// Create a new Tensor
pub fn create_tensor(shape: Vec<usize>) -> Tensor {
    Tensor::new(shape)
}

/// Example integration usage
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_titanruntime_integration() {
        let runtime = TitanRuntime::new();

        // Set a variable
        runtime.set_global("name".to_string(), TitanValue::String("Omnisystem".to_string()));

        // Get the variable
        let value = runtime.get_global("name");
        assert!(value.is_some());
        assert_eq!(value.unwrap().type_name(), "string");
    }

    #[test]
    fn test_tensor_operations() {
        let tensor = Tensor::from_vec(
            vec![1.0, 2.0, 3.0, 4.0],
            vec![2, 2]
        ).unwrap();

        assert_eq!(tensor.sum(), 10.0);
        assert_eq!(tensor.mean(), 2.5);
    }

    #[test]
    fn test_omni_document_creation() {
        let mut doc = OmniDocument::new();
        doc.content = OmniValue::String("Test content".to_string());

        let serialized = doc.serialize().unwrap();
        let deserialized = OmniDocument::deserialize(&serialized).unwrap();

        assert_eq!(
            deserialized.content.as_string(),
            Some("Test content".to_string())
        );
    }

    #[test]
    fn test_oql_query() {
        let data = vec![
            OmniValue::Integer(1),
            OmniValue::Integer(2),
            OmniValue::Integer(3),
        ];

        let query = OqlParser::parse("SELECT * FROM data LIMIT 2").unwrap();
        let result = QueryExecutor::execute(&query, &data).unwrap();

        assert_eq!(result.count, 2);
    }

    #[test]
    fn test_multilanguage_interop() {
        // TITAN runtime
        let runtime = TitanRuntime::new();
        runtime.set_global("value".to_string(), TitanValue::F64(42.0));

        // Create tensor (SYLVA)
        let tensor = Tensor::zeros(vec![2, 3]);

        // OMNI document (universal format)
        let mut doc = OmniDocument::new();
        doc.content = OmniValue::Float(tensor.mean());

        assert!(doc.serialize().is_ok());
    }
}
