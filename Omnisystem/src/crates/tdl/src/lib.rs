//! `tdl` — Training Data Library: a SQLite-backed dataset/example versioning
//! system with JSONL and Parquet export.

pub mod db;
pub mod error;
pub mod library;
pub mod models;

pub use db::TrainingDataDb;
pub use error::{Result, TdlError};
pub use library::{ExportFormat, TrainingDataLibrary};
pub use models::{Example, Metadata, Version, VersionInfo};
