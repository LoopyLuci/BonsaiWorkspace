//! bkp
//!
//! BKP: a backup/model-package format bundling a base model, KMOD knowledge
//! modules, and LoRA/QLoRA adapters into a single zstd-compressed, optionally
//! Ed25519-signed archive.

pub mod builder;
pub mod core;
pub mod error;
pub mod loader;
pub mod manifest;
pub mod types;

pub use builder::BkpBuilder;
pub use core::Core;
pub use error::{BkpError, BkpResult, Error, Result};
pub use loader::BkpLoader;
pub use manifest::{AdapterInfo, BaseModelInfo, BkpManifest, FileHash, KmodInfo};
pub use types::State;
