//! collaboration: team rule governance — team-scoped rule profiles with
//! file-backed persistence, community rule proposals with voting, and a
//! shared/synchronized rule library.

pub mod core;
pub mod error;
pub mod shared_library;
pub mod team_profiles;
pub mod types;
pub mod voting;

pub use core::Core;
pub use shared_library::{RuleLibraryEntry, SharedLibrary, SharedRule, SyncStatus};
pub use team_profiles::{TeamProfileManager, TeamRuleConfig, TeamRuleProfile};
pub use types::State;
pub use voting::{ProposalStatus, RuleProposal, Vote, VotingSystem};
