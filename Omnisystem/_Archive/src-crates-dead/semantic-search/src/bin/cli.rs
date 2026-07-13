//! CLI

use semantic_search::SemanticSearchEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = SemanticSearchEngine::new();
    engine.create_embedding("doc1", vec![1.0, 0.0, 0.0]).await?;

    let results = engine.semantic_search("query", vec![1.0, 0.0, 0.0]).await?;
    println!("Found {} results", results.len());
    println!("Total embeddings: {}", engine.embedding_count());
    Ok(())
}
