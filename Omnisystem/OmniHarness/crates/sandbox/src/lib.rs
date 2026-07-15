//! Bonsai Enclave — universal dependency & environment manager.
//!
//! `manifest`/`lockfile` are the project's `enclave.toml`/`enclave.lock`,
//! `resolver` turns a manifest into a lockfile, `cas` is the BLAKE3
//! content-addressed store everything is cached under, `environment` creates
//! isolated per-project environments, `runtime` provisions language
//! runtimes, and `enclave` ties all of it together behind the `Enclave`
//! handle used by the CLI.

pub mod advisor;
pub mod cas;
pub mod enclave;
pub mod environment;
pub mod error;
pub mod lockfile;
pub mod manifest;
pub mod p2p;
pub mod resolver;
pub mod runtime;
pub mod sandbox;
pub mod types;

pub use cas::{ContentAddressedStore, ContentHash};
pub use enclave::{Enclave, EnclaveConfig};
pub use environment::{Environment, EnvironmentManager};
pub use error::{Error, Result as EnclaveResult};
pub use lockfile::{Lockfile, LockedPackage, LockedRuntime};
pub use manifest::{DependencySpec, Manifest, ProjectMetadata};
pub use resolver::DependencyResolver;
pub use runtime::{RuntimeDownloader, RuntimeEntry, RuntimeManifest};
