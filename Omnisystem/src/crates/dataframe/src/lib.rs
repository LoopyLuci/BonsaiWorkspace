//! `dataframe` — an eager + lazy DataFrame library wrapping `polars`.
//!
//! Exposes `BonsaiFrame` (eager) and `BonsaiLazyFrame` (lazy) alongside a
//! serialisable operation DSL (`ops`) that can travel across an IPC boundary
//! (e.g. Tauri) as JSON and be evaluated against a frame.

pub mod error;
pub mod frame;
pub mod io;
pub mod lazy;
pub mod ops;
pub mod types;

pub use error::{DfError, DfResult};
pub use frame::BonsaiFrame;
pub use lazy::BonsaiLazyFrame;
pub use ops::{AggExpr, FilterExpr, Scalar, SortSpec};
