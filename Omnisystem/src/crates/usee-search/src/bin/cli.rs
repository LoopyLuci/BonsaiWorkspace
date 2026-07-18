//! CLI demo for usee-search: indexes a handful of documents and runs a
//! real BM25 search against them via SearchEngine.

use std::collections::HashMap;
use usee_search::{Document, Query, SearchEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = SearchEngine::new();

    let docs = vec![
        ("doc1", "Rust Programming", "Rust is a systems programming language that empowers developers"),
        ("doc2", "Python Basics", "Python is a high-level programming language for developers"),
        ("doc3", "Cooking Guide", "A guide to cooking pasta and other Italian dishes"),
    ];

    for (id, title, content) in docs {
        engine.index_document(Document {
            id: id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            metadata: HashMap::new(),
            score: 0.0,
        })?;
    }

    println!("Indexed {} documents", engine.document_count());

    let query = Query {
        text: "programming language".to_string(),
        limit: 5,
        offset: 0,
        filters: HashMap::new(),
    };

    let results = engine.search(&query)?;
    println!(
        "Search for {:?}: {} results in {}ms",
        query.text, results.total, results.query_time_ms
    );
    for doc in &results.documents {
        println!("  {} ({:.3}): {}", doc.id, doc.score, doc.title);
    }

    Ok(())
}
