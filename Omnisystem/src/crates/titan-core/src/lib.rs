//! titan-core: Clojure-inspired persistent data structures and concurrency
//! primitives for Rust.
//!
//! - [`vector::PersistentVector`] / [`hashmap::PersistentHashMap`]: immutable,
//!   structurally-shared collections (thin, tested wrappers over the `im`
//!   crate's HAMT/RRB-tree implementations).
//! - [`concurrency::Atom`] / [`concurrency::Ref`] / [`concurrency::Agent`]:
//!   Clojure-style shared mutable state (atomic swap, alter, and
//!   fire-and-forget-flavored send, all backed by `parking_lot::RwLock`).
//! - [`var::Var`]: dynamically-scoped variable binding (push/pop style,
//!   Clojure `binding`-alike).
//!
//! [`proofs`] holds prose/pseudo-math correctness sketches for the above
//! invariants (immutability, structural sharing, thread safety) -- not a
//! machine-checked proof, but design documentation kept alongside the code.

pub mod concurrency;
pub mod hashmap;
pub mod proofs;
pub mod var;
pub mod vector;

pub use concurrency::{Agent, Atom, Ref};
pub use hashmap::PersistentHashMap;
pub use var::Var;
pub use vector::PersistentVector;
