//! CLI for exercising the query-engine crate.

use query_engine::QueryEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = QueryEngine::new();

    let query = engine.submit_query("SELECT * FROM users WHERE id = 42").await?;
    println!("Submitted query {} ({})", query.query_id, query.sql);

    let plan = engine
        .create_plan(query.query_id, vec!["IndexScan".to_string()], 12.5)
        .await?;
    println!("Plan estimates {} rows at cost {:.1}", plan.estimated_rows, plan.estimated_cost);

    let optimized = engine.optimize_plan(plan.plan_id).await?;
    println!("Optimized cost {:.2} -> {:.2}", optimized.original_cost, optimized.optimized_cost);

    engine.register_index("idx_users_id", "users", vec!["id".to_string()]).await?;

    let stats = engine.execute_query(query.query_id).await?;
    println!(
        "Executed: examined {} rows, returned {}, index used: {:?}, took {}ms",
        stats.rows_examined, stats.rows_returned, stats.index_used, stats.execution_time_ms
    );

    println!("Total queries tracked: {}", engine.query_count());
    Ok(())
}
