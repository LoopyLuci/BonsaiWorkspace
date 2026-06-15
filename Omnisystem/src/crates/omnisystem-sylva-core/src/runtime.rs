// Sylva Runtime - executes canonical Omnisystem modules

use omnisystem_ums::{ModuleRuntime, ModuleRegistry, ModuleResolver};
use std::sync::Arc;
use tracing;

/// Sylva Runtime - canonical implementation executor
pub struct SylvaRuntime {
    /// Module runtime from UMS
    pub module_runtime: Option<Arc<ModuleRuntime>>,

    /// Execution context
    pub context: SylvaContext,

    /// Builtin functions available to modules
    pub builtins: BuiltinRegistry,
}

/// Execution context
#[derive(Debug, Clone)]
pub struct SylvaContext {
    pub execution_id: String,
    pub phase: u32,
    pub max_memory_mb: u64,
    pub timeout_ms: u64,
}

impl Default for SylvaContext {
    fn default() -> Self {
        Self {
            execution_id: uuid::Uuid::new_v4().to_string(),
            phase: 1,
            max_memory_mb: 1024,
            timeout_ms: 30000,
        }
    }
}

/// Registry of builtin functions
pub struct BuiltinRegistry {
    builtins: std::collections::HashMap<String, Box<dyn Builtin>>,
}

/// Trait for builtin functions
pub trait Builtin: Send + Sync {
    fn name(&self) -> &str;
    fn call(&self, args: Vec<serde_json::Value>) -> anyhow::Result<serde_json::Value>;
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut builtins = std::collections::HashMap::new();

        // Register common builtins
        builtins.insert(
            "print".to_string(),
            Box::new(PrintBuiltin) as Box<dyn Builtin>,
        );
        builtins.insert(
            "panic".to_string(),
            Box::new(PanicBuiltin) as Box<dyn Builtin>,
        );
        builtins.insert(
            "assert".to_string(),
            Box::new(AssertBuiltin) as Box<dyn Builtin>,
        );

        Self { builtins }
    }

    pub fn register(&mut self, builtin: Box<dyn Builtin>) {
        self.builtins.insert(builtin.name().to_string(), builtin);
    }

    pub fn call(&self, name: &str, args: Vec<serde_json::Value>) -> anyhow::Result<serde_json::Value> {
        self.builtins
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown builtin: {}", name))?
            .call(args)
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Builtin implementations
struct PrintBuiltin;
impl Builtin for PrintBuiltin {
    fn name(&self) -> &str {
        "print"
    }

    fn call(&self, args: Vec<serde_json::Value>) -> anyhow::Result<serde_json::Value> {
        for arg in args {
            println!("{}", arg);
        }
        Ok(serde_json::json!(null))
    }
}

struct PanicBuiltin;
impl Builtin for PanicBuiltin {
    fn name(&self) -> &str {
        "panic"
    }

    fn call(&self, args: Vec<serde_json::Value>) -> anyhow::Result<serde_json::Value> {
        let msg = args
            .first()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "panic called".to_string());
        Err(anyhow::anyhow!("{}", msg))
    }
}

struct AssertBuiltin;
impl Builtin for AssertBuiltin {
    fn name(&self) -> &str {
        "assert"
    }

    fn call(&self, args: Vec<serde_json::Value>) -> anyhow::Result<serde_json::Value> {
        if args.is_empty() {
            return Err(anyhow::anyhow!("assert requires condition argument"));
        }

        let condition = args[0].as_bool().unwrap_or(false);
        if !condition {
            let msg = args
                .get(1)
                .map(|v| v.as_str().unwrap_or("assertion failed"))
                .unwrap_or("assertion failed");
            return Err(anyhow::anyhow!("{}", msg));
        }

        Ok(serde_json::json!(true))
    }
}

impl SylvaRuntime {
    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("Creating Sylva runtime");

        Ok(Self {
            module_runtime: None,
            context: SylvaContext::default(),
            builtins: BuiltinRegistry::new(),
        })
    }

    pub async fn with_ums(mut self, runtime: Arc<ModuleRuntime>) -> anyhow::Result<Self> {
        self.module_runtime = Some(runtime);
        Ok(self)
    }

    pub fn context(&self) -> &SylvaContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut SylvaContext {
        &mut self.context
    }

    pub fn builtins(&self) -> &BuiltinRegistry {
        &self.builtins
    }

    pub fn builtins_mut(&mut self) -> &mut BuiltinRegistry {
        &mut self.builtins
    }

    /// Execute a builtin function
    pub fn execute_builtin(&self, name: &str, args: Vec<serde_json::Value>) -> anyhow::Result<serde_json::Value> {
        self.builtins.call(name, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = SylvaRuntime::new().await.unwrap();
        assert_eq!(runtime.context.phase, 1);
    }

    #[test]
    fn test_builtin_print() {
        let runtime = SylvaRuntime::new().await.ok().unwrap();
        let result = runtime
            .execute_builtin("print", vec![serde_json::json!("hello")])
            .unwrap();
        assert_eq!(result, serde_json::json!(null));
    }

    #[test]
    fn test_builtin_assert() {
        let runtime = SylvaRuntime::new().await.ok().unwrap();
        let result = runtime
            .execute_builtin("assert", vec![serde_json::json!(true)])
            .unwrap();
        assert_eq!(result, serde_json::json!(true));

        let result = runtime.execute_builtin("assert", vec![serde_json::json!(false)]);
        assert!(result.is_err());
    }
}
