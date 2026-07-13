use crate::Result;

pub struct Validator;

impl Validator {
    pub fn calculate_accuracy(predictions: &[f32], actual: &[f32]) -> Result<f32> {
        if predictions.len() != actual.len() {
            return Err(crate::TrainerError::ValidationError("Length mismatch".to_string()));
        }
        
        let correct = predictions
            .iter()
            .zip(actual.iter())
            .filter(|(p, a)| (p - a).abs() < 0.5)
            .count();
        
        Ok(correct as f32 / predictions.len() as f32)
    }

    pub fn calculate_loss(predictions: &[f32], actual: &[f32]) -> Result<f32> {
        if predictions.len() != actual.len() {
            return Err(crate::TrainerError::ValidationError("Length mismatch".to_string()));
        }
        
        let mse: f32 = predictions
            .iter()
            .zip(actual.iter())
            .map(|(p, a)| (p - a).powi(2))
            .sum::<f32>() / predictions.len() as f32;
        
        Ok(mse.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accuracy() {
        let pred = vec![1.0, 0.0, 1.0];
        let actual = vec![1.0, 0.0, 1.0];
        let acc = Validator::calculate_accuracy(&pred, &actual).unwrap();
        assert_eq!(acc, 1.0);
    }
}
