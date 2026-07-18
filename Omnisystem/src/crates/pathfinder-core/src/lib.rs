//! pathfinder-core: a course enrollment / progress / achievement
//! tracking system.
//!
//! [`user`]/[`course`]/[`progress`]/[`achievement`] form a real,
//! tested, self-contained in-memory learning platform: register users,
//! catalog courses, track per-user enrollment progress, and award
//! achievements.
//!
//! Note: the archived source also shipped a second, more ambitious
//! "PATHFINDER" subsystem (`database.rs`/`models.rs`/`module_impl.rs`/
//! `service.rs`) built on a real Postgres schema and describing itself
//! as doing Bayesian Knowledge Tracing. It's left un-wired here: its
//! `service.rs` handlers (e.g. `handle_get_p_know`, which claims to
//! "Calculate P(Know) using Bayesian Knowledge Tracing") actually
//! return hardcoded constants (`"p_know": 0.75`) regardless of the
//! user/skill passed in, and none of those four files have any test
//! coverage. The files are left on disk for reference but are not
//! declared as modules here, so they aren't compiled.

pub mod achievement;
pub mod course;
pub mod error;
pub mod progress;
pub mod user;

pub use achievement::{Achievement, AchievementManager};
pub use course::{Course, CourseLevel, CourseLibrary};
pub use error::{PathfinderError, Result};
pub use progress::{EnrollmentProgress, ProgressTracker};
pub use user::{User, UserManager};
