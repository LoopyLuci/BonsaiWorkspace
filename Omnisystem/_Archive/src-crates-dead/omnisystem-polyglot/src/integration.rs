/// Polyglot Integration Layer
/// Main orchestrator for 750+ language modules
/// Manages language chain execution and inter-module communication

use crate::framework::{ModuleRegistry, PolyglotModule, PolyglotRuntime};
use crate::messaging::MessageBus;
use dashmap::DashMap;
use std::sync::Arc;

/// Language Chain - execution order for all 750 languages
#[derive(Debug, Clone)]
pub struct LanguageChain {
    // Ordered list of language IDs
    chain: Vec<String>,
    // Mapping of language -> previous language
    previous: DashMap<String, Option<String>>,
    // Mapping of language -> next language
    next: DashMap<String, Option<String>>,
}

impl LanguageChain {
    pub fn new() -> Self {
        LanguageChain {
            chain: Vec::new(),
            previous: DashMap::new(),
            next: DashMap::new(),
        }
    }

    /// Add language to the chain (in order)
    pub fn add_language(&self, language_id: &str) {
        let position = self.chain.len();
        let chain_vec = unsafe {
            // This is safe because we're the only writer during initialization
            &mut *(self.chain.as_ptr() as *mut Vec<String>)
        };
        chain_vec.push(language_id.to_string());

        // Set previous link
        if position > 0 {
            let prev_id = &self.chain[position - 1];
            self.previous
                .insert(language_id.to_string(), Some(prev_id.clone()));
            self.next.insert(prev_id.clone(), Some(language_id.to_string()));
        } else {
            self.previous.insert(language_id.to_string(), None);
        }

        // Next will be set when next language is added
        if position == 0 {
            self.next.insert(language_id.to_string(), None);
        }
    }

    /// Get language chain in order
    pub fn get_chain(&self) -> Vec<String> {
        self.chain.clone()
    }

    /// Get previous language in chain
    pub fn get_previous(&self, language_id: &str) -> Option<String> {
        self.previous
            .get(language_id)
            .and_then(|p| p.clone())
    }

    /// Get next language in chain
    pub fn get_next(&self, language_id: &str) -> Option<String> {
        self.next.get(language_id).and_then(|n| n.clone())
    }

    /// Chain length
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }
}

impl Default for LanguageChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Main Polyglot Integration System
pub struct PolyglotIntegration {
    registry: Arc<ModuleRegistry>,
    runtime: Arc<PolyglotRuntime>,
    message_bus: Arc<MessageBus>,
    language_chain: Arc<LanguageChain>,
    execution_stats: Arc<DashMap<String, ExecutionStats>>,
}

/// Execution statistics for polyglot system
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub uptime_seconds: u64,
}

impl PolyglotIntegration {
    pub fn new() -> Self {
        let registry = Arc::new(ModuleRegistry::new());
        let runtime = Arc::new(PolyglotRuntime::new(registry.clone()));
        let message_bus = Arc::new(MessageBus::new());
        let language_chain = Arc::new(LanguageChain::new());

        PolyglotIntegration {
            registry,
            runtime,
            message_bus,
            language_chain,
            execution_stats: Arc::new(DashMap::new()),
        }
    }

    /// Register a module in the integration system
    pub async fn register_module(
        &self,
        module: Arc<dyn PolyglotModule>,
    ) -> anyhow::Result<()> {
        let lang_id = module.language_id();

        // Register with module registry
        self.registry.register(module.clone());

        // Register with message bus
        self.message_bus.register_language(lang_id);

        // Initialize module
        module.initialize().await?;

        tracing::info!("Registered polyglot module: {}", lang_id);

        Ok(())
    }

    /// Get module by language ID
    pub fn get_module(&self, language_id: &str) -> Option<Arc<dyn PolyglotModule>> {
        self.registry.get(language_id)
    }

    /// List all registered languages
    pub fn list_languages(&self) -> Vec<String> {
        self.registry.list_modules()
    }

    /// Total number of languages
    pub fn language_count(&self) -> usize {
        self.registry.module_count()
    }

    /// Get language chain
    pub fn language_chain(&self) -> &LanguageChain {
        &self.language_chain
    }

    /// Execute all modules in order
    pub async fn execute_all(&self) -> anyhow::Result<()> {
        let start = std::time::Instant::now();

        self.runtime.execute_chain().await?;

        let elapsed = start.elapsed().as_secs();

        // Update stats
        let mut stats = self.execution_stats
            .entry("__global__".to_string())
            .or_insert_with(|| ExecutionStats {
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
                total_messages_sent: 0,
                total_messages_received: 0,
                uptime_seconds: 0,
            });

        stats.total_executions += 1;
        stats.successful_executions += 1;
        stats.uptime_seconds += elapsed;

        tracing::info!(
            "Polyglot execution complete: {} languages in {}s",
            self.language_count(),
            elapsed
        );

        Ok(())
    }

    /// Execute single module
    pub async fn execute_module(&self, language_id: &str) -> anyhow::Result<()> {
        if let Some(module) = self.get_module(language_id) {
            let start = std::time::Instant::now();

            match module.execute().await {
                Ok(_) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    self.registry.update_stats(language_id, true, elapsed);

                    tracing::debug!("Module {} executed successfully", language_id);
                    Ok(())
                }
                Err(e) => {
                    self.registry.update_stats(language_id, false, 0);
                    Err(e)
                }
            }
        } else {
            Err(anyhow::anyhow!("Module {} not found", language_id))
        }
    }

    /// Send message between languages
    pub fn send_message(
        &self,
        from: &str,
        to: &str,
        message_type: &str,
        payload: serde_json::Value,
    ) -> anyhow::Result<String> {
        let msg_id = self.message_bus.send_message(from, to, message_type, payload)?;

        let mut stats = self.execution_stats
            .entry("__global__".to_string())
            .or_insert_with(|| ExecutionStats {
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
                total_messages_sent: 0,
                total_messages_received: 0,
                uptime_seconds: 0,
            });

        stats.total_messages_sent += 1;

        Ok(msg_id)
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> Option<ExecutionStats> {
        self.execution_stats.get("__global__").map(|s| s.clone())
    }

    /// Get message bus
    pub fn message_bus(&self) -> &MessageBus {
        &self.message_bus
    }

    /// Get module registry
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    /// Get polyglot runtime
    pub fn runtime(&self) -> &PolyglotRuntime {
        &self.runtime
    }

    /// Health check - verify all modules are healthy
    pub async fn health_check(&self) -> anyhow::Result<bool> {
        let languages = self.list_languages();

        for lang_id in languages {
            if let Some(module) = self.get_module(&lang_id) {
                if !module.health_check().await? {
                    tracing::warn!("Module {} health check failed", lang_id);
                    return Ok(false);
                }
            }
        }

        tracing::info!("Polyglot system health check: OK ({} modules)", self.language_count());
        Ok(true)
    }

    /// Shutdown polyglot system
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.message_bus.shutdown().await?;

        tracing::info!("Polyglot system shutdown complete");

        Ok(())
    }
}

impl Default for PolyglotIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_chain() {
        let chain = LanguageChain::new();
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_polyglot_integration() {
        let integration = PolyglotIntegration::new();
        assert_eq!(integration.language_count(), 0);
    }
}
