//! Schema Evolution Engine
//!
//! Migrations are Aether functions, versioned and tested alongside application logic.

pub mod engine;

pub use engine::{Migration, MigrationEngine, MigrationState};

/// Trait for implementing a migration
pub trait MigrationOp: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> u32;
    fn depends_on(&self) -> Option<u32>;
    fn up(&self) -> String;
    fn down(&self) -> String;
}
