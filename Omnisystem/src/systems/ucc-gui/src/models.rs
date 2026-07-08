//! Data models for UnixCC GUI

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Build result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub id: String,
    pub success: bool,
    pub duration_ms: u128,
    pub errors: usize,
    pub warnings: usize,
    pub output: String,
    pub timestamp: DateTime<Utc>,
}

/// Build metrics
#[derive(Debug, Clone, Default)]
pub struct BuildMetrics {
    pub total_builds: usize,
    pub successful_builds: usize,
    pub average_build_time_ms: u128,
    pub cache_hit_rate: f32,
    pub total_compile_time_ms: u128,
}

/// Compilation unit information
#[derive(Debug, Clone)]
pub struct CompilationUnitInfo {
    pub name: String,
    pub language: String,
    pub status: UnitStatus,
    pub duration_ms: u128,
    pub dependencies: Vec<String>,
}

/// Compilation unit status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitStatus {
    Pending,
    Compiling,
    Success,
    Failed,
    Cached,
}

impl UnitStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            UnitStatus::Pending => "⏳",
            UnitStatus::Compiling => "🔨",
            UnitStatus::Success => "✅",
            UnitStatus::Failed => "❌",
            UnitStatus::Cached => "⚡",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            UnitStatus::Pending => egui::Color32::GRAY,
            UnitStatus::Compiling => egui::Color32::YELLOW,
            UnitStatus::Success => egui::Color32::GREEN,
            UnitStatus::Failed => egui::Color32::RED,
            UnitStatus::Cached => egui::Color32::LIGHT_BLUE,
        }
    }
}

/// Project information
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub name: String,
    pub path: std::path::PathBuf,
    pub languages: Vec<String>,
    pub total_units: usize,
}
