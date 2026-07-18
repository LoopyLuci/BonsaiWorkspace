//! Aether Database Layer — Actor-Native Database Integration
//!
//! Integrates AriaDB with Aether's actor model, providing:
//! - Type-safe schema definitions that are also Aether types
//! - Reactive queries (LiveSet<T>, Live<T>) that push to actors
//! - Algebraic effects for database operations (DbRead, DbWrite)
//! - Capability-based security and row-level policies
//! - Automatic persistence for actor state

pub mod schema;
pub mod query;
pub mod effects;
pub mod reactive;
pub mod capabilities;
pub mod persistence;

pub use schema::{Schema, EntityType, Field};
pub use query::{Query, QueryPlan};
pub use effects::{DbReadEffect, DbWriteEffect};
pub use reactive::{LiveSet, Live, SetDelta};
pub use capabilities::{DbCapability, CapabilityToken};
pub use persistence::{PersistenceManager, ActorState};

/// A database entity is a type that can be persisted and queried
pub trait Entity: Send + Sync {
    /// Get this entity's type name (maps to database table)
    fn entity_type() -> &'static str;

    /// Get this entity's unique ID
    fn id(&self) -> uuid::Uuid;

    /// Validate this entity against the schema
    fn validate(&self) -> Result<(), String>;
}

/// Database connection context for actors
pub struct DbContext {
    pub schema: Schema,
    pub persistence: PersistenceManager,
}

impl DbContext {
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            persistence: PersistenceManager::new(),
        }
    }
}
