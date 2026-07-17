use crate::{Query, QueryStatus, QueryPlan, OptimizedPlan, ExecutionStats, IndexInfo, QueryError, QueryResult};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
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

        let estimated_rows = Self::estimate_rows(&operations, estimated_cost);

        let plan = QueryPlan {
            plan_id: Uuid::new_v4(),
            query_id,
            operations,
            estimated_cost,
            estimated_rows,
        };

        self.plans.insert(plan.plan_id, plan.clone());
        Ok(plan)
    }

    /// Derive a row-count estimate from the plan's operations and cost,
    /// the way a real optimizer would use selectivity heuristics per
    /// operator rather than a single constant regardless of the plan.
    fn estimate_rows(operations: &[String], estimated_cost: f64) -> u64 {
        let mut rows = (estimated_cost * 100.0).max(1.0);
        for op in operations {
            match op.as_str() {
                "IndexScan" => rows *= 0.01,
                "Filter" => rows *= 0.3,
                "Join" => rows *= 2.0,
                _ => {}
            }
        }
        rows.round().max(1.0) as u64
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

    /// Execute the query, using the most recent plan created for it (if
    /// any) to derive real row-count estimates and checking the registered
    /// index catalog for an index whose table appears in the SQL text,
    /// rather than fabricating fixed numbers regardless of the query.
    pub async fn execute_query(&self, query_id: Uuid) -> QueryResult<ExecutionStats> {
        let query = self
            .queries
            .get(&query_id)
            .ok_or(QueryError::ExecutionFailed)?
            .clone();

        let started = Instant::now();

        let plan = self
            .plans
            .iter()
            .filter(|e| e.value().query_id == query_id)
            .map(|e| e.value().clone())
            .last();

        let rows_examined = plan.as_ref().map(|p| p.estimated_rows).unwrap_or(0);

        let index_used = self
            .indexes
            .iter()
            .find(|idx| query.sql.contains(&idx.value().table_name))
            .map(|idx| idx.value().index_name.clone());

        // An index-assisted scan returns a much smaller fraction of the
        // rows it examines than a full scan does.
        let rows_returned = if index_used.is_some() {
            ((rows_examined as f64) * 0.1).ceil() as u64
        } else {
            rows_examined
        };

        let execution_time_ms = started.elapsed().as_millis() as u64;

        let stats = ExecutionStats {
            stats_id: Uuid::new_v4(),
            query_id,
            rows_examined,
            rows_returned,
            execution_time_ms,
            index_used,
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

    #[tokio::test]
    async fn test_create_plan_estimated_rows_reflects_operations() {
        let engine = QueryEngine::new();
        let query = engine.submit_query("SELECT * FROM orders").await.unwrap();

        let seq_scan = engine
            .create_plan(query.query_id, vec!["SeqScan".to_string()], 100.0)
            .await
            .unwrap();
        let index_scan = engine
            .create_plan(query.query_id, vec!["IndexScan".to_string()], 100.0)
            .await
            .unwrap();

        // An index scan should be estimated to touch far fewer rows than a
        // sequential scan of the same estimated cost.
        assert!(index_scan.estimated_rows < seq_scan.estimated_rows);
    }

    #[tokio::test]
    async fn test_execute_query_uses_plan_and_index() {
        let engine = QueryEngine::new();
        let query = engine.submit_query("SELECT * FROM users WHERE id = 1").await.unwrap();
        engine
            .create_plan(query.query_id, vec!["IndexScan".to_string()], 10.0)
            .await
            .unwrap();
        engine
            .register_index("idx_users_id", "users", vec!["id".to_string()])
            .await
            .unwrap();

        let stats = engine.execute_query(query.query_id).await.unwrap();
        assert_eq!(stats.index_used.as_deref(), Some("idx_users_id"));
        assert!(stats.rows_returned <= stats.rows_examined);
        assert!(stats.rows_examined > 0);
    }

    #[tokio::test]
    async fn test_execute_query_without_plan_or_index() {
        let engine = QueryEngine::new();
        let query = engine.submit_query("SELECT 1").await.unwrap();

        let stats = engine.execute_query(query.query_id).await.unwrap();
        assert_eq!(stats.rows_examined, 0);
        assert_eq!(stats.rows_returned, 0);
        assert!(stats.index_used.is_none());
    }

    #[tokio::test]
    async fn test_execute_unknown_query_fails() {
        let engine = QueryEngine::new();
        let result = engine.execute_query(Uuid::new_v4()).await;
        assert!(matches!(result, Err(QueryError::ExecutionFailed)));
    }
}
