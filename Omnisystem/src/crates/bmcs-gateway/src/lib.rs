//! BMCS Gateway - Bonsai Medical Companion System safety pipeline
//!
//! A layered safety gateway for a medical/mental-health support AI:
//! - L0 [`safety::InputSanitizer`]: strips PII, detects adversarial prompts
//! - L1 [`safety::ContextClassifier`]: classifies queries into a [`ResponseTier`]
//! - L3+L5 [`axiom::AxiomVerifier`]: verifies AI responses against the Seven
//!   Laws of Medical AI (non-maleficence, never diagnose, never prescribe,
//!   respect autonomy, ...)
//! - L4 [`response::ResponseBuilder`]: assembles the final response with the
//!   mandatory tier-appropriate disclaimer
//! - L6 [`fallback::FallbackSystem`]: pre-approved, ethicist-reviewed safe
//!   responses used when no other path can produce a safe answer

pub mod axiom;
pub mod core;
pub mod fallback;
pub mod response;
pub mod safety;
pub mod types;

pub use axiom::{AxiomVerifier, MedicalLawViolation, VerificationResult, ViolationSeverity};
pub use core::{BMCSContext, BMCSResponse, ClassificationResult, Message, ResponseTier, Vitals};
pub use fallback::FallbackSystem;
pub use response::ResponseBuilder;
pub use safety::{ContextClassifier, InputSanitizer};
pub use types::{ConfidenceScore, EmotionalState, MedicalLaw};
