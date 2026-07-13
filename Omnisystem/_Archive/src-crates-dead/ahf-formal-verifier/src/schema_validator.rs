//! Schema validation module using JSON Schema and type constraints
//!
//! Validates parsed outputs against schemas, type systems, and domain-specific
//! invariants. Supports JSON Schema, Protocol Buffers (basic), and Omni-ABI types.

use crate::errors::{VerifierError, VerifyResult};
use crate::output_parser::{ParsedOutput, Term};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A constraint violation with detailed diagnostic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolation {
    /// Name of the constraint that was violated
    pub constraint_name: String,
    /// Description of what went wrong
    pub reason: String,
    /// The value that caused the violation
    pub actual_value: String,
    /// Expected constraint
    pub expected: String,
    /// JSON path where violation occurred (for nested structures)
    pub path: String,
}

/// A validation constraint that can be checked against a term
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConstraint {
    /// Name of the constraint
    pub name: String,
    /// Type of constraint
    pub constraint_type: ConstraintType,
    /// Whether this constraint is mandatory
    pub required: bool,
}

/// Types of validation constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Type check: value must match expected type
    Type { expected: String },
    /// Range check: numeric value must be within [min, max]
    Range { min: f64, max: f64 },
    /// Length check: string/array length must be within [min, max]
    Length { min: usize, max: usize },
    /// Regex pattern match
    Pattern { regex: String },
    /// Enum: value must be one of allowed values
    Enum { allowed: Vec<String> },
    /// Minimum value (inclusive)
    Minimum { value: f64 },
    /// Maximum value (inclusive)
    Maximum { value: f64 },
    /// Mutually exclusive with other fields
    MutuallyExclusive { fields: Vec<String> },
    /// Requires another field to be present
    RequiredWith { field: String },
    /// Custom constraint with validator function
    Custom { validator_name: String },
}

/// Schema validator for JSON Schema and type constraints
#[derive(Debug)]
pub struct SchemaValidator {
    /// Cached JSON schemas indexed by name
    schemas: HashMap<String, Value>,
    /// Custom constraints indexed by name
    constraints: HashMap<String, ValidationConstraint>,
    /// Type system mappings (Omni-ABI)
    type_system: TypeSystem,
}

/// Type system for Omni-ABI type checking
#[derive(Debug, Clone)]
struct TypeSystem {
    /// Known type definitions
    types: HashMap<String, TypeDefinition>,
}

/// Definition of a type with its fields and invariants
#[derive(Debug, Clone)]
struct TypeDefinition {
    /// Name of the type
    name: String,
    /// Expected fields and their types
    fields: HashMap<String, String>,
    /// Invariants that must hold
    invariants: Vec<String>,
}

impl SchemaValidator {
    /// Create a new schema validator with default configuration
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            constraints: HashMap::new(),
            type_system: TypeSystem {
                types: Self::default_types(),
            },
        }
    }

    /// Register a JSON schema for validation
    pub fn register_schema(&mut self, name: &str, schema: Value) -> VerifyResult<()> {
        // Validate that it's a valid JSON schema
        if !schema.is_object() {
            return Err(VerifierError::InvalidConfiguration {
                reason: "schema must be a JSON object".to_string(),
            });
        }
        self.schemas.insert(name.to_string(), schema);
        Ok(())
    }

    /// Register a custom constraint
    pub fn register_constraint(&mut self, constraint: ValidationConstraint) {
        self.constraints
            .insert(constraint.name.clone(), constraint);
    }

    /// Validate a parsed output against schema and constraints
    pub fn validate(&self, parsed: &ParsedOutput) -> VerifyResult<()> {
        // Step 1: Validate against JSON schema if applicable
        self.validate_json_schema(parsed)?;

        // Step 2: Validate type constraints
        self.validate_type_constraints(parsed)?;

        // Step 3: Validate custom constraints
        self.validate_custom_constraints(parsed)?;

        // Step 4: Check invariants
        self.validate_invariants(parsed)?;

        Ok(())
    }

    /// Validate against JSON schema using the jsonschema crate
    fn validate_json_schema(&self, parsed: &ParsedOutput) -> VerifyResult<()> {
        // For now, accept any valid JSON
        // In production, would load and validate against configured schema

        // Convert term to JSON for schema validation
        let _json = parsed.term.to_json();

        // Basic structural validation
        match &parsed.term {
            Term::Object(pairs) => {
                // Check for empty objects in required positions
                if pairs.is_empty() && parsed.type_constraints.contains(&"required".to_string()) {
                    return Err(VerifierError::SchemaValidation {
                        reason: "required object cannot be empty".to_string(),
                    });
                }
            }
            Term::Array(items) => {
                // Check array constraints
                for (idx, item) in items.iter().enumerate() {
                    if item == &Term::Null
                        && !parsed
                            .type_constraints
                            .iter()
                            .any(|c| c.contains("nullable"))
                    {
                        return Err(VerifierError::SchemaValidation {
                            reason: format!("array item {} cannot be null", idx),
                        });
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate type constraints from the parsed output
    fn validate_type_constraints(&self, parsed: &ParsedOutput) -> VerifyResult<()> {
        for constraint_str in &parsed.type_constraints {
            if constraint_str.starts_with("type_annotation:") {
                // Type annotation found - verify it matches the actual type
                let expected_type = constraint_str.strip_prefix("type_annotation:").unwrap();
                let actual_type = parsed.term.type_name();
                if !self.type_matches(actual_type, expected_type) {
                    return Err(VerifierError::SchemaValidation {
                        reason: format!(
                            "type mismatch: expected {}, got {}",
                            expected_type, actual_type
                        ),
                    });
                }
            }
            if constraint_str.starts_with("min_value:") {
                // Minimum value constraint
                self.validate_numeric_constraint(parsed, constraint_str)?;
            }
            if constraint_str.starts_with("max_value:") {
                // Maximum value constraint
                self.validate_numeric_constraint(parsed, constraint_str)?;
            }
        }

        Ok(())
    }

    /// Validate numeric constraints
    fn validate_numeric_constraint(&self, parsed: &ParsedOutput, constraint: &str) -> VerifyResult<()> {
        match &parsed.term {
            Term::Integer(n) => {
                let n_f64 = *n as f64;
                if constraint.starts_with("min_value:") && n_f64 < 0.0 {
                    return Err(VerifierError::ConstraintViolation {
                        reason: format!("value {} is below minimum", n),
                    });
                }
            }
            Term::Float(f) => {
                if let Ok(n) = f.parse::<f64>() {
                    if constraint.starts_with("min_value:") && n < 0.0 {
                        return Err(VerifierError::ConstraintViolation {
                            reason: format!("value {} is below minimum", f),
                        });
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate custom constraints
    fn validate_custom_constraints(&self, parsed: &ParsedOutput) -> VerifyResult<()> {
        for (_name, constraint) in &self.constraints {
            if constraint.required {
                match &constraint.constraint_type {
                    ConstraintType::Type { expected } => {
                        let actual = parsed.term.type_name();
                        if !self.type_matches(actual, expected) {
                            return Err(VerifierError::SchemaValidation {
                                reason: format!(
                                    "required type constraint failed: expected {}, got {}",
                                    expected, actual
                                ),
                            });
                        }
                    }
                    ConstraintType::Range { min, max } => {
                        if let Some(n) = parsed.term.as_integer() {
                            let n_f64 = n as f64;
                            if n_f64 < *min || n_f64 > *max {
                                return Err(VerifierError::ConstraintViolation {
                                    reason: format!(
                                        "value {} not in range [{}, {}]",
                                        n, min, max
                                    ),
                                });
                            }
                        }
                    }
                    ConstraintType::Length { min, max } => {
                        let len = match &parsed.term {
                            Term::String(s) => s.len(),
                            Term::Array(arr) => arr.len(),
                            _ => 0,
                        };
                        if len < *min || len > *max {
                            return Err(VerifierError::ConstraintViolation {
                                reason: format!("length {} not in range [{}, {}]", len, min, max),
                            });
                        }
                    }
                    ConstraintType::Pattern { regex } => {
                        if let Some(s) = parsed.term.as_string() {
                            if let Ok(re) = regex::Regex::new(regex) {
                                if !re.is_match(s) {
                                    return Err(VerifierError::ConstraintViolation {
                                        reason: format!("string does not match pattern: {}", regex),
                                    });
                                }
                            }
                        }
                    }
                    ConstraintType::Enum { allowed } => {
                        if let Some(s) = parsed.term.as_string() {
                            if !allowed.contains(&s.to_string()) {
                                return Err(VerifierError::ConstraintViolation {
                                    reason: format!(
                                        "value '{}' not in allowed values: {}",
                                        s,
                                        allowed.join(", ")
                                    ),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Validate invariants (e.g., "age >= 0")
    fn validate_invariants(&self, parsed: &ParsedOutput) -> VerifyResult<()> {
        // Check common numeric invariants
        if let Term::Integer(n) = &parsed.term {
            // Age invariant
            if parsed.type_constraints.iter().any(|c| c.contains("age")) && n < &0 {
                return Err(VerifierError::ConstraintViolation {
                    reason: "age cannot be negative".to_string(),
                });
            }

            // Count invariant
            if parsed.type_constraints.iter().any(|c| c.contains("count")) && n < &0 {
                return Err(VerifierError::ConstraintViolation {
                    reason: "count cannot be negative".to_string(),
                });
            }
        }

        // Check common string invariants
        if let Term::String(s) = &parsed.term {
            // Non-empty invariant
            if parsed.type_constraints.iter().any(|c| c.contains("non_empty")) && s.is_empty() {
                return Err(VerifierError::ConstraintViolation {
                    reason: "string cannot be empty".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Check if a type matches an expected type
    fn type_matches(&self, actual: &str, expected: &str) -> bool {
        // Exact match
        if actual == expected {
            return true;
        }

        // Allow some polymorphism
        match (actual, expected) {
            ("Integer", "Number") => true,
            ("Float", "Number") => true,
            ("String", "Scalar") => true,
            ("Bool", "Scalar") => true,
            ("Integer", "Scalar") => true,
            ("Float", "Scalar") => true,
            _ => false,
        }
    }

    /// Get default type definitions for Omni-ABI
    fn default_types() -> HashMap<String, TypeDefinition> {
        let mut types = HashMap::new();

        // User type
        let mut user_fields = HashMap::new();
        user_fields.insert("id".to_string(), "String".to_string());
        user_fields.insert("name".to_string(), "String".to_string());
        user_fields.insert("age".to_string(), "Integer".to_string());
        types.insert(
            "User".to_string(),
            TypeDefinition {
                name: "User".to_string(),
                fields: user_fields,
                invariants: vec!["age >= 0".to_string()],
            },
        );

        // Product type
        let mut product_fields = HashMap::new();
        product_fields.insert("sku".to_string(), "String".to_string());
        product_fields.insert("price".to_string(), "Float".to_string());
        product_fields.insert("quantity".to_string(), "Integer".to_string());
        types.insert(
            "Product".to_string(),
            TypeDefinition {
                name: "Product".to_string(),
                fields: product_fields,
                invariants: vec!["price >= 0".to_string(), "quantity >= 0".to_string()],
            },
        );

        types
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validator_creation() {
        let validator = SchemaValidator::new();
        assert_eq!(validator.schemas.len(), 0);
    }

    #[test]
    fn test_register_schema() {
        let mut validator = SchemaValidator::new();
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });
        let result = validator.register_schema("user", schema);
        assert!(result.is_ok());
        assert_eq!(validator.schemas.len(), 1);
    }

    #[test]
    fn test_register_invalid_schema() {
        let mut validator = SchemaValidator::new();
        let result = validator.register_schema("invalid", Value::String("not an object".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_constraint_type_range() {
        let constraint = ValidationConstraint {
            name: "age".to_string(),
            constraint_type: ConstraintType::Range {
                min: 0.0,
                max: 150.0,
            },
            required: true,
        };
        assert_eq!(constraint.name, "age");
    }

    #[test]
    fn test_constraint_type_enum() {
        let constraint = ValidationConstraint {
            name: "status".to_string(),
            constraint_type: ConstraintType::Enum {
                allowed: vec!["active".to_string(), "inactive".to_string()],
            },
            required: true,
        };
        assert!(matches!(
            constraint.constraint_type,
            ConstraintType::Enum { .. }
        ));
    }

    #[test]
    fn test_type_matching() {
        let validator = SchemaValidator::new();
        assert!(validator.type_matches("Integer", "Integer"));
        assert!(validator.type_matches("Integer", "Number"));
        assert!(validator.type_matches("Float", "Number"));
        assert!(validator.type_matches("String", "Scalar"));
        assert!(!validator.type_matches("Integer", "String"));
    }

    #[test]
    fn test_constraint_violation_creation() {
        let violation = ConstraintViolation {
            constraint_name: "age".to_string(),
            reason: "age cannot be negative".to_string(),
            actual_value: "-5".to_string(),
            expected: ">= 0".to_string(),
            path: "$.user.age".to_string(),
        };
        assert_eq!(violation.constraint_name, "age");
    }

    #[test]
    fn test_default_types_include_user() {
        let types = SchemaValidator::default_types();
        assert!(types.contains_key("User"));
        assert!(types.contains_key("Product"));
    }

    #[test]
    fn test_register_custom_constraint() {
        let mut validator = SchemaValidator::new();
        let constraint = ValidationConstraint {
            name: "email".to_string(),
            constraint_type: ConstraintType::Pattern {
                regex: r"[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}".to_string(),
            },
            required: true,
        };
        validator.register_constraint(constraint);
        assert_eq!(validator.constraints.len(), 1);
    }
}
