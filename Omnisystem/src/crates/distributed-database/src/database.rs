use crate::{Table, TableSchema, Shard, Partition, QueryRoute, ConsistencyLevel, IndexDefinition, IndexType, DatabaseError, DatabaseResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct DistributedDatabase {
    tables: Arc<DashMap<Uuid, Table>>,
    shards: Arc<DashMap<Uuid, Shard>>,
    partitions: Arc<DashMap<Uuid, Partition>>,
    routes: Arc<DashMap<Uuid, QueryRoute>>,
    indexes: Arc<DashMap<Uuid, IndexDefinition>>,
}

impl DistributedDatabase {
    pub fn new() -> Self {
        Self {
            tables: Arc::new(DashMap::new()),
            shards: Arc::new(DashMap::new()),
            partitions: Arc::new(DashMap::new()),
            routes: Arc::new(DashMap::new()),
            indexes: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_table(&self, name: &str, schema: TableSchema, shard_count: u32) -> DatabaseResult<Table> {
        let table = Table {
            table_id: Uuid::new_v4(),
            name: name.to_string(),
            schema,
            shard_count,
            created_at: Utc::now(),
        };

        self.tables.insert(table.table_id, table.clone());
        Ok(table)
    }

    pub async fn create_shard(&self, table_id: Uuid, shard_number: u32, key_range: (String, String)) -> DatabaseResult<Shard> {
        if self.tables.get(&table_id).is_none() {
            return Err(DatabaseError::TableNotFound);
        }

        let shard = Shard {
            shard_id: Uuid::new_v4(),
            table_id,
            shard_number,
            key_range,
            replica_count: 3,
            node_id: format!("node-{}", shard_number),
        };

        self.shards.insert(shard.shard_id, shard.clone());
        Ok(shard)
    }

    pub async fn create_partition(&self, shard_id: Uuid, min_key: &str, max_key: &str) -> DatabaseResult<Partition> {
        if self.shards.get(&shard_id).is_none() {
            return Err(DatabaseError::ShardNotFound);
        }

        let partition = Partition {
            partition_id: Uuid::new_v4(),
            shard_id,
            min_key: min_key.to_string(),
            max_key: max_key.to_string(),
            record_count: 0,
            size_bytes: 0,
        };

        self.partitions.insert(partition.partition_id, partition.clone());
        Ok(partition)
    }

    pub async fn plan_query(&self, table_id: Uuid, consistency: ConsistencyLevel) -> DatabaseResult<QueryRoute> {
        let mut shards_involved = Vec::new();

        for entry in self.shards.iter() {
            if entry.value().table_id == table_id {
                shards_involved.push(entry.value().shard_id);
            }
        }

        if shards_involved.is_empty() {
            return Err(DatabaseError::RoutingFailed);
        }

        let route = QueryRoute {
            route_id: Uuid::new_v4(),
            shards_involved,
            consistency_level: consistency,
            timeout_ms: 5000,
        };

        self.routes.insert(route.route_id, route.clone());
        Ok(route)
    }

    pub async fn create_index(&self, table_id: Uuid, column_name: &str, index_type: IndexType) -> DatabaseResult<IndexDefinition> {
        if self.tables.get(&table_id).is_none() {
            return Err(DatabaseError::TableNotFound);
        }

        let index = IndexDefinition {
            index_id: Uuid::new_v4(),
            table_id,
            column_name: column_name.to_string(),
            index_type,
            unique: false,
        };

        self.indexes.insert(index.index_id, index.clone());
        Ok(index)
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }
}

impl Default for DistributedDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColumnDef;

    #[tokio::test]
    async fn test_create_table() {
        let db = DistributedDatabase::new();
        let schema = TableSchema {
            columns: vec![
                ColumnDef { name: "id".to_string(), data_type: "integer".to_string(), nullable: false },
                ColumnDef { name: "name".to_string(), data_type: "string".to_string(), nullable: false },
            ],
            primary_key: "id".to_string(),
        };

        let table = db.create_table("users", schema, 8).await.unwrap();
        assert_eq!(table.name, "users");
        assert_eq!(table.shard_count, 8);
        assert_eq!(db.table_count(), 1);
    }

    #[tokio::test]
    async fn test_create_shard() {
        let db = DistributedDatabase::new();
        let schema = TableSchema {
            columns: vec![],
            primary_key: "id".to_string(),
        };

        let table = db.create_table("products", schema, 4).await.unwrap();
        let shard = db.create_shard(table.table_id, 0, ("a".to_string(), "m".to_string())).await.unwrap();

        assert_eq!(shard.shard_number, 0);
        assert_eq!(shard.replica_count, 3);
    }

    #[tokio::test]
    async fn test_create_partition() {
        let db = DistributedDatabase::new();
        let schema = TableSchema { columns: vec![], primary_key: "id".to_string() };
        let table = db.create_table("orders", schema, 2).await.unwrap();
        let shard = db.create_shard(table.table_id, 0, ("a".to_string(), "z".to_string())).await.unwrap();

        let partition = db.create_partition(shard.shard_id, "a", "m").await.unwrap();
        assert_eq!(partition.min_key, "a");
    }

    #[tokio::test]
    async fn test_plan_query() {
        let db = DistributedDatabase::new();
        let schema = TableSchema { columns: vec![], primary_key: "id".to_string() };
        let table = db.create_table("data", schema, 3).await.unwrap();

        db.create_shard(table.table_id, 0, ("a".to_string(), "i".to_string())).await.unwrap();

        let route = db.plan_query(table.table_id, ConsistencyLevel::Strong).await.unwrap();
        assert!(route.shards_involved.len() > 0);
    }
}
