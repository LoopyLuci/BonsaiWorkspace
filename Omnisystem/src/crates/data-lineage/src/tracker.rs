use crate::{DataLineage, Dependency, Provenance, Transformation, LineageError, LineageResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct LineageTracker {
    lineages: Arc<DashMap<Uuid, DataLineage>>,
    dependencies: Arc<DashMap<Uuid, Dependency>>,
    provenances: Arc<DashMap<Uuid, Provenance>>,
    transformations: Arc<DashMap<Uuid, Transformation>>,
}

impl LineageTracker {
    pub fn new() -> Self {
        Self {
            lineages: Arc::new(DashMap::new()),
            dependencies: Arc::new(DashMap::new()),
            provenances: Arc::new(DashMap::new()),
            transformations: Arc::new(DashMap::new()),
        }
    }

    pub async fn track_lineage(&self, lineage: &DataLineage) -> LineageResult<()> {
        self.lineages.insert(lineage.lineage_id, lineage.clone());
        Ok(())
    }

    pub async fn track_dependency(&self, dep: &Dependency) -> LineageResult<()> {
        self.dependencies.insert(dep.dep_id, dep.clone());
        Ok(())
    }

    pub async fn record_provenance(&self, prov: &Provenance) -> LineageResult<()> {
        self.provenances.insert(prov.prov_id, prov.clone());
        Ok(())
    }

    pub async fn track_transformation(&self, transform: &Transformation) -> LineageResult<()> {
        self.transformations.insert(transform.transform_id, transform.clone());
        Ok(())
    }

    pub async fn get_lineage(&self, dataset_id: Uuid) -> LineageResult<Vec<DataLineage>> {
        let mut result = Vec::new();

        for entry in self.lineages.iter() {
            if entry.value().source_dataset_id == dataset_id || entry.value().target_dataset_id == dataset_id {
                result.push(entry.value().clone());
            }
        }

        Ok(result)
    }

    pub fn lineage_count(&self) -> usize {
        self.lineages.len()
    }
}

impl Default for LineageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_track_lineage() {
        let tracker = LineageTracker::new();
        let lineage = DataLineage {
            lineage_id: Uuid::new_v4(),
            source_dataset_id: Uuid::new_v4(),
            target_dataset_id: Uuid::new_v4(),
            transformation: "map".to_string(),
        };

        tracker.track_lineage(&lineage).await.unwrap();
        assert_eq!(tracker.lineage_count(), 1);
    }

    #[tokio::test]
    async fn test_track_dependency() {
        let tracker = LineageTracker::new();
        let dep = Dependency {
            dep_id: Uuid::new_v4(),
            source_dataset: "raw_events".to_string(),
            target_dataset: "processed_events".to_string(),
            dep_type: "transformation".to_string(),
        };

        tracker.track_dependency(&dep).await.unwrap();
    }

    #[tokio::test]
    async fn test_record_provenance() {
        let tracker = LineageTracker::new();
        let prov = Provenance {
            prov_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            origin: "kafka_topic".to_string(),
            transformations: vec!["normalize".to_string(), "aggregate".to_string()],
        };

        tracker.record_provenance(&prov).await.unwrap();
    }

    #[tokio::test]
    async fn test_track_transformation() {
        let tracker = LineageTracker::new();
        let transform = Transformation {
            transform_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            transform_type: "filter".to_string(),
            script: "SELECT * WHERE amount > 100".to_string(),
        };

        tracker.track_transformation(&transform).await.unwrap();
    }
}
