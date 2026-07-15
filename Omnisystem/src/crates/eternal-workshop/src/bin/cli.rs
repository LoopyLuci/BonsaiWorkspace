//! EternalWorkshop CLI: seeds a temporary memory-node database, runs a real
//! dream cycle (falling back to the heuristic consolidator since no
//! DreamAgent sidecar is running), and prints the resulting BONSAI.md.

use eternal_workshop::{config::Config, dream_executor, memory_nodes::MemoryNode, MemoryNodeStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let workspace = tempfile::TempDir::new()?;
    let db_path = workspace.path().join("memory_nodes.db");
    let store = MemoryNodeStore::open(&db_path).await?;

    let now = chrono::Utc::now().timestamp_millis();
    let samples = [
        ("n1", "Fixed off-by-one error in the chunk scheduler"),
        ("n2", "Fixed off-by-one error in the chunk scheduler"), // duplicate, should dedup
        ("n3", "Wrote integration test for the relay handshake"),
        ("n4", "Decided to use BLAKE3 for session token derivation"),
    ];
    for (id, content) in samples {
        store
            .insert_node(&MemoryNode {
                id: id.to_string(),
                timestamp_ms: now,
                node_type: "edit".to_string(),
                source: "cli-demo".to_string(),
                content: content.to_string(),
                tags: vec!["demo".to_string()],
                consolidated: false,
            })
            .await?;
    }

    println!("pending nodes before cycle: {}", store.pending_count().await?);

    let cfg = Config {
        db_path: db_path.clone(),
        workspace_path: Some(workspace.path().to_path_buf()),
        api_port: 0,
        dream_agent_port: 1, // nothing listens here -> forces the heuristic fallback
        idle_trigger_mins: 30,
    };

    let summary = dream_executor::run_dream_cycle(&store, &cfg, cfg.workspace_path.as_deref()).await?;

    println!("cycle summary: {summary}");
    println!("pending nodes after cycle: {}", store.pending_count().await?);

    let bonsai_md = std::fs::read_to_string(workspace.path().join("BONSAI.md"))?;
    println!("\n--- BONSAI.md ---\n{bonsai_md}");

    Ok(())
}
