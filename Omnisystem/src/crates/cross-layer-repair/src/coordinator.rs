//! Cross-layer repair coordination

use crate::{
    CrossLayerError, CrossLayerRepairConfig, CrossLayerRepairStatistics, DependencyGraph,
    SystemLayer,
};
use anyhow::Result;

pub struct CrossLayerRepairCoordinator {
    config: CrossLayerRepairConfig,
    layer_status: std::collections::HashMap<SystemLayer, LayerStatus>,
}

#[derive(Clone, Debug)]
struct LayerStatus {
    layer: SystemLayer,
    health: LayerHealth,
    last_repair: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum LayerHealth {
    Healthy,
    Degraded,
    Critical,
}

impl CrossLayerRepairCoordinator {
    pub fn new(config: &CrossLayerRepairConfig) -> Result<Self> {
        let mut layer_status = std::collections::HashMap::new();

        // Initialize all layers
        layer_status.insert(
            SystemLayer::UOSC,
            LayerStatus {
                layer: SystemLayer::UOSC,
                health: LayerHealth::Healthy,
                last_repair: None,
            },
        );
        layer_status.insert(
            SystemLayer::Omnisystem,
            LayerStatus {
                layer: SystemLayer::Omnisystem,
                health: LayerHealth::Healthy,
                last_repair: None,
            },
        );
        layer_status.insert(
            SystemLayer::BonsaiEcosystem,
            LayerStatus {
                layer: SystemLayer::BonsaiEcosystem,
                health: LayerHealth::Healthy,
                last_repair: None,
            },
        );

        Ok(Self {
            config: config.clone(),
            layer_status,
        })
    }

    /// Identify which layers are affected by an error
    pub fn identify_affected_layers(&self, error: &CrossLayerError) -> Result<Vec<SystemLayer>> {
        let mut affected = vec![error.origin_layer.clone()];

        // Determine cascading effects
        match error.origin_layer {
            SystemLayer::UOSC => {
                // UOSC errors affect all layers
                affected.push(SystemLayer::Omnisystem);
                affected.push(SystemLayer::BonsaiEcosystem);
            }
            SystemLayer::Omnisystem => {
                // Omnisystem errors affect BonsaiEcosystem
                affected.push(SystemLayer::BonsaiEcosystem);
            }
            SystemLayer::BonsaiEcosystem => {
                // BonsaiEcosystem errors are isolated
            }
        }

        Ok(affected)
    }

    /// Build a dependency graph between layers
    pub async fn build_dependency_graph(&self, layers: &[SystemLayer]) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();

        // Add nodes for each layer
        for layer in layers {
            graph.add_layer(layer.clone())?;
        }

        // Add edges showing dependencies
        if layers.contains(&SystemLayer::UOSC) && layers.contains(&SystemLayer::Omnisystem) {
            graph.add_dependency(SystemLayer::Omnisystem, SystemLayer::UOSC)?;
        }

        if layers.contains(&SystemLayer::Omnisystem)
            && layers.contains(&SystemLayer::BonsaiEcosystem)
        {
            graph.add_dependency(SystemLayer::BonsaiEcosystem, SystemLayer::Omnisystem)?;
        }

        Ok(graph)
    }

    /// Update layer health status
    pub fn update_layer_status(&mut self, layer: SystemLayer, health: LayerHealth) {
        if let Some(status) = self.layer_status.get_mut(&layer) {
            status.health = health;
        }
    }

    /// Check if a layer is healthy
    pub fn is_layer_healthy(&self, layer: &SystemLayer) -> bool {
        self.layer_status
            .get(layer)
            .map(|s| s.health == LayerHealth::Healthy)
            .unwrap_or(false)
    }

    /// Get statistics about repairs
    pub async fn get_statistics(&self) -> Result<CrossLayerRepairStatistics> {
        Ok(CrossLayerRepairStatistics {
            total_cross_layer_repairs: 0,
            successful: 0,
            failed: 0,
            avg_cascade_depth: 0.0,
            most_common_origin_layer: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() -> Result<()> {
        let config = CrossLayerRepairConfig::default();
        let coordinator = CrossLayerRepairCoordinator::new(&config)?;
        assert!(coordinator.is_layer_healthy(&SystemLayer::UOSC));
        Ok(())
    }

    #[test]
    fn test_identify_affected_layers() -> Result<()> {
        let config = CrossLayerRepairConfig::default();
        let coordinator = CrossLayerRepairCoordinator::new(&config)?;

        let error = CrossLayerError {
            origin_layer: SystemLayer::UOSC,
            error_type: "kernel_panic".to_string(),
            message: "Test error".to_string(),
            affected_components: vec!["scheduler".to_string()],
        };

        let affected = coordinator.identify_affected_layers(&error)?;
        assert_eq!(affected.len(), 3); // All layers affected by UOSC error
        Ok(())
    }
}
