#[derive(Debug, Clone)]
pub enum CloudProvider {
    AWS,
    GCP,
    Azure,
    Kubernetes,
}

pub struct CloudDeployment {
    pub provider: CloudProvider,
    pub region: String,
    pub replicas: u32,
}

impl CloudDeployment {
    pub fn new(provider: CloudProvider, region: String, replicas: u32) -> Self {
        Self {
            provider,
            region,
            replicas,
        }
    }

    pub fn deployment_config(&self) -> String {
        format!(
            "Provider: {:?}, Region: {}, Replicas: {}",
            self.provider, self.region, self.replicas
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cloud_deployment() {
        let deploy = CloudDeployment::new(CloudProvider::AWS, "us-east-1".to_string(), 3);
        assert_eq!(deploy.replicas, 3);
    }
}
