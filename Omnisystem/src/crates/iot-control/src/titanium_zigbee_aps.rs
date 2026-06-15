use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApsFrame {
    pub frame_control: u8,
    pub destination_endpoint: u8,
    pub cluster_id: u16,
    pub profile_id: u16,
    pub source_endpoint: u8,
    pub sequence: u8,
    pub security: bool,
    pub payload: Vec<u8>,
}

pub struct TitaniumAps {
    short_address: u16,
    endpoint_registry: HashMap<u8, EndpointInfo>,
    binding_table: Vec<Binding>,
    sequence_number: u8,
}

#[derive(Clone, Debug)]
pub struct EndpointInfo {
    pub endpoint: u8,
    pub profile_id: u16,
    pub device_id: u16,
    pub clusters: Vec<u16>,
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub source_endpoint: u8,
    pub cluster_id: u16,
    pub dest_address: u64,
    pub dest_endpoint: u8,
}

impl TitaniumAps {
    pub fn new(short_address: u16) -> Self {
        TitaniumAps {
            short_address,
            endpoint_registry: HashMap::new(),
            binding_table: Vec::new(),
            sequence_number: 0,
        }
    }

    pub fn register_endpoint(&mut self, info: EndpointInfo) {
        self.endpoint_registry.insert(info.endpoint, info);
    }

    pub fn add_binding(&mut self, binding: Binding) {
        self.binding_table.push(binding);
    }

    pub fn create_frame(
        &mut self,
        src_ep: u8,
        dest_ep: u8,
        cluster_id: u16,
        profile_id: u16,
        payload: Vec<u8>,
    ) -> ApsFrame {
        ApsFrame {
            frame_control: 0x00,
            destination_endpoint: dest_ep,
            cluster_id,
            profile_id,
            source_endpoint: src_ep,
            sequence: self.get_next_sequence(),
            security: false,
            payload,
        }
    }

    pub fn get_bindings_for_cluster(&self, cluster_id: u16) -> Vec<&Binding> {
        self.binding_table
            .iter()
            .filter(|b| b.cluster_id == cluster_id)
            .collect()
    }

    pub fn get_endpoint_info(&self, endpoint: u8) -> Option<&EndpointInfo> {
        self.endpoint_registry.get(&endpoint)
    }

    fn get_next_sequence(&mut self) -> u8 {
        let seq = self.sequence_number;
        self.sequence_number = self.sequence_number.wrapping_add(1);
        seq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aps_creation() {
        let aps = TitaniumAps::new(0x0001);
        assert!(aps.get_endpoint_info(1).is_none());
    }

    #[test]
    fn test_endpoint_registration() {
        let mut aps = TitaniumAps::new(0x0001);
        let ep = EndpointInfo {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0100,
            clusters: vec![0x0000, 0x0001],
        };

        aps.register_endpoint(ep.clone());
        assert!(aps.get_endpoint_info(1).is_some());
    }

    #[test]
    fn test_binding_table() {
        let mut aps = TitaniumAps::new(0x0001);
        let binding = Binding {
            source_endpoint: 1,
            cluster_id: 0x0006,
            dest_address: 0x0002,
            dest_endpoint: 1,
        };

        aps.add_binding(binding);
        assert_eq!(aps.binding_table.len(), 1);
    }

    #[test]
    fn test_get_bindings_for_cluster() {
        let mut aps = TitaniumAps::new(0x0001);
        aps.add_binding(Binding {
            source_endpoint: 1,
            cluster_id: 0x0006,
            dest_address: 0x0002,
            dest_endpoint: 1,
        });
        aps.add_binding(Binding {
            source_endpoint: 1,
            cluster_id: 0x0008,
            dest_address: 0x0003,
            dest_endpoint: 1,
        });

        let bindings = aps.get_bindings_for_cluster(0x0006);
        assert_eq!(bindings.len(), 1);
    }

    #[test]
    fn test_frame_creation() {
        let mut aps = TitaniumAps::new(0x0001);
        let frame = aps.create_frame(1, 1, 0x0006, 0x0104, vec![1, 0]);
        assert_eq!(frame.cluster_id, 0x0006);
        assert_eq!(frame.source_endpoint, 1);
    }
}
