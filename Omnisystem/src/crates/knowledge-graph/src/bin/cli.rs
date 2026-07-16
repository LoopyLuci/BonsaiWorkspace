//! CLI: build a small graph and find a multi-hop path through it.

use knowledge_graph::KnowledgeGraph;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph = KnowledgeGraph::new();

    let alice = graph.create_entity("Alice", "Person").await?;
    let bob = graph.create_entity("Bob", "Person").await?;
    let carol = graph.create_entity("Carol", "Person").await?;

    graph.create_relationship(alice.entity_id, bob.entity_id, "knows").await?;
    graph.create_relationship(bob.entity_id, carol.entity_id, "knows").await?;
    graph.add_triple(alice.entity_id, "works_at", "Acme_Corp").await?;

    let query = graph.query_graph("works_at").await?;
    println!("query 'works_at' matched {} triple(s)", query.results_count);

    let path = graph.find_path(alice.entity_id, carol.entity_id).await?;
    println!(
        "path from Alice to Carol: {} hop(s) through {} entities",
        path.path_length,
        path.path_entities.len()
    );

    println!("total entities: {}", graph.entity_count());

    Ok(())
}
