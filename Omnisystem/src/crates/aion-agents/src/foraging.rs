use crate::Result;

pub struct ForagingBehavior {
    energy: f32,
    exploration_range: f32,
}

impl ForagingBehavior {
    pub fn new(initial_energy: f32) -> Self {
        Self {
            energy: initial_energy,
            exploration_range: 10.0,
        }
    }

    pub fn explore(&mut self) -> Result<Vec<f32>> {
        if self.energy <= 0.0 {
            return Err(crate::AgentError::AgentError("No energy".to_string()));
        }
        
        self.energy -= 1.0;
        Ok(vec![0.1, 0.2, 0.3])
    }

    pub fn return_to_nest(&mut self) -> Result<()> {
        self.energy -= 0.5;
        tracing::info!("Returning to nest");
        Ok(())
    }

    pub fn get_energy(&self) -> f32 {
        self.energy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foraging() {
        let mut behavior = ForagingBehavior::new(10.0);
        assert!(behavior.explore().is_ok());
        assert!(behavior.get_energy() < 10.0);
    }
}
