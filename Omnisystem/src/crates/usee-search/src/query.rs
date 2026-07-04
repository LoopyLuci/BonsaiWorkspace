use crate::{Query, SearchError};

pub struct QueryParser {
    min_token_length: usize,
}

impl QueryParser {
    pub fn new() -> Self {
        Self {
            min_token_length: 2,
        }
    }

    pub fn parse(&self, query_text: &str) -> Result<Vec<String>, SearchError> {
        let tokens: Vec<String> = query_text
            .to_lowercase()
            .split_whitespace()
            .filter(|s| s.len() > self.min_token_length)
            .map(|s| s.to_string())
            .collect();
        
        if tokens.is_empty() {
            return Err(SearchError::QueryError("No valid tokens".to_string()));
        }
        
        Ok(tokens)
    }

    pub fn validate_query(&self, query: &Query) -> Result<(), SearchError> {
        if query.text.is_empty() {
            return Err(SearchError::QueryError("Empty query".to_string()));
        }
        if query.limit == 0 {
            return Err(SearchError::QueryError("Limit must be > 0".to_string()));
        }
        Ok(())
    }
}

impl Default for QueryParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_query_parser() {
        let parser = QueryParser::new();
        let tokens = parser.parse("hello world").unwrap();
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn test_validate_query() {
        let parser = QueryParser::new();
        let query = Query {
            text: "test".to_string(),
            limit: 10,
            offset: 0,
            filters: HashMap::new(),
        };
        assert!(parser.validate_query(&query).is_ok());
    }
}
