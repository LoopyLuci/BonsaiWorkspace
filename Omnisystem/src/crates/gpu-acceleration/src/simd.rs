use crate::{SimdOperation, GpuError, GpuResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct SimdEngine {
    operations: Arc<DashMap<Uuid, SimdOperation>>,
}

impl SimdEngine {
    pub fn new() -> Self {
        Self {
            operations: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_operation(&self, vector_width: usize, element_count: usize) -> GpuResult<Uuid> {
        let operation_id = Uuid::new_v4();

        let operation = SimdOperation {
            operation_id,
            vector_width,
            element_count,
            operation_type: "vector_add".to_string(),
        };

        self.operations.insert(operation_id, operation);
        Ok(operation_id)
    }

    pub async fn add_vectors(&self, operation_id: Uuid, a: &[f32], b: &[f32]) -> GpuResult<Vec<f32>> {
        if !self.operations.contains_key(&operation_id) {
            return Err(GpuError::InvalidOperation);
        }

        if a.len() != b.len() {
            return Err(GpuError::InvalidOperation);
        }

        let result: Vec<f32> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
        Ok(result)
    }

    pub async fn multiply_vectors(&self, operation_id: Uuid, a: &[f32], b: &[f32]) -> GpuResult<Vec<f32>> {
        if !self.operations.contains_key(&operation_id) {
            return Err(GpuError::InvalidOperation);
        }

        if a.len() != b.len() {
            return Err(GpuError::InvalidOperation);
        }

        let result: Vec<f32> = a.iter().zip(b.iter()).map(|(x, y)| x * y).collect();
        Ok(result)
    }

    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

impl Default for SimdEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_operation() {
        let engine = SimdEngine::new();
        let op_id = engine.register_operation(256, 1024).await.unwrap();

        assert!(engine.operations.contains_key(&op_id));
        assert_eq!(engine.operation_count(), 1);
    }

    #[tokio::test]
    async fn test_add_vectors() {
        let engine = SimdEngine::new();
        let op_id = engine.register_operation(256, 8).await.unwrap();

        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0, 8.0];

        let result = engine.add_vectors(op_id, &a, &b).await.unwrap();
        assert_eq!(result, vec![6.0, 8.0, 10.0, 12.0]);
    }

    #[tokio::test]
    async fn test_multiply_vectors() {
        let engine = SimdEngine::new();
        let op_id = engine.register_operation(256, 4).await.unwrap();

        let a = vec![2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 2.0, 2.0, 2.0];

        let result = engine.multiply_vectors(op_id, &a, &b).await.unwrap();
        assert_eq!(result, vec![4.0, 6.0, 8.0, 10.0]);
    }
}
