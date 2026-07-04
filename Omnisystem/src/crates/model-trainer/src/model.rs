use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub model_type: ModelType,
    pub accuracy: f32,
    pub parameters: u64,
    pub trained: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    LinearRegression,
    LogisticRegression,
    NeuralNetwork,
    DecisionTree,
    RandomForest,
    SVM,
    KMeans,
    LSTM,
}

impl Model {
    pub fn new(id: String, name: String, model_type: ModelType) -> Self {
        Self {
            id,
            name,
            model_type,
            accuracy: 0.0,
            parameters: 0,
            trained: false,
        }
    }

    pub fn set_accuracy(&mut self, accuracy: f32) {
        self.accuracy = accuracy;
    }

    pub fn mark_trained(&mut self) {
        self.trained = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_creation() {
        let model = Model::new(
            "m1".to_string(),
            "Neural Net".to_string(),
            ModelType::NeuralNetwork,
        );
        assert_eq!(model.model_type, ModelType::NeuralNetwork);
    }
}
