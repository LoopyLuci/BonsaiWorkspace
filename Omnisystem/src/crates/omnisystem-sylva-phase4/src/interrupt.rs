// Interrupt Management - IRQ routing, MSI, exception handling

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Interrupt Handler - processes interrupts
pub type InterruptHandler = Box<dyn Fn(u32) -> anyhow::Result<()> + Send + Sync>;

/// Interrupt Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptInfo {
    pub irq_number: u32,
    pub cpu_affinity: u32,
    pub handler_count: u32,
    pub triggered_count: u64,
}

/// Interrupt Manager
pub struct InterruptManager {
    handlers: HashMap<u32, Vec<String>>,
    routing_table: HashMap<u32, u32>, // IRQ -> CPU
}

impl InterruptManager {
    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("Initializing Interrupt Manager");

        Ok(Self {
            handlers: HashMap::new(),
            routing_table: HashMap::new(),
        })
    }

    pub async fn register_handler(&mut self, irq: u32, name: String) -> anyhow::Result<()> {
        tracing::info!("Registering handler for IRQ {}: {}", irq, name);
        self.handlers.entry(irq).or_insert_with(Vec::new).push(name);
        Ok(())
    }

    pub async fn route_to_cpu(&mut self, irq: u32, cpu: u32) -> anyhow::Result<()> {
        tracing::info!("Routing IRQ {} to CPU {}", irq, cpu);
        self.routing_table.insert(irq, cpu);
        Ok(())
    }

    pub async fn get_routing(&self, irq: u32) -> anyhow::Result<u32> {
        Ok(*self.routing_table.get(&irq).unwrap_or(&0))
    }

    pub async fn enable_msi(&mut self, device_id: u32, vectors: u32) -> anyhow::Result<()> {
        tracing::info!("Enabling MSI for device {}: {} vectors", device_id, vectors);
        Ok(())
    }

    pub async fn list_interrupts(&self) -> anyhow::Result<Vec<InterruptInfo>> {
        let mut infos = Vec::new();
        for (irq, handlers) in &self.handlers {
            infos.push(InterruptInfo {
                irq_number: *irq,
                cpu_affinity: self.routing_table.get(irq).copied().unwrap_or(0),
                handler_count: handlers.len() as u32,
                triggered_count: 0,
            });
        }
        Ok(infos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interrupt_manager_creation() {
        let _manager = InterruptManager::new().await.unwrap();
    }

    #[tokio::test]
    async fn test_register_handler() {
        let mut manager = InterruptManager::new().await.unwrap();
        manager
            .register_handler(0, "timer_handler".to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_route_to_cpu() {
        let mut manager = InterruptManager::new().await.unwrap();
        manager.route_to_cpu(0, 0).await.unwrap();
        let cpu = manager.get_routing(0).await.unwrap();
        assert_eq!(cpu, 0);
    }
}
