//! Hot-reload runtime: swap live function implementations behind a
//! `Send + Sync` function-pointer table, with atomic transactional
//! rollback of any state that was mutated during a failed reload.

mod pointer_table;
mod runtime;
mod transaction;

pub use pointer_table::FunctionPointerTable;
pub use runtime::HotReloadRuntime;
pub use transaction::{AtomicTransaction, Snapshot, StateSnapshot};
