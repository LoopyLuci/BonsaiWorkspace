//! ICDS CLI - exercises the Infinite Context Data Store engine end to end

use icds::atom::{AtomMetadata, SourceType};
use icds::InfiniteContextEngine;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = InfiniteContextEngine::new().await?;

    let metadata = AtomMetadata {
        source: SourceType::UserInput,
        agent_id: Uuid::new_v4(),
        conversation_id: None,
        tags: vec!["cli".to_string()],
        importance: 1.0,
    };

    let ids = engine
        .ingest(
            "The Infinite Context Data Store keeps agent history queryable.",
            metadata,
        )
        .await?;
    println!("Ingested {} atom(s): {:?}", ids.len(), ids);

    let results = engine.query("context data store", 5).await?;
    println!(
        "Query returned {} atom(s) in {}us",
        results.atoms.len(),
        results.latency_us
    );

    let context = engine.assemble_context("context data store", 200).await?;
    println!("Assembled context:\n{}", context);

    println!("Total atoms stored: {}", engine.atom_count().await?);

    Ok(())
}
