use serde::{Deserialize, Serialize};

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub name: String,
    pub refresh_interval_secs: u64,
    pub panels: Vec<Panel>,
}

/// Dashboard panel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Panel {
    pub title: String,
    pub panel_type: PanelType,
    pub metric: String,
    pub position: Position,
}

/// Panel type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelType {
    Graph,
    Gauge,
    Stat,
    Table,
    Heatmap,
}

/// Panel position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl DashboardConfig {
    /// Create a new dashboard configuration
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            refresh_interval_secs: 10,
            panels: Vec::new(),
        }
    }

    /// Add a panel to dashboard
    pub fn add_panel(&mut self, panel: Panel) {
        self.panels.push(panel);
    }

    /// Create default ecosystem dashboard
    pub fn default_ecosystem() -> Self {
        let mut dashboard = DashboardConfig::new("Bonsai Ecosystem Overview");
        dashboard.refresh_interval_secs = 10;

        // Add panels
        dashboard.add_panel(Panel {
            title: "Request Latency (p50/p95/p99)".to_string(),
            panel_type: PanelType::Graph,
            metric: "request_duration_ms".to_string(),
            position: Position {
                x: 0,
                y: 0,
                width: 12,
                height: 8,
            },
        });

        dashboard.add_panel(Panel {
            title: "Error Rate".to_string(),
            panel_type: PanelType::Gauge,
            metric: "error_rate".to_string(),
            position: Position {
                x: 0,
                y: 8,
                width: 6,
                height: 8,
            },
        });

        dashboard.add_panel(Panel {
            title: "SLA Compliance".to_string(),
            panel_type: PanelType::Gauge,
            metric: "sla_compliance".to_string(),
            position: Position {
                x: 6,
                y: 8,
                width: 6,
                height: 8,
            },
        });

        dashboard.add_panel(Panel {
            title: "Throughput (req/sec)".to_string(),
            panel_type: PanelType::Stat,
            metric: "throughput".to_string(),
            position: Position {
                x: 12,
                y: 0,
                width: 6,
                height: 8,
            },
        });

        dashboard
    }

    /// Create per-system dashboard
    pub fn per_system(system: &str) -> Self {
        let mut dashboard = DashboardConfig::new(&format!("{} System Dashboard", system));

        dashboard.add_panel(Panel {
            title: format!("{} - Latency", system),
            panel_type: PanelType::Graph,
            metric: format!("system_latency{{system=\"{}\"}}", system),
            position: Position {
                x: 0,
                y: 0,
                width: 12,
                height: 8,
            },
        });

        dashboard.add_panel(Panel {
            title: format!("{} - Error Rate", system),
            panel_type: PanelType::Gauge,
            metric: format!("system_error_rate{{system=\"{}\"}}", system),
            position: Position {
                x: 12,
                y: 0,
                width: 6,
                height: 8,
            },
        });

        dashboard
    }

    /// Create performance dashboard
    pub fn performance() -> Self {
        let mut dashboard = DashboardConfig::new("Performance Analysis");

        dashboard.add_panel(Panel {
            title: "Hot Path Latencies".to_string(),
            panel_type: PanelType::Heatmap,
            metric: "hot_path_latencies".to_string(),
            position: Position {
                x: 0,
                y: 0,
                width: 12,
                height: 12,
            },
        });

        dashboard.add_panel(Panel {
            title: "Memory Allocations".to_string(),
            panel_type: PanelType::Graph,
            metric: "memory_allocations_mb".to_string(),
            position: Position {
                x: 12,
                y: 0,
                width: 6,
                height: 6,
            },
        });

        dashboard.add_panel(Panel {
            title: "GC Pauses".to_string(),
            panel_type: PanelType::Graph,
            metric: "gc_pause_ms".to_string(),
            position: Position {
                x: 12,
                y: 6,
                width: 6,
                height: 6,
            },
        });

        dashboard
    }

    /// Export dashboard config as JSON
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("JSON serialization failed: {}", e))
    }

    /// Export dashboard config as Grafana JSON
    pub fn to_grafana_json(&self) -> Result<String, String> {
        // Simplified Grafana export format
        let grafana_config = serde_json::json!({
            "dashboard": {
                "title": &self.name,
                "panels": self.panels.iter().map(|p| {
                    serde_json::json!({
                        "title": &p.title,
                        "type": format!("{:?}", p.panel_type).to_lowercase(),
                        "targets": [{
                            "expr": format!("{}[5m]", &p.metric)
                        }],
                        "gridPos": {
                            "x": p.position.x,
                            "y": p.position.y,
                            "w": p.position.width,
                            "h": p.position.height
                        }
                    })
                }).collect::<Vec<_>>(),
                "refresh": format!("{}s", self.refresh_interval_secs),
            }
        });

        serde_json::to_string_pretty(&grafana_config)
            .map_err(|e| format!("Grafana JSON serialization failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let dashboard = DashboardConfig::default_ecosystem();
        assert_eq!(dashboard.name, "Bonsai Ecosystem Overview");
        assert!(!dashboard.panels.is_empty());
    }

    #[test]
    fn test_dashboard_json_export() {
        let dashboard = DashboardConfig::new("Test");
        let json = dashboard.to_json();
        assert!(json.is_ok());
    }
}
