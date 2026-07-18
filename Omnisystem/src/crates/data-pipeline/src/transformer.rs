use crate::Result;

pub struct Transformer;

impl Transformer {
    pub fn normalize(values: &[f32]) -> Result<Vec<f32>> {
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let range = max - min;
        
        if range == 0.0 {
            return Ok(vec![0.0; values.len()]);
        }
        
        Ok(values
            .iter()
            .map(|v| (v - min) / range)
            .collect())
    }

    pub fn deduplicate<T: PartialEq>(items: &[T]) -> Vec<T>
    where
        T: Clone,
    {
        let mut result = Vec::new();
        for item in items {
            if !result.contains(item) {
                result.push(item.clone());
            }
        }
        result
    }

    pub fn filter_nulls(items: &[Option<String>]) -> Vec<String> {
        items
            .iter()
            .filter_map(|item| item.as_ref().map(|s| s.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let normalized = Transformer::normalize(&values).unwrap();
        assert!(normalized[0] >= 0.0 && normalized[0] <= 1.0);
    }

    #[test]
    fn test_deduplicate() {
        let items = vec![1, 2, 2, 3, 3, 3];
        let result = Transformer::deduplicate(&items);
        assert_eq!(result.len(), 3);
    }
}
