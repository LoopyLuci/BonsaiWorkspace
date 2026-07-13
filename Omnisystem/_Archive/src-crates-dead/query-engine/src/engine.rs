use crate::{Query, QueryStatus, QueryPlan, OptimizedPlan, ExecutionStats, IndexInfo, QueryError, QueryResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct QueryEngine {
    queries: Arc<DashMap<Uuid, Query>>,
    plans: Arc<DashMap<Uuid, QueryPlan>>,
    optimizations: Arc<DashMap<Uuid, OptimizedPlan>>,
    stats: Arc<DashMap<Uuid, ExecutionStats>>,
    indexes: Arc<DashMap<Uuid, IndexInfo>>,
}

impl QueryEngine {
    pub fn new() -> Self {
        Self {
            queries: Arc::new(DashMap::new()),
            plans: Arc::new(DashMap::new()),
            optimizations: Arc::new(DashMap::new()),
            stats: Arc::new(DashMap::new()),
            indexes: Arc::new(DashMap::new()),
        }
    }

    pub async fn submit_query(&self, sql: &str) -> QueryResult<Query> {
        let query = Query {
            query_id: Uuid::new_v4(),
            sql: sql.to_string(),
            submitted_at: Utc::now(),
            status: QueryStatus::Submitted,
        };

        self.queries.insert(query.query_id, query.clone());
        Ok(query)
    }

    pub async fn create_plan(&self, query_id: Uuid, operations: Vec<String>, estimated_cost: f64) -> QueryResult<QueryPlan> {
        if self.queries.get(&query_id).is_none() {
            return Err(QueryError::ExecutionFailed);
        }

        let plan = QueryPlan {
            plan_id: Uuid::new_v4(),
            query_id,
            operations,
            estimated_cost,
            estimated_rows: 1000,
        };

        self.plans.insert(plan.plan_id, plan.clone());
        Ok(plan)
    }

    pub async fn optimize_plan(&self, plan_id: Uuid) -> QueryResult<OptimizedPlan> {
        if let Some(plan) = self.plans.get(&plan_id) {
            let original_cost = plan.estimated_cost;
            let optimized_cost = original_cost * 0.7;

            let optimization = OptimizedPlan {
                optimization_id: Uuid::new_v4(),
                plan_id,
                original_cost,
                optimized_cost,
                optimization_rules: vec!["index_push_down".to_string(), "predicate_pushdown".to_string()],
            };

            self.optimizations.insert(optimization.optimization_id, optimization.clone());
            Ok(optimization)
        } else {
            Err(QueryError::PlanningFailed)
        }
    }

    pub async fn execute_query(&self, query_id: Uuid) -> QueryResult<ExecutionStats> {
        if self.queries.get(&query_id).is_none() {
            return Err(QueryError::ExecutionFailed);
        }

        let stats = ExecutionStats {
            stats_id: Uuid::new_v4(),
            query_id,
            rows_examined: 1500,
            rows_returned: 500,
            execution_time_ms: 125,
            index_used: Some("idx_primary".to_string()),
        };

        self.stats.insert(stats.stats_id, stats.clone());
        Ok(stats)
    }

    pub async fn register_index(&self, index_name: &str, table_name: &str, columns: Vec<String>) -> QueryResult<IndexInfo> {
        let index = IndexInfo {
            index_id: Uuid::new_v4(),
            index_name: index_name.to_string(),
            table_name: table_name.to_string(),
            column_names: columns,
            index_type: "BTree".to_string(),
            cardinality: 1000000,
        };

        self.indexes.insert(index.index_id, index.clone());
        Ok(index)
    }

    pub fn query_count(&self) -> usize {
        self.queries.len()
    }
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_submit_query() {
        let engine = QueryEngine::new();
        let query = engine.submit_query("SELECT * FROM users WHERE id = 123").await.unwrap();

        assert_eq!(query.status, QueryStatus::Submitted);
        assert_eq!(engine.query_count(), 1);
    }

    #[tokio::test]
    async fn test_create_plan() {
        let engine = QueryEngine::new();
        let query = engine.submit_query("SELECT * FROM orders").await.unwrap();

        let plan = engine
            .create_plan(query.query_id, vec!["SeqScan".to_string()], 100.0)
            .await
            .unwrap();

        assert_eq!(plan.estimated_cost, 100.0);
    }

    #[tokio::test]
    async fn test_optimize_plan() {
        let engine = QueryEngine::new();
        let query = engine.submit_query("SELECT * FROM products").await.unwrap();
        let plan = engine.create_plan(query.query_id, vec![], 50.0).await.unwrap();

        let optimized = engine.optimize_plan(plan.plan_id).await.unwrap();
        assert!(optimized.optimized_cost < optimized.original_cost);
    }

    #[tokio::test]
    async fn test_register_index() {
        let engine = QueryEngine::new();
        let index = engine
            .register_index("idx_user_id", "users", vec!["user_id".to_string()])
            .await
            .unwrap();

        assert_eq!(index.index_name, "idx_user_id");
        assert_eq!(index.table_name, "users");
    }
}
