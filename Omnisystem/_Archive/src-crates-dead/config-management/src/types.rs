use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigValue {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub flag_id: String,
    pub enabled: bool,
    pub percentage: u32,
}
