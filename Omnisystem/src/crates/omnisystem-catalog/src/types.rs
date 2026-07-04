use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub keyword: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entry: CatalogEntry,
    pub relevance_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_entry_creation() {
        let entry = CatalogEntry {
            id: "test".to_string(),
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            description: "test".to_string(),
            tags: vec!["test".to_string()],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert_eq!(entry.name, "test");
    }

    #[test]
    fn test_search_query() {
        let query = SearchQuery {
            keyword: Some("test".to_string()),
            tags: vec![],
            author: None,
            limit: 10,
            offset: 0,
        };
        assert_eq!(query.limit, 10);
    }
}
