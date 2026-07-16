//! CLI that exercises the module catalog and search engine.

use omnisystem_catalog::{CatalogEntry, ModuleCatalog, SearchQuery};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ModuleCatalog::new();

    catalog.register(CatalogEntry {
        id: "http-toolkit".to_string(),
        name: "http client toolkit".to_string(),
        version: "1.0.0".to_string(),
        author: "omnisystem".to_string(),
        description: "an http client toolkit for services".to_string(),
        tags: vec!["net".to_string()],
        metadata: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })?;
    catalog.register(CatalogEntry {
        id: "generic-toolkit".to_string(),
        name: "toolkit".to_string(),
        version: "1.0.0".to_string(),
        author: "omnisystem".to_string(),
        description: "a generic toolkit".to_string(),
        tags: vec!["misc".to_string()],
        metadata: HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })?;

    println!("Catalog has {} entries", catalog.count());

    let query = SearchQuery {
        keyword: Some("toolkit".to_string()),
        tags: vec![],
        author: None,
        limit: 10,
        offset: 0,
    };
    for result in catalog.search(&query)? {
        println!(
            "{} (relevance {:.2})",
            result.entry.name, result.relevance_score
        );
    }

    Ok(())
}
