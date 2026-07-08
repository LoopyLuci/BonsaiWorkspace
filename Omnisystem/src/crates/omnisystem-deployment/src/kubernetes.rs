use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesManifest {
    pub api_version: String,
    pub kind: String,
    pub metadata: KubernetesMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KubernetesMetadata {
    pub name: String,
    pub namespace: String,
}

impl KubernetesManifest {
    pub fn new(name: String) -> Self {
        Self {
            api_version: "v1".to_string(),
            kind: "Deployment".to_string(),
            metadata: KubernetesMetadata {
                name,
                namespace: "default".to_string(),
            },
        }
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_k8s_manifest() {
        let manifest = KubernetesManifest::new("omnisystem".to_string());
        assert_eq!(manifest.kind, "Deployment");
    }
}
