//! pathfinder-user-service: authentication primitives for the
//! pathfinder-* crate group (pathfinder-core, pathfinder-test-framework).
//!
//! [`auth`] is real, tested cryptography: bcrypt password hashing and
//! JWT sign/verify with a hardened, env-var-backed signing secret (no
//! hardcoded fallback; a too-short `JWT_SECRET` is rejected).
//!
//! Note: the archived source also shipped `service.rs`, a request-handler
//! layer (handle_register/handle_authenticate/handle_get_profile/etc.).
//! It's left un-wired here, mirroring the same call already made for
//! pathfinder-core's own service.rs (see that crate's lib.rs doc
//! comment): every handler in it returns hardcoded data regardless of
//! its arguments (handle_authenticate always checks against the literal
//! string `"$2b$12$somehash"`; handle_get_profile always returns
//! `"user@example.com"` / `"John Doe"` for any user_id) with no database
//! or store behind any of it. The file is kept on disk for reference but
//! not declared as a module here, so it isn't compiled.

pub mod auth;
