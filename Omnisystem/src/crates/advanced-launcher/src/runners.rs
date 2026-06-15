pub trait Runner: Send + Sync {
    fn supports(&self, app_type: &str) -> bool;
}

pub struct StandardRunner;

impl Runner for StandardRunner {
    fn supports(&self, _app_type: &str) -> bool {
        true
    }
}

pub struct DockerRunner;

impl Runner for DockerRunner {
    fn supports(&self, _app_type: &str) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runners() {
        let standard = StandardRunner;
        assert!(standard.supports("elf"));
        
        let docker = DockerRunner;
        assert!(docker.supports("docker"));
    }
}
