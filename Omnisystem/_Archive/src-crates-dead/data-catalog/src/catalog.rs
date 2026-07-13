use crate::{Dataset, DatasetMetadata, DatasetTag, DatasetOwnership, SearchResult, CatalogError, CatalogResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct DataCatalog {
    datasets: Arc<DashMap<Uuid, Dataset>>,
    metadata: Arc<DashMap<Uuid, DatasetMetadata>>,
    tags: Arc<DashMap<Uuid, DatasetTag>>,
    ownership: Arc<DashMap<Uuid, DatasetOwnership>>,
}

impl DataCatalog {
    pub fn new() -> Self {
        Self {
            datasets: Arc::new(DashMap::new()),
            metadata: Arc::new(DashMap::new()),
            tags: Arc::new(DashMap::new()),
            ownership: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_dataset(&self, dataset: &Dataset) -> CatalogResult<()> {
        self.datasets.insert(dataset.dataset_id, dataset.clone());
        Ok(())
    }

    pub async fn get_dataset(&self, dataset_id: Uuid) -> CatalogResult<Dataset> {
        self.datasets
            .get(&dataset_id)
            .map(|d| d.clone())
            .ok_or(CatalogError::DatasetNotFound)
    }

    pub async fn add_metadata(&self, metadata: &DatasetMetadata) -> CatalogResult<()> {
        self.metadata.insert(metadata.metadata_id, metadata.clone());
        Ok(())
    }

    pub async fn add_tag(&self, tag: &DatasetTag) -> CatalogResult<()> {
        self.tags.insert(tag.tag_id, tag.clone());
        Ok(())
    }

    pub async fn set_ownership(&self, ownership: &DatasetOwnership) -> CatalogResult<()> {
        self.ownership.insert(ownership.ownership_id, ownership.clone());
        Ok(())
    }

    pub async fn search_datasets(&self, query: &str) -> CatalogResult<Vec<SearchResult>> {
        let mut results = Vec::new();

        for entry in self.datasets.iter() {
            let dataset = entry.value();
            if dataset.name.to_lowercase().contains(&query.to_lowercase()) {
                results.push(SearchResult {
                    result_id: Uuid::new_v4(),
                    dataset_id: dataset.dataset_id,
                    match_score: 1.0,
                    relevant_fields: vec!["name".to_string()],
                });
            }
        }

        Ok(results)
    }

    pub fn dataset_count(&self) -> usize {
        self.datasets.len()
    }
}

impl Default for DataCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_dataset() {
        let catalog = DataCatalog::new();
        let dataset = Dataset {
            dataset_id: Uuid::new_v4(),
            name: "customer_events".to_string(),
            owner: "data_team".to_string(),
            location: "s3://bucket/customer_events".to_string(),
            created_at: Utc::now(),
        };

        catalog.register_dataset(&dataset).await.unwrap();
        assert_eq!(catalog.dataset_count(), 1);
    }

    #[tokio::test]
    async fn test_add_metadata() {
        let catalog = DataCatalog::new();
        let dataset_id = Uuid::new_v4();
        let metadata = DatasetMetadata {
            metadata_id: Uuid::new_v4(),
            dataset_id,
            record_count: 1_000_000,
            size_bytes: 10_000_000,
            format: "parquet".to_string(),
        };

        catalog.add_metadata(&metadata).await.unwrap();
    }

    #[tokio::test]
    async fn test_add_tag() {
        let catalog = DataCatalog::new();
        let tag = DatasetTag {
            tag_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            tag: "pii".to_string(),
            category: "sensitivity".to_string(),
        };

        catalog.add_tag(&tag).await.unwrap();
    }

    #[tokio::test]
    async fn test_search_datasets() {
        let catalog = DataCatalog::new();
        let dataset = Dataset {
            dataset_id: Uuid::new_v4(),
            name: "user_profiles".to_string(),
            owner: "analytics".to_string(),
            location: "s3://bucket/users".to_string(),
            created_at: Utc::now(),
        };

        catalog.register_dataset(&dataset).await.unwrap();
        let results = catalog.search_datasets("user").await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
