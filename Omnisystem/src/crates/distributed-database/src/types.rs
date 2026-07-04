use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Table {
    pub table_id: Uuid,
    pub name: String,
    pub schema: TableSchema,
    pub shard_count: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TableSchema {
    pub columns: Vec<ColumnDef>,
    pub primary_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shard {
    pub shard_id: Uuid,
    pub table_id: Uuid,
    pub shard_number: u32,
    pub key_range: (String, String),
    pub replica_count: u32,
    pub node_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Partition {
    pub partition_id: Uuid,
    pub shard_id: Uuid,
    pub min_key: String,
    pub max_key: String,
    pub record_count: u64,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryRoute {
    pub route_id: Uuid,
    pub shards_involved: Vec<Uuid>,
    pub consistency_level: ConsistencyLevel,
    pub timeout_ms: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum ConsistencyLevel {
    Strong,
    Eventual,
    WeakEventual,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexDefinition {
    pub index_id: Uuid,
    pub table_id: Uuid,
    pub column_name: String,
    pub index_type: IndexType,
    pub unique: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum IndexType {
    BTree,
    Hash,
    Bloom,
}
