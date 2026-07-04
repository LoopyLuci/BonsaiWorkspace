// OMNI QUERY LANGUAGE (OQL) - Query Engine
// Universal query language for OMNI documents
// Version: 2.0

use std::collections::HashMap;
use crate::omni_format::OmniValue;

/// OQL Query types
#[derive(Debug, Clone)]
pub enum QueryType {
    Select {
        fields: Vec<String>,
        from: String,
        where_clause: Option<Condition>,
        order_by: Option<(String, OrderDirection)>,
        limit: Option<usize>,
        offset: Option<usize>,
    },
    Filter {
        from: String,
        conditions: Vec<Condition>,
    },
    Aggregate {
        operations: Vec<AggregateOp>,
        from: String,
        group_by: Option<Vec<String>>,
        having: Option<Condition>,
    },
    Join {
        table1: String,
        table2: String,
        on: JoinCondition,
        join_type: JoinType,
    },
    Search {
        terms: Vec<String>,
        in_fields: Vec<String>,
        weights: Option<HashMap<String, f32>>,
    },
}

#[derive(Debug, Clone)]
pub enum OrderDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone)]
pub struct JoinCondition {
    pub left_field: String,
    pub right_field: String,
}

#[derive(Debug, Clone)]
pub enum AggregateOp {
    Sum(String),
    Count,
    Avg(String),
    Min(String),
    Max(String),
    StdDev(String),
}

#[derive(Debug, Clone)]
pub enum Condition {
    Equals {
        field: String,
        value: OmniValue,
    },
    NotEquals {
        field: String,
        value: OmniValue,
    },
    GreaterThan {
        field: String,
        value: OmniValue,
    },
    LessThan {
        field: String,
        value: OmniValue,
    },
    GreaterThanOrEqual {
        field: String,
        value: OmniValue,
    },
    LessThanOrEqual {
        field: String,
        value: OmniValue,
    },
    Contains {
        field: String,
        value: String,
    },
    StartsWith {
        field: String,
        value: String,
    },
    EndsWith {
        field: String,
        value: String,
    },
    In {
        field: String,
        values: Vec<OmniValue>,
    },
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

/// Query Result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows: Vec<OmniValue>,
    pub count: usize,
    pub execution_time_ms: u128,
}

/// OQL Parser
pub struct OqlParser;

impl OqlParser {
    pub fn parse(query: &str) -> Result<QueryType, ParseError> {
        let tokens = Self::tokenize(query)?;
        Self::parse_tokens(&tokens)
    }

    fn tokenize(query: &str) -> Result<Vec<String>, ParseError> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_string = false;
        let mut escape = false;

        for ch in query.chars() {
            if escape {
                current_token.push(ch);
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = !in_string;
            } else if !in_string && (ch == ' ' || ch == ',' || ch == '(' || ch == ')') {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                if ch != ' ' {
                    tokens.push(ch.to_string());
                }
            } else {
                current_token.push(ch);
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        Ok(tokens)
    }

    fn parse_tokens(tokens: &[String]) -> Result<QueryType, ParseError> {
        if tokens.is_empty() {
            return Err(ParseError::EmptyQuery);
        }

        let first = tokens[0].to_uppercase();

        match first.as_str() {
            "SELECT" => Self::parse_select(tokens),
            "FILTER" => Self::parse_filter(tokens),
            "AGGREGATE" => Self::parse_aggregate(tokens),
            "JOIN" => Self::parse_join(tokens),
            "SEARCH" => Self::parse_search(tokens),
            _ => Err(ParseError::UnknownKeyword(first)),
        }
    }

    fn parse_select(tokens: &[String]) -> Result<QueryType, ParseError> {
        // Simple SELECT parser
        let mut fields = vec!["*".to_string()];
        let mut from = String::new();
        let mut where_clause = None;
        let mut order_by = None;
        let mut limit = None;
        let mut offset = None;

        let mut i = 1;
        while i < tokens.len() {
            match tokens[i].to_uppercase().as_str() {
                "FROM" => {
                    i += 1;
                    if i < tokens.len() {
                        from = tokens[i].clone();
                    }
                }
                "WHERE" => {
                    // Parse WHERE clause
                    i += 1;
                    let mut condition_tokens = Vec::new();
                    while i < tokens.len() && !["ORDER", "LIMIT", "OFFSET"].contains(&tokens[i].to_uppercase().as_str()) {
                        condition_tokens.push(tokens[i].clone());
                        i += 1;
                    }
                    where_clause = Some(Condition::Equals {
                        field: "placeholder".to_string(),
                        value: OmniValue::Bool(true),
                    });
                    continue;
                }
                "ORDER" => {
                    i += 1;
                    if i < tokens.len() && tokens[i].to_uppercase() == "BY" {
                        i += 1;
                        if i < tokens.len() {
                            let field = tokens[i].clone();
                            i += 1;
                            let direction = if i < tokens.len() && tokens[i].to_uppercase() == "DESC" {
                                OrderDirection::Descending
                            } else {
                                OrderDirection::Ascending
                            };
                            order_by = Some((field, direction));
                        }
                    }
                    continue;
                }
                "LIMIT" => {
                    i += 1;
                    if i < tokens.len() {
                        if let Ok(n) = tokens[i].parse() {
                            limit = Some(n);
                        }
                    }
                }
                "OFFSET" => {
                    i += 1;
                    if i < tokens.len() {
                        if let Ok(n) = tokens[i].parse() {
                            offset = Some(n);
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }

        Ok(QueryType::Select {
            fields,
            from,
            where_clause,
            order_by,
            limit,
            offset,
        })
    }

    fn parse_filter(tokens: &[String]) -> Result<QueryType, ParseError> {
        // Simple FILTER parser
        let mut from = String::new();
        let mut conditions = Vec::new();

        let mut i = 1;
        while i < tokens.len() {
            if tokens[i].to_uppercase() == "WHERE" {
                from = if i > 1 { tokens[i - 1].clone() } else { "data".to_string() };
                conditions.push(Condition::Equals {
                    field: "placeholder".to_string(),
                    value: OmniValue::Bool(true),
                });
            }
            i += 1;
        }

        Ok(QueryType::Filter { from, conditions })
    }

    fn parse_aggregate(tokens: &[String]) -> Result<QueryType, ParseError> {
        // Simple AGGREGATE parser
        let operations = vec![AggregateOp::Count];
        let from = if tokens.len() > 1 {
            tokens[1].clone()
        } else {
            "data".to_string()
        };

        Ok(QueryType::Aggregate {
            operations,
            from,
            group_by: None,
            having: None,
        })
    }

    fn parse_join(_tokens: &[String]) -> Result<QueryType, ParseError> {
        // Placeholder for JOIN parsing
        Err(ParseError::NotImplemented("JOIN parsing".to_string()))
    }

    fn parse_search(tokens: &[String]) -> Result<QueryType, ParseError> {
        // Simple SEARCH parser
        let mut terms = Vec::new();
        let mut in_fields = Vec::new();

        let mut i = 1;
        while i < tokens.len() {
            match tokens[i].to_uppercase().as_str() {
                "FOR" => {
                    i += 1;
                    if i < tokens.len() {
                        terms.push(tokens[i].trim_matches('"').to_string());
                    }
                }
                "IN" => {
                    i += 1;
                    while i < tokens.len() && tokens[i] != "," {
                        in_fields.push(tokens[i].clone());
                        i += 1;
                    }
                    continue;
                }
                _ => {}
            }
            i += 1;
        }

        Ok(QueryType::Search {
            terms,
            in_fields,
            weights: None,
        })
    }
}

/// Query Executor
pub struct QueryExecutor;

impl QueryExecutor {
    pub fn execute(query: &QueryType, data: &[OmniValue]) -> Result<QueryResult, ExecutionError> {
        let start = std::time::Instant::now();

        let rows = match query {
            QueryType::Select { fields, where_clause, order_by, limit, offset, .. } => {
                Self::execute_select(data, fields, where_clause.as_ref(), order_by.as_ref(), *limit, *offset)?
            }
            QueryType::Filter { conditions, .. } => {
                Self::execute_filter(data, conditions)?
            }
            QueryType::Aggregate { operations, group_by, .. } => {
                Self::execute_aggregate(data, operations, group_by.as_ref())?
            }
            _ => return Err(ExecutionError::NotImplemented("Query type not implemented".to_string())),
        };

        let count = rows.len();
        let execution_time_ms = start.elapsed().as_millis();

        Ok(QueryResult {
            rows,
            count,
            execution_time_ms,
        })
    }

    fn execute_select(
        data: &[OmniValue],
        _fields: &[String],
        _where_clause: Option<&Condition>,
        _order_by: Option<&(String, OrderDirection)>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<OmniValue>, ExecutionError> {
        let mut result = data.to_vec();

        if let Some(offset) = offset {
            if offset < result.len() {
                result = result[offset..].to_vec();
            } else {
                result.clear();
            }
        }

        if let Some(limit) = limit {
            result.truncate(limit);
        }

        Ok(result)
    }

    fn execute_filter(
        data: &[OmniValue],
        _conditions: &[Condition],
    ) -> Result<Vec<OmniValue>, ExecutionError> {
        // Simple filter - return all for now
        Ok(data.to_vec())
    }

    fn execute_aggregate(
        data: &[OmniValue],
        operations: &[AggregateOp],
        _group_by: Option<&Vec<String>>,
    ) -> Result<Vec<OmniValue>, ExecutionError> {
        let mut result = HashMap::new();

        for op in operations {
            match op {
                AggregateOp::Count => {
                    result.insert("count".to_string(), OmniValue::Integer(data.len() as i64));
                }
                AggregateOp::Sum(_field) => {
                    result.insert("sum".to_string(), OmniValue::Integer(0));
                }
                AggregateOp::Avg(_field) => {
                    result.insert("avg".to_string(), OmniValue::Float(0.0));
                }
                AggregateOp::Min(_field) => {
                    result.insert("min".to_string(), OmniValue::Null);
                }
                AggregateOp::Max(_field) => {
                    result.insert("max".to_string(), OmniValue::Null);
                }
                AggregateOp::StdDev(_field) => {
                    result.insert("stddev".to_string(), OmniValue::Float(0.0));
                }
            }
        }

        Ok(vec![OmniValue::Object(result)])
    }
}

/// Parse Errors
#[derive(Debug)]
pub enum ParseError {
    EmptyQuery,
    UnknownKeyword(String),
    SyntaxError(String),
    NotImplemented(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseError::EmptyQuery => write!(f, "Empty query"),
            ParseError::UnknownKeyword(kw) => write!(f, "Unknown keyword: {}", kw),
            ParseError::SyntaxError(msg) => write!(f, "Syntax error: {}", msg),
            ParseError::NotImplemented(feature) => write!(f, "Not implemented: {}", feature),
        }
    }
}

/// Execution Errors
#[derive(Debug)]
pub enum ExecutionError {
    InvalidQuery(String),
    NotImplemented(String),
    ExecutionFailed(String),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ExecutionError::InvalidQuery(msg) => write!(f, "Invalid query: {}", msg),
            ExecutionError::NotImplemented(feature) => write!(f, "Not implemented: {}", feature),
            ExecutionError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oql_tokenize() {
        let tokens = OqlParser::tokenize("SELECT * FROM users WHERE id = 1").unwrap();
        assert!(!tokens.is_empty());
        assert_eq!(tokens[0], "SELECT");
    }

    #[test]
    fn test_oql_parse_select() {
        let query = OqlParser::parse("SELECT * FROM users").unwrap();
        match query {
            QueryType::Select { from, .. } => {
                assert_eq!(from, "users");
            }
            _ => panic!("Expected SELECT query"),
        }
    }

    #[test]
    fn test_query_executor() {
        let data = vec![
            OmniValue::Integer(1),
            OmniValue::Integer(2),
            OmniValue::Integer(3),
        ];

        let query = QueryType::Select {
            fields: vec!["*".to_string()],
            from: "data".to_string(),
            where_clause: None,
            order_by: None,
            limit: Some(2),
            offset: None,
        };

        let result = QueryExecutor::execute(&query, &data).unwrap();
        assert_eq!(result.count, 2);
    }
}
