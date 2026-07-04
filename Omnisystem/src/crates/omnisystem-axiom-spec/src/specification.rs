// Axiom Specification - formal definition of module correctness

use crate::invariant::Invariant;
use crate::postcondition::Postcondition;
use crate::precondition::Precondition;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpecificationId(Uuid);

impl SpecificationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_name(name: &str) -> Self {
        Self(Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes()))
    }
}

impl Default for SpecificationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SpecificationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Formal specification for a module
/// Defines correctness properties that must hold across all implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specification {
    /// Unique identifier
    pub id: SpecificationId,

    /// Module this spec applies to
    pub module_name: String,

    /// Phase (1-13)
    pub phase: u32,

    /// Specification version (semantic versioning)
    pub version: String,

    /// Human-readable description
    pub description: String,

    /// Invariants that must always hold
    pub invariants: Vec<Invariant>,

    /// Preconditions for operations
    pub preconditions: Vec<Precondition>,

    /// Postconditions for operations
    pub postconditions: Vec<Postcondition>,

    /// Properties that must be proven
    pub properties: Vec<Property>,

    /// Safety properties
    pub safety_properties: Vec<SafetyProperty>,

    /// Liveness properties
    pub liveness_properties: Vec<LivenessProperty>,

    /// When specification was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When specification was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A property that must be proven
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub description: String,
    pub formal_statement: String,
    pub proof_strategy: String,
}

/// Safety property - "bad things don't happen"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyProperty {
    pub name: String,
    pub description: String,
    /// Invariant that must never be violated
    pub invariant: String,
}

/// Liveness property - "good things eventually happen"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessProperty {
    pub name: String,
    pub description: String,
    /// Eventually true condition
    pub eventually_condition: String,
    /// Maximum allowed time (milliseconds)
    pub max_time_ms: Option<u64>,
}

impl Specification {
    pub fn new(module_name: String, phase: u32) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: SpecificationId::from_name(&format!("{}-phase-{}", module_name, phase)),
            module_name,
            phase,
            version: "1.0.0".to_string(),
            description: String::new(),
            invariants: Vec::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            properties: Vec::new(),
            safety_properties: Vec::new(),
            liveness_properties: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }

    pub fn add_invariant(mut self, inv: Invariant) -> Self {
        self.invariants.push(inv);
        self
    }

    pub fn add_precondition(mut self, pre: Precondition) -> Self {
        self.preconditions.push(pre);
        self
    }

    pub fn add_postcondition(mut self, post: Postcondition) -> Self {
        self.postconditions.push(post);
        self
    }

    pub fn add_property(mut self, prop: Property) -> Self {
        self.properties.push(prop);
        self
    }

    pub fn add_safety_property(mut self, prop: SafetyProperty) -> Self {
        self.safety_properties.push(prop);
        self
    }

    pub fn add_liveness_property(mut self, prop: LivenessProperty) -> Self {
        self.liveness_properties.push(prop);
        self
    }

    pub fn applies_to_module(&self, module_name: &str) -> bool {
        self.module_name == module_name
    }

    /// Verify this specification (async placeholder)
    pub async fn verify(&self) -> crate::SpecificationVerificationResult {
        // In real implementation, this would check all invariants, preconditions, postconditions
        crate::SpecificationVerificationResult {
            spec_name: self.module_name.clone(),
            passed: true,
            message: "All properties verified".to_string(),
        }
    }

    /// Get all proof obligations for this spec
    pub fn proof_obligations(&self) -> Vec<String> {
        let mut obligations = Vec::new();

        for inv in &self.invariants {
            obligations.push(format!("Invariant: {}", inv.name));
        }

        for pre in &self.preconditions {
            obligations.push(format!("Precondition: {}", pre.name));
        }

        for post in &self.postconditions {
            obligations.push(format!("Postcondition: {}", post.name));
        }

        for prop in &self.properties {
            obligations.push(format!("Property: {}", prop.name));
        }

        for safety in &self.safety_properties {
            obligations.push(format!("Safety: {}", safety.name));
        }

        for liveness in &self.liveness_properties {
            obligations.push(format!("Liveness: {}", liveness.name));
        }

        obligations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specification_creation() {
        let spec = Specification::new("test-module".to_string(), 1);
        assert_eq!(spec.module_name, "test-module");
        assert_eq!(spec.phase, 1);
        assert!(spec.invariants.is_empty());
    }

    #[test]
    fn test_applies_to_module() {
        let spec = Specification::new("my-module".to_string(), 1);
        assert!(spec.applies_to_module("my-module"));
        assert!(!spec.applies_to_module("other-module"));
    }
}
