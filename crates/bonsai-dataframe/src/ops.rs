//! Serialisable operation descriptors for DataFrame transformations.
//!
//! These types act as a thin DSL that can be sent over the Tauri IPC boundary
//! (JSON) and then evaluated against a `BonsaiFrame`.

use serde::{Deserialize, Serialize};

/// Scalar literal — used in filter/sort expressions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "v")]
pub enum Scalar {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

/// A filter predicate expressed as a portable JSON structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum FilterExpr {
    /// col == value
    Eq   { col: String, val: Scalar },
    /// col != value
    Ne   { col: String, val: Scalar },
    /// col < value
    Lt   { col: String, val: Scalar },
    /// col <= value
    Le   { col: String, val: Scalar },
    /// col > value
    Gt   { col: String, val: Scalar },
    /// col >= value
    Ge   { col: String, val: Scalar },
    /// col contains substring (strings only)
    Contains { col: String, pat: String },
    /// col is null
    IsNull   { col: String },
    /// col is not null
    IsNotNull{ col: String },
    /// Logical AND of sub-expressions
    And(Vec<FilterExpr>),
    /// Logical OR  of sub-expressions
    Or(Vec<FilterExpr>),
    /// Logical NOT of a sub-expression
    Not(Box<FilterExpr>),
}

/// Sort specification for one column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    pub col: String,
    pub descending: bool,
    pub nulls_last: bool,
}

/// Aggregation expression used in `group_by` operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "agg")]
pub enum AggExpr {
    Sum   { col: String, alias: Option<String> },
    Mean  { col: String, alias: Option<String> },
    Min   { col: String, alias: Option<String> },
    Max   { col: String, alias: Option<String> },
    Count { col: String, alias: Option<String> },
    First { col: String, alias: Option<String> },
    Last  { col: String, alias: Option<String> },
    Std   { col: String, alias: Option<String> },
    Median{ col: String, alias: Option<String> },
}

impl AggExpr {
    pub fn col(&self) -> &str {
        match self {
            Self::Sum { col, .. } | Self::Mean { col, .. } | Self::Min { col, .. }
            | Self::Max { col, .. } | Self::Count { col, .. } | Self::First { col, .. }
            | Self::Last { col, .. } | Self::Std { col, .. } | Self::Median { col, .. } => col,
        }
    }
    pub fn alias(&self) -> Option<&str> {
        match self {
            Self::Sum { alias, .. } | Self::Mean { alias, .. } | Self::Min { alias, .. }
            | Self::Max { alias, .. } | Self::Count { alias, .. } | Self::First { alias, .. }
            | Self::Last { alias, .. } | Self::Std { alias, .. } | Self::Median { alias, .. } => alias.as_deref(),
        }
    }
}
