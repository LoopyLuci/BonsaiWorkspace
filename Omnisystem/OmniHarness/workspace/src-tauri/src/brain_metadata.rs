//! Persistent brain metadata: tracks which training phases have completed.
//! Stored at ~/.bonsai/brain_metadata.json.

use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrainMetadata {
    pub lessons_completed: u32,
    pub phases_done: Vec<String>,
    pub last_training: Option<String>,
}

fn metadata_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".bonsai")
        .join("brain_metadata.json")
}

impl BrainMetadata {
    pub fn load() -> Self {
        let path = metadata_path();
        if let Ok(raw) = std::fs::read_to_string(&path) {
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let path = metadata_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            std::fs::write(&path, json).ok();
        }
    }

    pub fn record_phase(&mut self, phase: &str) {
        if !self.phases_done.contains(&phase.to_string()) {
            self.phases_done.push(phase.to_string());
            self.lessons_completed += 1;
        }
        self.last_training = Some(Utc::now().to_rfc3339());
        info!(
            "[brain_metadata] phase '{}' recorded — {} total lessons",
            phase, self.lessons_completed
        );
        self.save();
    }
}

// ── Tauri commands ─────────────────────────────────────────────────────────────

#[tauri::command]
#[specta::specta]
pub fn get_brain_metadata() -> BrainMetadata {
    BrainMetadata::load()
}

#[tauri::command]
#[specta::specta]
pub fn record_lesson_complete(phase: String) -> BrainMetadata {
    let mut meta = BrainMetadata::load();
    meta.record_phase(&phase);
    meta
}
