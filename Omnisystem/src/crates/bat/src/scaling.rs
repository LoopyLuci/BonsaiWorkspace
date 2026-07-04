use crate::config::BatConfig;

pub struct ScaleMap {
    pub depth: Vec<u32>,
    pub width: Vec<u32>,
    pub experts: Vec<u32>,
}

impl Default for ScaleMap {
    fn default() -> Self {
        Self {
            depth: vec![4, 12, 24, 48, 100],
            width: vec![256, 512, 1024, 2048],
            experts: vec![2, 8, 32, 128, 256],
        }
    }
}

impl ScaleMap {
    pub fn nearest(&self, target_params: u64) -> BatConfig {
        let depth = if target_params < 500_000_000 {
            self.depth[0]
        } else if target_params < 2_000_000_000 {
            self.depth[1]
        } else if target_params < 7_000_000_000 {
            self.depth[2]
        } else if target_params < 30_000_000_000 {
            self.depth[3]
        } else {
            self.depth[4]
        };

        let width = if target_params < 1_000_000_000 {
            self.width[0]
        } else if target_params < 7_000_000_000 {
            self.width[1]
        } else if target_params < 30_000_000_000 {
            self.width[2]
        } else {
            self.width[3]
        };

        let experts = if target_params < 1_000_000_000 {
            self.experts[0]
        } else if target_params < 7_000_000_000 {
            self.experts[1]
        } else if target_params < 30_000_000_000 {
            self.experts[2]
        } else if target_params < 70_000_000_000 {
            self.experts[3]
        } else {
            self.experts[4]
        };

        BatConfig {
            depth,
            width,
            num_experts: experts,
            ..BatConfig::default()
        }
    }
}
