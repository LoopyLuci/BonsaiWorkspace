use crate::{Model, Result, TrainerError, ModelType};
use dashmap::DashMap;
use std::sync::Arc;

pub struct Trainer {
    models: Arc<DashMap<String, Model>>,
    training_history: Arc<std::sync::Mutex<Vec<TrainingRecord>>>,
}

#[derive(Debug, Clone)]
pub struct TrainingRecord {
    pub model_id: String,
    pub epoch: u32,
    pub loss: f32,
    pub accuracy: f32,
}

impl Trainer {
    pub fn new() -> Self {
        Self {
            models: Arc::new(DashMap::new()),
            training_history: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn create_model(&self, model: Model) -> Result<()> {
        self.models.insert(model.id.clone(), model);
        tracing::info!("Model created");
        Ok(())
    }

    pub fn train(&self, model_id: &str, epochs: u32) -> Result<()> {
        if let Some(mut model) = self.models.get_mut(model_id) {
            let mut history = self.training_history.lock().unwrap();
            
            for epoch in 0..epochs {
                let loss = 1.0 / ((epoch + 1) as f32);
                let accuracy = 1.0 - loss;
                
                history.push(TrainingRecord {
                    model_id: model_id.to_string(),
                    epoch,
                    loss,
                    accuracy,
                });
            }
            
            model.set_accuracy(1.0 - (1.0 / (epochs as f32)));
            model.mark_trained();
            tracing::info!("Training complete");
            Ok(())
        } else {
            Err(TrainerError::ModelError("Model not found".to_string()))
        }
    }

    pub fn get_model(&self, id: &str) -> Result<Model> {
        self.models
            .get(id)
            .map(|m| m.value().clone())
            .ok_or_else(|| TrainerError::ModelError("Model not found".to_string()))
    }

    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    pub fn history_length(&self) -> usize {
        self.training_history.lock().unwrap().len()
    }
}

impl Default for Trainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trainer() {
        let trainer = Trainer::new();
        let model = Model::new(
            "m1".to_string(),
            "Test Model".to_string(),
            ModelType::NeuralNetwork,
        );
        assert!(trainer.create_model(model).is_ok());
        assert!(trainer.train("m1", 10).is_ok());
        let trained = trainer.get_model("m1").unwrap();
        assert!(trained.trained);
    }
}
