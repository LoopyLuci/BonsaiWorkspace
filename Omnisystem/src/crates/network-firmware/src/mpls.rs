use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct LabelPath {
    pub label: u32,
    pub outgoing_label: u32,
    pub outgoing_interface: u16,
    pub ttl: u8,
}

pub struct MplsManager {
    label_map: Arc<DashMap<u32, LabelPath>>,
    next_label: Arc<std::sync::Mutex<u32>>,
}

impl MplsManager {
    pub fn new() -> Self {
        Self {
            label_map: Arc::new(DashMap::new()),
            next_label: Arc::new(std::sync::Mutex::new(100)),
        }
    }

    pub fn create_label(&self, outgoing_iface: u16) -> u32 {
        let mut next = self.next_label.lock().unwrap();
        let label = *next;
        *next += 1;

        let path = LabelPath {
            label,
            outgoing_label: label + 1,
            outgoing_interface: outgoing_iface,
            ttl: 255,
        };
        self.label_map.insert(label, path);
        label
    }

    pub fn get_path(&self, label: u32) -> Option<LabelPath> {
        self.label_map.get(&label).map(|p| p.clone())
    }

    pub fn push_label(&self, label: u32) -> bool {
        self.label_map.contains_key(&label)
    }

    pub fn pop_label(&self, label: u32) -> bool {
        self.label_map.remove(&label).is_some()
    }

    pub fn label_count(&self) -> usize {
        self.label_map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_creation() {
        let mpls = MplsManager::new();
        let label1 = mpls.create_label(1);
        let label2 = mpls.create_label(2);
        assert_ne!(label1, label2);
        assert_eq!(mpls.label_count(), 2);
    }

    #[test]
    fn test_label_path() {
        let mpls = MplsManager::new();
        let label = mpls.create_label(5);
        let path = mpls.get_path(label).unwrap();
        assert_eq!(path.outgoing_interface, 5);
    }
}
