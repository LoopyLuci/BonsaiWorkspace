//! bedf-property: a small property-based testing harness for the BEDF
//! family (see also bedf-fuzzing, bedf-concurrency).
//!
//! [`generator::InputGenerator`] produces random byte/number/string
//! inputs and can shrink a failing input toward a minimal
//! counterexample. [`property::Property`] is the trait properties
//! implement (e.g. [`property::CommutativeProperty`],
//! [`property::IdempotentProperty`]), and [`property::PropertyTester`]
//! ties generation and checking together, running a property against
//! `config.num_tests` random inputs and reporting failures.

pub mod config;
pub mod generator;
pub mod property;

pub use config::PropertyTestConfig;
pub use generator::InputGenerator;
pub use property::{CommutativeProperty, IdempotentProperty, Property, PropertyResult, PropertyTester};
