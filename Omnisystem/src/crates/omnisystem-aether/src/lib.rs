//! omnisystem-aether: actor-native database integration layer.
//!
//! Provides a type-safe schema/query system, reactive collections
//! (`LiveSet<T>`/`Live<T>`) that push deltas to subscribers, algebraic
//! effects for DB reads/writes, capability-based row-level security,
//! actor state persistence, and a schema migration engine — all
//! designed to sit under an actor runtime ([`actor`]).
//!
//! Note: this crate originally also shipped a `frontend.rs` implementing
//! a `language_system::LanguageFrontend` for a `core_ir`-based compiler
//! pipeline. Both `language_system` and `core_ir` are themselves still
//! archived, unrestored crates, so that adapter was dropped rather than
//! pulling in two more unaudited dependencies; the database/actor/
//! migration layers below don't depend on either and are independently
//! complete.

pub mod actor;
pub mod database;
pub mod error;
pub mod migrations;

pub use database::{DbContext, Entity};
pub use error::{Error, Result};
