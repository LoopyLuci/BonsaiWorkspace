//! CLI: create a sharded table, add a shard and partition, plan a query, and
//! create an index.

use distributed_database::{ColumnDef, ConsistencyLevel, DistributedDatabase, IndexType, TableSchema};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = DistributedDatabase::new();

    let schema = TableSchema {
        columns: vec![
            ColumnDef { name: "id".to_string(), data_type: "integer".to_string(), nullable: false },
            ColumnDef { name: "email".to_string(), data_type: "string".to_string(), nullable: false },
        ],
        primary_key: "id".to_string(),
    };
    let table = db.create_table("users", schema, 8).await?;
    println!("created table '{}' with {} shards", table.name, table.shard_count);

    let shard = db.create_shard(table.table_id, 0, ("a".to_string(), "m".to_string())).await?;
    println!("created shard {} on {}", shard.shard_number, shard.node_id);

    let partition = db.create_partition(shard.shard_id, "a", "g").await?;
    println!("created partition [{}, {})", partition.min_key, partition.max_key);

    let route = db.plan_query(table.table_id, ConsistencyLevel::Strong).await?;
    println!("query plan touches {} shard(s)", route.shards_involved.len());

    let index = db.create_index(table.table_id, "email", IndexType::Hash).await?;
    println!("created {:?} index on column '{}'", index.index_type, index.column_name);

    println!("total tables: {}", db.table_count());

    Ok(())
}
