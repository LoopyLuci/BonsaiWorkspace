use crate::rules::static_rule::{StaticRule, PatternType, SeverityLevel};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Request to generate a rule from a natural language description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleGenerationRequest {
    pub description: String,
    pub language: String,
    pub severity: Option<String>,
    pub example_good: Option<String>,
    pub example_bad: Option<String>,
}

/// Response from AI rule generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleGenerationResponse {
    pub rule: StaticRule,
    pub confidence: f32,
    pub explanation: String,
}

/// Generate a linting rule from a natural language description using BonsAI.
pub async fn generate_rule(request: RuleGenerationRequest) -> Result<RuleGenerationResponse> {
    // TODO: Integrate with BonsAI API to generate rules
    // This is a placeholder implementation

    let severity = match request.severity.as_deref() {
        Some("error") => SeverityLevel::Error,
        Some("warning") => SeverityLevel::Warning,
        Some("hint") => SeverityLevel::Hint,
        Some("note") => SeverityLevel::Note,
        _ => SeverityLevel::Warning,
    };

    let rule = StaticRule {
        id: format!("generated-{}", uuid::Uuid::new_v4()),
        name: request.description.clone(),
        description: Some(request.description.clone()),
        languages: vec![request.language],
        pattern: String::from(r"\w+"),     // Placeholder pattern
        pattern_type: PatternType::Regex,
        message: format!("Violates rule: {}", request.description),
        severity,
        fix: None,
        enabled: true,
        tags: vec!["generated".to_string()],
        category: String::new(),
    };

    Ok(RuleGenerationResponse {
        rule,
        confidence: 0.65,
        explanation: "Generated from natural language description. Manual review recommended.".to_string(),
    })
}

/// Validate a generated rule by testing it against the provided examples.
pub async fn validate_rule(
    rule: &StaticRule,
    example_good: &str,
    example_bad: &str,
) -> Result<ValidationResult> {
    // TODO: Test rule against examples
    Ok(ValidationResult {
        passes_good: true,
        passes_bad: false,
        confidence: 0.8,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passes_good: bool,
    pub passes_bad: bool,
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rule_generation_placeholder() -> Result<()> {
        let request = RuleGenerationRequest {
            description: "Warn when a function has more than 5 parameters".to_string(),
            language: "rust".to_string(),
            severity: Some("warning".to_string()),
            example_good: Some("fn foo(a: i32, b: i32) {}".to_string()),
            example_bad: Some("fn foo(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}".to_string()),
        };

        let response = generate_rule(request).await?;
        assert_eq!(response.confidence, 0.65);
        assert!(!response.rule.id.is_empty());

        Ok(())
    }
}
