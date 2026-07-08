//! Output parsing module that converts raw model output to structured terms
//!
//! Supports multiple formats (JSON, plain text, structured terms) with deterministic
//! parsing and type information preservation.

use crate::errors::{VerifierError, VerifyResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Object component of a claim (can be string, numeric, or temporal)
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ClaimObject {
    String(String),
    Number(i64),
    Decimal(String),
    Date(String),
    Boolean(bool),
}

impl ClaimObject {
    pub fn is_numeric(&self) -> bool {
        matches!(self, ClaimObject::Number(_) | ClaimObject::Decimal(_))
    }

    pub fn is_temporal(&self) -> bool {
        matches!(self, ClaimObject::Date(_))
    }
}

/// Supported output formats
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum OutputFormat {
    /// JSON format (parsed into serde_json::Value)
    Json,
    /// Plain text format (string content)
    PlainText,
    /// Structured Omni-ABI term format
    StructuredTerm,
    /// Protocol Buffer serialized format (basic support)
    ProtoBuf,
}

impl OutputFormat {
    /// Infer format from output content
    fn infer(content: &str) -> Self {
        // Try to detect format based on content patterns
        let trimmed = content.trim();

        // Check for JSON
        if (trimmed.starts_with('{') || trimmed.starts_with('[')) && trimmed.ends_with(['}', ']']) {
            return OutputFormat::Json;
        }

        // Check for structured term (e.g., Term::String("..."), Term::Number(42))
        if trimmed.contains("Term::") {
            return OutputFormat::StructuredTerm;
        }

        // Default to plain text
        OutputFormat::PlainText
    }
}

/// A structured term following Omni-ABI conventions
///
/// Represents typed, structured output from the model with full type information
/// preserved for validation and verification.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum Term {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Integer(i64),
    /// Floating-point value (stored as string to avoid precision issues)
    Float(String),
    /// String value
    String(String),
    /// Array of terms
    Array(Vec<Term>),
    /// Object (map) of terms
    Object(Vec<(String, Term)>),
    /// Custom type with variant and payload
    Custom {
        type_name: String,
        variant: String,
        payload: Option<Box<Term>>,
    },
}

impl Term {
    /// Get the type name of this term
    pub fn type_name(&self) -> &'static str {
        match self {
            Term::Null => "Null",
            Term::Bool(_) => "Bool",
            Term::Integer(_) => "Integer",
            Term::Float(_) => "Float",
            Term::String(_) => "String",
            Term::Array(_) => "Array",
            Term::Object(_) => "Object",
            Term::Custom { .. } => "Custom",
        }
    }

    /// Check if this term is numeric
    pub fn is_numeric(&self) -> bool {
        matches!(self, Term::Integer(_) | Term::Float(_))
    }

    /// Check if this term is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Term::String(_))
    }

    /// Check if this term is a collection
    pub fn is_collection(&self) -> bool {
        matches!(self, Term::Array(_) | Term::Object(_))
    }

    /// Extract string value if possible
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Term::String(s) => Some(s),
            _ => None,
        }
    }

    /// Extract integer value if possible
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Term::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// Flatten to JSON value for schema validation
    pub fn to_json(&self) -> Value {
        match self {
            Term::Null => Value::Null,
            Term::Bool(b) => Value::Bool(*b),
            Term::Integer(i) => Value::Number((*i).into()),
            Term::Float(f) => {
                // Parse float carefully
                if let Ok(num) = f.parse::<f64>() {
                    if let Some(n) = serde_json::Number::from_f64(num) {
                        Value::Number(n)
                    } else {
                        Value::String(f.clone())
                    }
                } else {
                    Value::String(f.clone())
                }
            }
            Term::String(s) => Value::String(s.clone()),
            Term::Array(terms) => Value::Array(terms.iter().map(|t| t.to_json()).collect()),
            Term::Object(pairs) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in pairs {
                    obj.insert(k.clone(), v.to_json());
                }
                Value::Object(obj)
            }
            Term::Custom {
                type_name,
                variant,
                payload,
            } => {
                let mut obj = serde_json::Map::new();
                obj.insert("_type".to_string(), Value::String(type_name.clone()));
                obj.insert("_variant".to_string(), Value::String(variant.clone()));
                if let Some(p) = payload {
                    obj.insert("_payload".to_string(), p.to_json());
                }
                Value::Object(obj)
            }
        }
    }
}

/// A parsed output with full type information and format metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedOutput {
    /// Original content (up to 10KB)
    pub content: String,
    /// Detected format of the output
    pub format: OutputFormat,
    /// Structured term representation
    pub term: Term,
    /// Optional axiom proof (JSON representation)
    pub axiom_proof: Option<serde_json::Value>,
    /// Length of original content
    pub content_length: usize,
    /// Whether parsing was deterministic (no ambiguity)
    pub is_deterministic: bool,
    /// Type constraints found in the output
    pub type_constraints: Vec<String>,
}

/// Output parser: converts raw model output to structured terms
pub struct OutputParser {
    max_content_length: usize,
}

impl OutputParser {
    /// Create a new output parser with default settings
    pub fn new() -> Self {
        Self {
            max_content_length: 10 * 1024, // 10 KB default
        }
    }

    /// Create a parser with custom max content length
    pub fn with_max_length(max_length: usize) -> Self {
        Self {
            max_content_length: max_length,
        }
    }

    /// Parse raw output into a structured term
    ///
    /// Deterministically converts the output into a typed term while preserving
    /// all type information. Supports JSON, plain text, and structured term formats.
    pub fn parse(&self, raw_output: &str) -> VerifyResult<ParsedOutput> {
        // Check length constraint
        if raw_output.len() > self.max_content_length {
            return Err(VerifierError::ParseError {
                reason: format!(
                    "output length {} exceeds limit {}",
                    raw_output.len(),
                    self.max_content_length
                ),
            });
        }

        let trimmed = raw_output.trim();
        let format = OutputFormat::infer(trimmed);

        // Parse based on format
        let (term, type_constraints, is_deterministic) = match format {
            OutputFormat::Json => self.parse_json(trimmed)?,
            OutputFormat::PlainText => self.parse_plaintext(trimmed)?,
            OutputFormat::StructuredTerm => self.parse_structured_term(trimmed)?,
            OutputFormat::ProtoBuf => self.parse_protobuf(trimmed)?,
        };

        // Extract optional axiom proof if present
        let axiom_proof = self.extract_axiom_proof(trimmed);

        Ok(ParsedOutput {
            content: raw_output.to_string(),
            format,
            term,
            axiom_proof,
            content_length: raw_output.len(),
            is_deterministic,
            type_constraints,
        })
    }

    /// Parse JSON format
    fn parse_json(&self, json_str: &str) -> VerifyResult<(Term, Vec<String>, bool)> {
        let value: Value = serde_json::from_str(json_str)
            .map_err(|e| VerifierError::ParseError {
                reason: format!("invalid JSON: {}", e),
            })?;

        let term = self.value_to_term(&value)?;
        let type_constraints = self.extract_type_constraints(&value);

        Ok((term, type_constraints, true)) // JSON parsing is deterministic
    }

    /// Parse plain text format
    fn parse_plaintext(&self, text: &str) -> VerifyResult<(Term, Vec<String>, bool)> {
        // Plain text is stored as a single string term
        let term = Term::String(text.to_string());
        Ok((term, vec![], true))
    }

    /// Parse structured term format (e.g., from Omni-ABI)
    fn parse_structured_term(&self, term_str: &str) -> VerifyResult<(Term, Vec<String>, bool)> {
        // Very simple pattern matching for demonstration
        // In production, would use a full parser

        if term_str.contains("Term::String(") {
            // Extract string content
            if let Some(start) = term_str.find("\"") {
                if let Some(end) = term_str.rfind("\"") {
                    if start < end {
                        let content = &term_str[start + 1..end];
                        return Ok((Term::String(content.to_string()), vec![], true));
                    }
                }
            }
        }

        if term_str.contains("Term::Integer(") {
            if let Some(start) = term_str.find('(') {
                if let Some(end) = term_str.find(')') {
                    if start < end {
                        let num_str = &term_str[start + 1..end];
                        if let Ok(n) = num_str.parse::<i64>() {
                            return Ok((Term::Integer(n), vec!["Integer".to_string()], true));
                        }
                    }
                }
            }
        }

        // Fallback: treat as plain string
        Ok((Term::String(term_str.to_string()), vec![], false))
    }

    /// Parse protobuf format (basic support)
    fn parse_protobuf(&self, _pb_data: &str) -> VerifyResult<(Term, Vec<String>, bool)> {
        // For now, treat as opaque string term
        // Real implementation would decode protobuf
        Ok((
            Term::String(_pb_data.to_string()),
            vec!["ProtoBuf".to_string()],
            false,
        ))
    }

    /// Convert serde_json::Value to Term
    fn value_to_term(&self, value: &Value) -> VerifyResult<Term> {
        match value {
            Value::Null => Ok(Term::Null),
            Value::Bool(b) => Ok(Term::Bool(*b)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Term::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Term::Float(f.to_string()))
                } else {
                    Ok(Term::String(n.to_string()))
                }
            }
            Value::String(s) => Ok(Term::String(s.clone())),
            Value::Array(arr) => {
                let terms: VerifyResult<Vec<Term>> =
                    arr.iter().map(|v| self.value_to_term(v)).collect();
                Ok(Term::Array(terms?))
            }
            Value::Object(obj) => {
                let pairs = obj
                    .iter()
                    .map(|(k, v)| {
                        self.value_to_term(v)
                            .map(|term| (k.clone(), term))
                    })
                    .collect::<VerifyResult<Vec<_>>>()?;
                Ok(Term::Object(pairs))
            }
        }
    }

    /// Extract type constraints from JSON structure
    fn extract_type_constraints(&self, value: &Value) -> Vec<String> {
        let mut constraints = vec![];

        if let Value::Object(obj) = value {
            for (key, _val) in obj.iter() {
                // Look for common constraint patterns
                if key.ends_with("_type") || key.ends_with("Type") {
                    constraints.push(format!("type_annotation:{}", key));
                }
                if key.ends_with("_required") || key.ends_with("Required") {
                    constraints.push(format!("required:{}", key));
                }
                if key.ends_with("_min") || key.ends_with("Min") {
                    constraints.push(format!("min_value:{}", key));
                }
                if key.ends_with("_max") || key.ends_with("Max") {
                    constraints.push(format!("max_value:{}", key));
                }
            }
        }

        constraints
    }

    /// Extract optional axiom proof from output
    fn extract_axiom_proof(&self, content: &str) -> Option<serde_json::Value> {
        // Look for axiom proof markers
        if let Some(start) = content.find("\"axiom_proof\"") {
            if let Some(colon) = content[start..].find(':') {
                let proof_start = start + colon + 1;
                if let Some(end) = content[proof_start..].find('}') {
                    let proof_str = &content[proof_start..proof_start + end + 1];
                    return serde_json::from_str(proof_str).ok();
                }
            }
        }

        None
    }
}

impl Default for OutputParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_type_names() {
        assert_eq!(Term::Null.type_name(), "Null");
        assert_eq!(Term::Bool(true).type_name(), "Bool");
        assert_eq!(Term::Integer(42).type_name(), "Integer");
        assert_eq!(Term::String("hello".to_string()).type_name(), "String");
    }

    #[test]
    fn test_term_predicates() {
        assert!(Term::Integer(42).is_numeric());
        assert!(!Term::String("42".to_string()).is_numeric());
        assert!(Term::String("hello".to_string()).is_string());
        assert!(Term::Array(vec![]).is_collection());
    }

    #[test]
    fn test_parse_json() {
        let parser = OutputParser::new();
        let result = parser.parse(r#"{"name": "Alice", "age": 30}"#);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.format, OutputFormat::Json);
        assert!(matches!(parsed.term, Term::Object(_)));
    }

    #[test]
    fn test_parse_plaintext() {
        let parser = OutputParser::new();
        let result = parser.parse("This is plain text output");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.format, OutputFormat::PlainText);
        assert!(matches!(parsed.term, Term::String(_)));
    }

    #[test]
    fn test_parse_json_array() {
        let parser = OutputParser::new();
        let result = parser.parse("[1, 2, 3]");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(matches!(parsed.term, Term::Array(_)));
    }

    #[test]
    fn test_parse_length_limit() {
        let parser = OutputParser::with_max_length(10);
        let long_text = "a".repeat(100);
        let result = parser.parse(&long_text);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VerifierError::ParseError { .. }));
    }

    #[test]
    fn test_parse_invalid_json() {
        let parser = OutputParser::new();
        let result = parser.parse("{invalid json}");
        assert!(result.is_err());
    }

    #[test]
    fn test_term_to_json_conversion() {
        let term = Term::String("test".to_string());
        let json = term.to_json();
        assert_eq!(json, Value::String("test".to_string()));

        let term = Term::Integer(42);
        let json = term.to_json();
        assert_eq!(json, Value::Number(42.into()));
    }

    #[test]
    fn test_format_inference() {
        assert_eq!(OutputFormat::infer(r#"{"key": "value"}"#), OutputFormat::Json);
        assert_eq!(OutputFormat::infer("[1, 2, 3]"), OutputFormat::Json);
        assert_eq!(
            OutputFormat::infer("Plain text output"),
            OutputFormat::PlainText
        );
    }

    #[test]
    fn test_parse_nested_json() {
        let parser = OutputParser::new();
        let json = r#"{"user": {"name": "Alice", "age": 30}}"#;
        let result = parser.parse(json);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(matches!(parsed.term, Term::Object(_)));
    }

    #[test]
    fn test_parse_json_with_numbers() {
        let parser = OutputParser::new();
        let json = r#"{"int": 42, "float": 3.14}"#;
        let result = parser.parse(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_json_with_booleans() {
        let parser = OutputParser::new();
        let json = r#"{"active": true, "deleted": false}"#;
        let result = parser.parse(json);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(matches!(parsed.term, Term::Object(_)));
    }

    #[test]
    fn test_as_string_method() {
        let term = Term::String("hello".to_string());
        assert_eq!(term.as_string(), Some("hello"));

        let term = Term::Integer(42);
        assert_eq!(term.as_string(), None);
    }

    #[test]
    fn test_as_integer_method() {
        let term = Term::Integer(42);
        assert_eq!(term.as_integer(), Some(42));

        let term = Term::String("42".to_string());
        assert_eq!(term.as_integer(), None);
    }

    #[test]
    fn test_parse_empty_json() {
        let parser = OutputParser::new();
        let result = parser.parse("{}");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(matches!(parsed.term, Term::Object(_)));
    }

    #[test]
    fn test_parse_json_null() {
        let parser = OutputParser::new();
        let result = parser.parse("null");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(matches!(parsed.term, Term::Null));
    }

    #[test]
    fn test_deterministic_flag() {
        let parser = OutputParser::new();
        let result = parser.parse(r#"{"key": "value"}"#);
        assert!(result.unwrap().is_deterministic);

        let result = parser.parse("plain text");
        assert!(result.unwrap().is_deterministic);
    }

    #[test]
    fn test_type_constraints_extraction() {
        let parser = OutputParser::new();
        let json = r#"{"field_type": "string", "field_required": true}"#;
        let result = parser.parse(json);
        let parsed = result.unwrap();
        assert!(!parsed.type_constraints.is_empty());
    }
}
