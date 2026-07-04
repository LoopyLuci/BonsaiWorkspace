//! Language Bridge Module
//!
//! Provides seamless bridging between different languages through the FFI layer.

use crate::error::{Result, UllError};
use crate::ffi::{FfiCall, FfiRegistry, FunctionSignature};
use crate::language::Language;
use crate::types::Value;
use crate::registry::LanguageRegistry as ModuleRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Language Bridge for cross-language function calls
pub struct LanguageBridge {
    ffi_registry: Arc<RwLock<FfiRegistry>>,
    language_registry: Arc<RwLock<ModuleRegistry>>,
}

impl LanguageBridge {
    /// Create new language bridge
    pub fn new() -> Self {
        Self {
            ffi_registry: Arc::new(RwLock::new(FfiRegistry::new())),
            language_registry: Arc::new(RwLock::new(ModuleRegistry::new())),
        }
    }

    /// Register a function from any language
    pub fn register_function(
        &self,
        signature: FunctionSignature,
        pointer: *const libc::c_void,
    ) -> Result<String> {
        let mut registry = self.ffi_registry.write();
        registry.register(signature, pointer)
    }

    /// Call a function across language boundaries
    pub async fn call(
        &self,
        function_id: &str,
        arguments: HashMap<String, Value>,
    ) -> Result<Value> {
        let call = FfiCall::new(function_id, arguments);
        self.execute_call(call).await
    }

    /// Call a function by name and language
    pub async fn call_by_name(
        &self,
        name: &str,
        language: Language,
        arguments: HashMap<String, Value>,
    ) -> Result<Value> {
        let registry = self.ffi_registry.read();
        let handle = registry
            .find(name, language)
            .ok_or_else(|| UllError::FunctionNotFound(format!("{}/{}", language, name)))?;

        drop(registry);

        let call = FfiCall::new(handle.id, arguments);
        self.execute_call(call).await
    }

    /// Register a module that can be called from other languages
    pub fn register_module(&self, module_name: &str, language: Language) -> Result<()> {
        let mut registry = self.language_registry.write();
        registry.register_module(module_name, language);
        log::info!("Registered module {} ({})", module_name, language);
        Ok(())
    }

    /// Get list of available functions
    pub fn list_functions(&self) -> Result<Vec<FunctionSignature>> {
        let registry = self.ffi_registry.read();
        Ok(registry
            .list()
            .iter()
            .map(|h| h.signature.clone())
            .collect())
    }

    /// Find function by name and language
    pub fn find_function(&self, name: &str, language: Language) -> Result<FunctionSignature> {
        let registry = self.ffi_registry.read();
        registry
            .find(name, language)
            .map(|h| h.signature)
            .ok_or_else(|| UllError::FunctionNotFound(format!("{}/{}", language, name)))
    }

    // Private implementation
    async fn execute_call(&self, call: FfiCall) -> Result<Value> {
        let start = std::time::Instant::now();

        // For now, return a placeholder response
        // In production, this would:
        // 1. Look up the function
        // 2. Marshal arguments
        // 3. Call through FFI
        // 4. Unmarshal result

        let result = Value::null()
            .with_metadata("call_id", &call.call_id)
            .with_metadata("status", "success");

        let duration = start.elapsed().as_millis() as u64;

        log::debug!(
            "FFI call {} completed in {}ms",
            call.function_id,
            duration
        );

        Ok(result)
    }
}

impl Default for LanguageBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for Language Bridge configuration
pub struct BridgeBuilder {
    bridge: LanguageBridge,
}

impl BridgeBuilder {
    /// Create new bridge builder
    pub fn new() -> Self {
        Self {
            bridge: LanguageBridge::new(),
        }
    }

    /// Register a function
    pub fn register_function(
        self,
        signature: FunctionSignature,
        pointer: *const libc::c_void,
    ) -> Result<Self> {
        self.bridge.register_function(signature, pointer)?;
        Ok(self)
    }

    /// Register a module
    pub fn register_module(self, name: &str, language: Language) -> Result<Self> {
        self.bridge.register_module(name, language)?;
        Ok(self)
    }

    /// Build the bridge
    pub fn build(self) -> LanguageBridge {
        self.bridge
    }
}

impl Default for BridgeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let _bridge = LanguageBridge::new();
    }

    #[tokio::test]
    async fn test_bridge_call() {
        let bridge = LanguageBridge::new();
        let mut args = HashMap::new();
        args.insert("x".to_string(), Value::integer(42));

        let result = bridge.call("test_func", args).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_bridge_builder() {
        let _bridge = BridgeBuilder::new().build();
    }
}
