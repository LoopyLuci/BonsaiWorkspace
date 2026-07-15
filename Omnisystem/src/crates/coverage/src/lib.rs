//! Coverage - CI/CD test coverage tracking, gating, history, and reporting
//!
//! [`collector::CoverageCollector`] records per-crate coverage results,
//! [`enforcer::CoverageEnforcer`] checks them against configurable gates
//! (target percent + regression threshold), [`history::CoverageHistory`]
//! tracks trends over time, [`reporting::CoverageReporter`] renders
//! markdown/JSON reports, and [`integration::CICoverageIntegration`] wires
//! all of the above into a single CI-friendly check.

pub mod collector;
pub mod enforcer;
pub mod history;
pub mod integration;
pub mod reporting;

pub use collector::{AggregateCoverage, CoverageCollector, CoverageResult, FileCoverage};
pub use enforcer::{CoverageEnforcer, CoverageGate, EnforcementSummary, GateCheckResult};
pub use history::{CoverageHistory, CoverageHistoryPoint, TrendDirection};
pub use integration::{
    generate_badge_url, parse_tarpaulin_output, CICoverageCheckResult, CICoverageIntegration,
    CoverageData,
};
pub use reporting::{CoverageReport, CoverageReporter, CrateReport};
