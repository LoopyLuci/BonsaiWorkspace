use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmnisystemConfig {
    pub name: String,
    pub version: String,
    pub debug: bool,
    pub max_modules: usize,
    pub timeout_ms: u64,
}

impl OmnisystemConfig {
    pub fn default_config() -> Self {
        Self {
            name: "omnisystem".to_string(),
            version: "1.0.0".to_string(),
            debug: false,
            max_modules: 1000,
            timeout_ms: 5000,
        }
    }
}

impl Default for OmnisystemConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_config() {
        let config = OmnisystemConfig::default();
        assert_eq!(config.name, "omnisystem");
    }
}
