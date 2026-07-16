//! security-hardening: SBOM generation, secret scanning, supply-chain
//! artifact verification, vulnerability matching, and AES-256-GCM
//! encryption at rest / key management.

pub mod encryption;
pub mod error;
pub mod sbom;
pub mod secret_scanner;
pub mod supply_chain;
pub mod vulnerability;

pub use encryption::{EncryptionManager, KeyManager};
pub use error::{Result, SecurityError};
pub use sbom::{Component, Hash, Sbom, SbomGenerator, Service};
pub use secret_scanner::{SecretFinding, SecretScanner};
pub use supply_chain::{ArtifactVerification, Provenance, SupplyChainVerifier};
pub use vulnerability::{Vulnerability, VulnerabilityScanner};
