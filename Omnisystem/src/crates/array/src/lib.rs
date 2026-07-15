//! Array - an APL/J-inspired rank-polymorphic N-dimensional array language
//!
//! [`array::NdArray`] is a rank-polymorphic array of `f64`, [`ops`] provides
//! scalar/reduction/scan/inner-product primitives, and [`eval::AplEval`]
//! tokenizes and evaluates APL/J-style expression strings (e.g. `"+/ [1,2,3]"`)
//! against them.

pub mod array;
pub mod error;
pub mod eval;
pub mod ops;

pub use array::{compute_strides, flat_to_indices, NdArray};
pub use error::{ArrayError, ArrayResult};
pub use eval::AplEval;
