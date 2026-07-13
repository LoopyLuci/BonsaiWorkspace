use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct Dataset {
    name: String,
    samples: Arc<DashMap<String, Sample>>,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub id: String,
    pub features: Vec<f32>,
    pub label: Option<f32>,
}

impl Dataset {
    pub fn new(name: String) -> Self {
        Self {
            name,
            samples: Arc::new(DashMap::new()),
        }
    }

    pub fn add_sample(&self, sample: Sample) -> Result<()> {
        self.samples.insert(sample.id.clone(), sample);
        Ok(())
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn split(&self, train_ratio: f32) -> (Vec<Sample>, Vec<Sample>) {
        let total = self.samples.len();
        let train_count = (total as f32 * train_ratio) as usize;
        
        let mut train = Vec::new();
        let mut test = Vec::new();
        
        for (i, entry) in self.samples.iter().enumerate() {
            if i < train_count {
                train.push(entry.value().clone());
            } else {
                test.push(entry.value().clone());
            }
        }
        
        (train, test)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset() {
        let dataset = Dataset::new("test".to_string());
        let sample = Sample {
            id: "s1".to_string(),
            features: vec![1.0, 2.0, 3.0],
            label: Some(1.0),
        };
        assert!(dataset.add_sample(sample).is_ok());
        assert_eq!(dataset.sample_count(), 1);
    }

    #[test]
    fn test_split() {
        let dataset = Dataset::new("test".to_string());
        for i in 0..10 {
            let sample = Sample {
                id: format!("s{}", i),
                features: vec![1.0],
                label: Some(0.0),
            };
            dataset.add_sample(sample).unwrap();
        }
        let (train, test) = dataset.split(0.8);
        assert_eq!(train.len(), 8);
        assert_eq!(test.len(), 2);
    }
}
