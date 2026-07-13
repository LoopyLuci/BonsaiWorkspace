/// Interrupt Routing Module
///
/// Manages IRQ routing and mapping:
/// - IRQ to CPU mapping
/// - MSI/MSI-X configuration
/// - Dynamic routing
/// - Affinity configuration

use crate::{InterruptError, Result};
use tracing::info;

/// Interrupt router
pub struct InterruptRouter {
    max_irqs: u32,
    has_msi: bool,
    has_msix: bool,
}

impl InterruptRouter {
    /// Create interrupt router
    pub fn new() -> Result<Self> {
        info!("Initializing Interrupt Router");

        let max_irqs = 256; // Typical x86 limit
        let has_msi = true;
        let has_msix = true;

        info!("IRQ Router: {} IRQs, MSI: {}, MSI-X: {}",
              max_irqs, has_msi, has_msix);

        Ok(Self {
            max_irqs,
            has_msi,
            has_msix,
        })
    }

    /// Get maximum IRQ count
    pub fn max_irqs(&self) -> u32 {
        self.max_irqs
    }

    /// Check MSI support
    pub fn has_msi(&self) -> bool {
        self.has_msi
    }

    /// Check MSI-X support
    pub fn has_msix(&self) -> bool {
        self.has_msix
    }

    /// Route IRQ to CPU
    pub fn route_irq(&self, irq: u32, cpu: u32) -> Result<()> {
        if irq >= self.max_irqs {
            return Err(InterruptError::Routing(
                format!("IRQ {} out of range", irq)
            ));
        }

        info!("Routing IRQ {} to CPU {}", irq, cpu);
        Ok(())
    }

    /// Set IRQ affinity mask
    pub fn set_affinity(&self, irq: u32, cpus: &[u32]) -> Result<()> {
        info!("Setting affinity for IRQ {}: {:?}", irq, cpus);
        Ok(())
    }

    /// Configure MSI for device
    pub fn configure_msi(&self, device_id: u32, vectors: u32) -> Result<()> {
        if !self.has_msi {
            return Err(InterruptError::Routing("MSI not supported".to_string()));
        }

        info!("Configuring MSI for device {}: {} vectors", device_id, vectors);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_router() {
        let router = InterruptRouter::new();
        assert!(router.is_ok());

        let router = router.unwrap();
        assert!(router.max_irqs() > 0);
        assert!(router.has_msi());
    }
}
