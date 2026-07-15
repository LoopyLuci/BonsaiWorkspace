//! App Manager Security
//!
//! Per-app sandboxing, permission management, signature verification, and
//! audit logging for the Omnisystem app management ecosystem.

pub mod audit_logger;
pub mod error;
pub mod permission_manager;
pub mod sandbox_manager;
pub mod signature_verifier;
pub mod ull_wrapper;

pub use audit_logger::{AuditEvent, AuditLogger};
pub use error::{Result, SecurityError};
pub use permission_manager::{Permission, PermissionManager, ResourceLimits, SandboxLevel, SecurityPolicy};
pub use sandbox_manager::{SandboxConfig, SandboxManager};
pub use signature_verifier::SignatureVerifier;
pub use ull_wrapper::register_with_ull;
