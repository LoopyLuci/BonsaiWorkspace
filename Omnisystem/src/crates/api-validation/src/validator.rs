use crate::{ApiRequest, ValidationError, ValidationResult as ValidationResultType, ValidationRule};
use dashmap::DashMap;
use std::sync::Arc;

pub struct RequestValidator {
    rules: Arc<DashMap<String, Vec<ValidationRule>>>,
}

impl RequestValidator {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
        }
    }

    pub fn register_rules(&self, endpoint: &str, rules: Vec<ValidationRule>) {
        self.rules.insert(endpoint.to_string(), rules);
    }

    pub async fn validate(&self, request: &ApiRequest) -> crate::ValidationResult<()> {
        if request.endpoint.is_empty() {
            return Err(ValidationError::InvalidRequest);
        }

        if let Some(rules) = self.rules.get(&request.endpoint) {
            let mut errors = Vec::new();

            for rule in rules.iter() {
                if rule.required && request.body.is_empty() {
                    errors.push(format!("Field {} is required", rule.field));
                }

                if let Some(min_len) = rule.min_length {
                    if request.body.len() < min_len {
                        errors.push(format!(
                            "Field {} must be at least {} characters",
                            rule.field, min_len
                        ));
                    }
                }

                if let Some(max_len) = rule.max_length {
                    if request.body.len() > max_len {
                        errors.push(format!(
                            "Field {} must be at most {} characters",
                            rule.field, max_len
                        ));
                    }
                }
            }

            if !errors.is_empty() {
                return Err(ValidationError::ValidationFailed);
            }
        }

        Ok(())
    }

    pub fn is_some(&self) -> bool {
        true
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for RequestValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_register_rules() {
        let validator = RequestValidator::new();
        let rules = vec![ValidationRule {
            field: "name".to_string(),
            rule_type: "string".to_string(),
            required: true,
            min_length: Some(1),
            max_length: Some(100),
            pattern: None,
        }];

        validator.register_rules("/api/users", rules);
        assert_eq!(validator.rule_count(), 1);
    }

    #[tokio::test]
    async fn test_validate_empty_endpoint() {
        let validator = RequestValidator::new();
        let request = ApiRequest {
            request_id: "1".to_string(),
            endpoint: "".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: "{}".to_string(),
            query_params: HashMap::new(),
        };

        let result = validator.validate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_valid_request() {
        let validator = RequestValidator::new();
        let request = ApiRequest {
            request_id: "1".to_string(),
            endpoint: "/api/users".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: "test body with content".to_string(),
            query_params: HashMap::new(),
        };

        let result = validator.validate(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_with_rules() {
        let validator = RequestValidator::new();
        let rules = vec![ValidationRule {
            field: "body".to_string(),
            rule_type: "string".to_string(),
            required: true,
            min_length: Some(5),
            max_length: Some(100),
            pattern: None,
        }];

        validator.register_rules("/api/test", rules);

        let request = ApiRequest {
            request_id: "1".to_string(),
            endpoint: "/api/test".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: "hi".to_string(),
            query_params: HashMap::new(),
        };

        let result = validator.validate(&request).await;
        assert!(result.is_err());
    }
}
