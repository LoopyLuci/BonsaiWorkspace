use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ZclDataType {
    Boolean,
    UnsignedInt8,
    SignedInt8,
    UnsignedInt16,
    SignedInt16,
    UnsignedInt32,
    SignedInt32,
    Float32,
    String,
    OctetString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZclAttribute {
    pub id: u16,
    pub name: String,
    pub data_type: ZclDataType,
    pub value: serde_json::Value,
    pub readable: bool,
    pub writable: bool,
    pub reportable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZclCluster {
    pub cluster_id: u16,
    pub name: String,
    pub attributes: HashMap<u16, ZclAttribute>,
    pub server_side: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZclCommand {
    pub cluster_id: u16,
    pub command_id: u8,
    pub name: String,
    pub payload: Vec<u8>,
}

pub struct TitaniumZcl {
    clusters: HashMap<u16, ZclCluster>,
    reporting_config: HashMap<u16, ReportingConfig>,
}

#[derive(Clone, Debug)]
pub struct ReportingConfig {
    pub cluster_id: u16,
    pub attribute_id: u16,
    pub min_interval: u16,
    pub max_interval: u16,
    pub reportable_change: f32,
}

impl TitaniumZcl {
    pub fn new() -> Self {
        TitaniumZcl {
            clusters: HashMap::new(),
            reporting_config: HashMap::new(),
        }
    }

    pub fn register_cluster(&mut self, cluster: ZclCluster) {
        self.clusters.insert(cluster.cluster_id, cluster);
    }

    pub fn get_cluster(&self, cluster_id: u16) -> Option<&ZclCluster> {
        self.clusters.get(&cluster_id)
    }

    pub fn get_attribute(&self, cluster_id: u16, attr_id: u16) -> Option<&ZclAttribute> {
        self.clusters
            .get(&cluster_id)
            .and_then(|c| c.attributes.get(&attr_id))
    }

    pub fn read_attribute(&self, cluster_id: u16, attr_id: u16) -> Option<serde_json::Value> {
        self.get_attribute(cluster_id, attr_id)
            .map(|a| a.value.clone())
    }

    pub fn write_attribute(
        &mut self,
        cluster_id: u16,
        attr_id: u16,
        value: serde_json::Value,
    ) -> std::result::Result<(), String> {
        if let Some(cluster) = self.clusters.get_mut(&cluster_id) {
            if let Some(attr) = cluster.attributes.get_mut(&attr_id) {
                if attr.writable {
                    attr.value = value;
                    Ok(())
                } else {
                    Err("Attribute is read-only".to_string())
                }
            } else {
                Err("Attribute not found".to_string())
            }
        } else {
            Err("Cluster not found".to_string())
        }
    }

    pub fn add_reporting_config(&mut self, config: ReportingConfig) {
        let key = (config.cluster_id as u32) << 16 | (config.attribute_id as u32);
        self.reporting_config.insert(key as u16, config);
    }

    pub fn get_reporting_config(&self, cluster_id: u16, attr_id: u16) -> Option<&ReportingConfig> {
        let key = (cluster_id as u32) << 16 | (attr_id as u32);
        self.reporting_config.get(&(key as u16))
    }

    pub fn create_on_off_cluster() -> ZclCluster {
        let mut attrs = HashMap::new();
        attrs.insert(
            0x0000,
            ZclAttribute {
                id: 0x0000,
                name: "OnOff".to_string(),
                data_type: ZclDataType::Boolean,
                value: serde_json::json!(false),
                readable: true,
                writable: true,
                reportable: true,
            },
        );

        ZclCluster {
            cluster_id: 0x0006,
            name: "On/Off".to_string(),
            attributes: attrs,
            server_side: true,
        }
    }

    pub fn create_level_control_cluster() -> ZclCluster {
        let mut attrs = HashMap::new();
        attrs.insert(
            0x0000,
            ZclAttribute {
                id: 0x0000,
                name: "CurrentLevel".to_string(),
                data_type: ZclDataType::UnsignedInt8,
                value: serde_json::json!(254),
                readable: true,
                writable: true,
                reportable: true,
            },
        );

        ZclCluster {
            cluster_id: 0x0008,
            name: "Level Control".to_string(),
            attributes: attrs,
            server_side: true,
        }
    }

    pub fn create_color_control_cluster() -> ZclCluster {
        let mut attrs = HashMap::new();
        attrs.insert(
            0x0003,
            ZclAttribute {
                id: 0x0003,
                name: "ColorX".to_string(),
                data_type: ZclDataType::UnsignedInt16,
                value: serde_json::json!(0x616B),
                readable: true,
                writable: true,
                reportable: true,
            },
        );

        ZclCluster {
            cluster_id: 0x0300,
            name: "Color Control".to_string(),
            attributes: attrs,
            server_side: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zcl_creation() {
        let zcl = TitaniumZcl::new();
        assert!(zcl.get_cluster(0x0006).is_none());
    }

    #[test]
    fn test_register_cluster() {
        let mut zcl = TitaniumZcl::new();
        let cluster = TitaniumZcl::create_on_off_cluster();
        zcl.register_cluster(cluster);
        assert!(zcl.get_cluster(0x0006).is_some());
    }

    #[test]
    fn test_read_attribute() {
        let mut zcl = TitaniumZcl::new();
        let cluster = TitaniumZcl::create_on_off_cluster();
        zcl.register_cluster(cluster);

        let value = zcl.read_attribute(0x0006, 0x0000);
        assert_eq!(value, Some(serde_json::json!(false)));
    }

    #[test]
    fn test_write_attribute() {
        let mut zcl = TitaniumZcl::new();
        let cluster = TitaniumZcl::create_on_off_cluster();
        zcl.register_cluster(cluster);

        assert!(zcl
            .write_attribute(0x0006, 0x0000, serde_json::json!(true))
            .is_ok());

        let value = zcl.read_attribute(0x0006, 0x0000);
        assert_eq!(value, Some(serde_json::json!(true)));
    }

    #[test]
    fn test_preset_clusters() {
        let on_off = TitaniumZcl::create_on_off_cluster();
        assert_eq!(on_off.cluster_id, 0x0006);

        let level = TitaniumZcl::create_level_control_cluster();
        assert_eq!(level.cluster_id, 0x0008);

        let color = TitaniumZcl::create_color_control_cluster();
        assert_eq!(color.cluster_id, 0x0300);
    }

    #[test]
    fn test_reporting_config() {
        let mut zcl = TitaniumZcl::new();
        let config = ReportingConfig {
            cluster_id: 0x0006,
            attribute_id: 0x0000,
            min_interval: 1,
            max_interval: 300,
            reportable_change: 0.0,
        };

        zcl.add_reporting_config(config);
        assert!(zcl.get_reporting_config(0x0006, 0x0000).is_some());
    }
}
