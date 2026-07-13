//! UABI bridge for inter-language Clojure-Titan communication

use crate::capabilities::AccessControl;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Universal ABI term representation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Term {
    /// Nil/null value
    Nil,

    /// Boolean
    Bool(bool),

    /// Integer
    Int(i64),

    /// Float
    Float(f64),

    /// String
    String(String),

    /// List of terms
    List(Vec<Term>),

    /// Map/dictionary
    Map(HashMap<String, Term>),

    /// Opaque pointer (for Java objects)
    Pointer(u64),

    /// Error term
    Error(String),
}

impl Term {
    /// Convert to string representation
    pub fn to_string_repr(&self) -> String {
        match self {
            Term::Nil => "nil".to_string(),
            Term::Bool(b) => b.to_string(),
            Term::Int(i) => i.to_string(),
            Term::Float(f) => f.to_string(),
            Term::String(s) => format!("\"{}\"", s),
            Term::List(items) => {
                let items_str = items.iter()
                    .map(|t| t.to_string_repr())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("({})", items_str)
            }
            Term::Map(_) => "[map]".to_string(),
            Term::Pointer(p) => format!("@{}", p),
            Term::Error(e) => format!("(error \"{}\")", e),
        }
    }
}

/// JNI/UABI bridge for Clojure-Titan interop
pub struct UABIBridge {
    /// POSIX shim socket connection
    socket_path: String,

    /// Access control enforcer
    access_control: Arc<AccessControl>,

    /// Cached function signatures
    function_cache: Arc<RwLock<HashMap<String, FunctionSignature>>>,
}

/// Function signature for UABI calls
#[derive(Clone, Debug)]
pub struct FunctionSignature {
    /// Function name
    pub name: String,

    /// Parameter types
    pub params: Vec<String>,

    /// Return type
    pub return_type: String,
}

impl UABIBridge {
    /// Create a new UABI bridge
    pub fn new(socket_path: String, access_control: Arc<AccessControl>) -> Result<Self> {
        log::info!("Initializing UABI bridge with socket: {}", socket_path);

        Ok(Self {
            socket_path,
            access_control,
            function_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Call a Titan function from Clojure
    pub fn call_titan(&self, function_name: &str, args: Vec<Term>) -> Result<Term> {
        log::debug!("Calling Titan function: {} with {} args", function_name, args.len());

        // Check access control for this function call
        // (In real implementation, would verify capability)

        // In real implementation:
        // 1. Serialize args to UABI format
        // 2. Send to Titan via socket
        // 3. Wait for response
        // 4. Deserialize result

        // For now, return a dummy response
        Ok(Term::String(format!("Result of {}", function_name)))
    }

    /// Call a Sylva function from Clojure
    pub fn call_sylva(&self, function_name: &str, _args: Vec<Term>) -> Result<Term> {
        log::debug!("Calling Sylva function: {}", function_name);

        // Similar to Titan but routes to Sylva
        Ok(Term::String(format!("Sylva result for {}", function_name)))
    }

    /// Register a Clojure callback for Titan to call
    pub fn register_callback(&self, name: &str, sig: FunctionSignature) -> Result<()> {
        log::info!("Registering Clojure callback: {}", name);

        let mut cache = self.function_cache.write()
            .map_err(|e| Error::UABIBridgeError(format!("Lock error: {}", e)))?;
        cache.insert(name.to_string(), sig);

        Ok(())
    }

    /// Handle a callback from Titan
    pub fn handle_callback(&self, name: &str, _args: Vec<Term>) -> Result<Term> {
        log::debug!("Handling callback from Titan: {}", name);

        let cache = self.function_cache.read()
            .map_err(|e| Error::UABIBridgeError(format!("Lock error: {}", e)))?;
        if !cache.contains_key(name) {
            return Err(Error::UABIBridgeError(format!("Unknown callback: {}", name)));
        }

        // In real implementation:
        // 1. Look up callback function in Clojure
        // 2. Call it with args
        // 3. Return result

        Ok(Term::Nil)
    }

    /// Convert Clojure value to UABI term
    pub fn clojure_to_term(&self, value: &str) -> Result<Term> {
        // In real implementation, would parse Clojure syntax
        Ok(Term::String(value.to_string()))
    }

    /// Convert UABI term to Clojure value
    pub fn term_to_clojure(&self, term: &Term) -> Result<String> {
        Ok(term.to_string_repr())
    }

    /// Zero-copy data passing via shared heap
    pub fn share_memory(&self, _data: &[u8]) -> Result<u64> {
        // In real implementation:
        // 1. Allocate memory in shared heap
        // 2. Copy data
        // 3. Return pointer

        // For now, return a dummy pointer
        Ok(0xdeadbeef)
    }

    /// Get data from shared memory
    pub fn get_shared_memory(&self, _ptr: u64) -> Result<Vec<u8>> {
        // In real implementation:
        // 1. Retrieve data from shared memory at pointer
        // 2. Return as Vec<u8>

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_string_repr() {
        assert_eq!(Term::Nil.to_string_repr(), "nil");
        assert_eq!(Term::Bool(true).to_string_repr(), "true");
        assert_eq!(Term::Int(42).to_string_repr(), "42");
        assert_eq!(Term::String("hello".to_string()).to_string_repr(), "\"hello\"");
    }

    #[test]
    fn test_term_list_repr() {
        let list = Term::List(vec![
            Term::Int(1),
            Term::Int(2),
            Term::Int(3),
        ]);
        assert_eq!(list.to_string_repr(), "(1 2 3)");
    }

    #[test]
    fn test_uabi_bridge_creation() {
        let access_control = Arc::new(AccessControl::default());
        let bridge = UABIBridge::new(
            "/tmp/posix-shim.sock".to_string(),
            access_control,
        ).expect("Failed to create bridge");

        assert_eq!(bridge.socket_path, "/tmp/posix-shim.sock");
    }

    #[test]
    fn test_callback_registration() {
        let access_control = Arc::new(AccessControl::default());
        let bridge = UABIBridge::new(
            "/tmp/posix-shim.sock".to_string(),
            access_control,
        ).expect("Failed to create bridge");

        let sig = FunctionSignature {
            name: "my_callback".to_string(),
            params: vec!["int".to_string()],
            return_type: "string".to_string(),
        };

        bridge.register_callback("my_callback", sig).expect("Failed to register callback");

        // Verify it's registered
        let cache = bridge.function_cache.read().expect("Lock poisoned");
        assert!(cache.contains_key("my_callback"));
    }
}
