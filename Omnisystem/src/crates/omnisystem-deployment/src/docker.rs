use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerConfig {
    pub image: String,
    pub version: String,
    pub ports: Vec<u16>,
    pub environment: Vec<String>,
}

impl DockerConfig {
    pub fn default_config() -> Self {
        Self {
            image: "omnisystem:latest".to_string(),
            version: "1.0.0".to_string(),
            ports: vec![8080, 9090],
            environment: vec!["LOG_LEVEL=info".to_string()],
        }
    }

    pub fn to_dockerfile(&self) -> String {
        format!(
            "FROM rust:latest\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\nEXPOSE {}\nCMD [\"./target/release/omnisystem\"]",
            self.ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(" ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_docker_config() {
        let config = DockerConfig::default_config();
        assert_eq!(config.image, "omnisystem:latest");
    }
}
