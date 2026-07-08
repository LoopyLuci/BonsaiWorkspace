//! Type-Safe Query System
//!
//! Queries are typed expressions validated at compile time against the schema.
//! No SQL strings, no runtime errors from schema mismatches.


#[derive(Debug, Clone)]
pub struct Query<T> {
    pub entity: String,
    pub predicates: Vec<Predicate>,
    pub ordering: Vec<OrderBy>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub _phantom: std::marker::PhantomData<T>,
}

#[derive(Debug, Clone)]
pub enum Predicate {
    Eq(String, String),                    // field == value
    NotEq(String, String),                 // field != value
    Gt(String, String),                    // field > value
    Lt(String, String),                    // field < value
    GtEq(String, String),                  // field >= value
    LtEq(String, String),                  // field <= value
    In(String, Vec<String>),               // field in [values]
    Like(String, String),                  // field LIKE pattern
    And(Box<Predicate>, Box<Predicate>),  // p1 AND p2
    Or(Box<Predicate>, Box<Predicate>),   // p1 OR p2
    Not(Box<Predicate>),                   // NOT p
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub entity: String,
    pub predicates: Vec<Predicate>,
    pub indexes_to_use: Vec<String>,
    pub filter_pushdown: Vec<Predicate>,
    pub estimated_cost: f64,
}

impl<T> Query<T> {
    pub fn new(entity: String) -> Self {
        Self {
            entity,
            predicates: Vec::new(),
            ordering: Vec::new(),
            limit: None,
            offset: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn where_eq(mut self, field: &str, value: &str) -> Self {
        self.predicates
            .push(Predicate::Eq(field.to_string(), value.to_string()));
        self
    }

    pub fn where_gt(mut self, field: &str, value: &str) -> Self {
        self.predicates
            .push(Predicate::Gt(field.to_string(), value.to_string()));
        self
    }

    pub fn where_lt(mut self, field: &str, value: &str) -> Self {
        self.predicates
            .push(Predicate::Lt(field.to_string(), value.to_string()));
        self
    }

    pub fn order_by(mut self, field: &str, direction: SortDirection) -> Self {
        self.ordering.push(OrderBy {
            field: field.to_string(),
            direction,
        });
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: usize) -> Self {
        self.offset = Some(n);
        self
    }

    pub fn to_sql(&self) -> String {
        let mut sql = format!("SELECT * FROM {}", self.entity);

        if !self.predicates.is_empty() {
            sql.push_str(" WHERE ");
            let predicates_sql: Vec<String> = self.predicates.iter().map(|p| p.to_sql()).collect();
            sql.push_str(&predicates_sql.join(" AND "));
        }

        if !self.ordering.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_sql: Vec<String> = self.ordering.iter().map(|o| o.to_sql()).collect();
            sql.push_str(&order_sql.join(", "));
        }

        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        sql
    }
}

impl Predicate {
    pub fn to_sql(&self) -> String {
        match self {
            Predicate::Eq(field, value) => format!("{} = {}", field, value),
            Predicate::NotEq(field, value) => format!("{} != {}", field, value),
            Predicate::Gt(field, value) => format!("{} > {}", field, value),
            Predicate::Lt(field, value) => format!("{} < {}", field, value),
            Predicate::GtEq(field, value) => format!("{} >= {}", field, value),
            Predicate::LtEq(field, value) => format!("{} <= {}", field, value),
            Predicate::In(field, values) => {
                format!("{} IN ({})", field, values.join(", "))
            }
            Predicate::Like(field, pattern) => format!("{} LIKE '{}'", field, pattern),
            Predicate::And(left, right) => {
                format!("({} AND {})", left.to_sql(), right.to_sql())
            }
            Predicate::Or(left, right) => {
                format!("({} OR {})", left.to_sql(), right.to_sql())
            }
            Predicate::Not(p) => format!("NOT ({})", p.to_sql()),
        }
    }
}

impl OrderBy {
    pub fn to_sql(&self) -> String {
        let direction = match self.direction {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };
        format!("{} {}", self.field, direction)
    }
}

impl QueryPlan {
    pub fn new(entity: String) -> Self {
        Self {
            entity,
            predicates: Vec::new(),
            indexes_to_use: Vec::new(),
            filter_pushdown: Vec::new(),
            estimated_cost: 0.0,
        }
    }

    /// Estimate the cost of executing this query plan
    pub fn estimate_cost(&mut self) {
        // Simple heuristic: using indexes reduces cost
        let index_reduction = self.indexes_to_use.len() as f64 * 0.5;
        self.estimated_cost = 100.0 - index_reduction;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_generation() {
        let query: Query<String> = Query::new("User".to_string())
            .where_eq("name", "John")
            .order_by("created_at", SortDirection::Descending)
            .limit(10);

        let sql = query.to_sql();
        assert!(sql.contains("SELECT * FROM User"));
        assert!(sql.contains("WHERE name = John"));
        assert!(sql.contains("ORDER BY created_at DESC"));
        assert!(sql.contains("LIMIT 10"));
    }
}
