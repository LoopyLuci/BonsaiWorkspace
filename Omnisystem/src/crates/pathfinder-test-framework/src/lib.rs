//! pathfinder-test-framework: shared test support for the pathfinder-*
//! crates (pathfinder-core, pathfinder-user-service).
//!
//! Provides JSON-shape [`assertions::PathfinderAssertions`], builder-style
//! [`fixtures`] for common domain objects (users/skills/exercises), and
//! [`mocks`] for module request/response round-tripping in tests.

pub mod assertions;
pub mod fixtures;
pub mod mocks;

pub use assertions::PathfinderAssertions;
pub use fixtures::{ExerciseBuilder, ExerciseFixture, SkillFixture, UserBuilder, UserFixture};
pub use mocks::{MockModuleRequest, MockResponse};
