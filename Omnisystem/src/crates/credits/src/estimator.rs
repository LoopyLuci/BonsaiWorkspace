use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::urv::BASE_RATE;

/// Profile describing the resource needs of a task type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProfile {
    pub task_type: String,
    pub urv_minutes_per_unit: f64,
    pub unit_name: String,
    pub gpu_fraction: f64,
    pub parallelism_factor: f64,
    /// P50 deviation factor for ETA confidence interval.
    pub p50_deviation: f64,
}

/// Input for a project estimate request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEstimateRequest {
    pub task_type: String,
    pub units: f64,
    pub num_devices: u32,
    pub total_urv_per_min: f64,
}

/// Estimated resource usage and timeline for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEstimate {
    pub total_urv_minutes: f64,
    pub eta_minutes: f64,
    pub eta_low: f64,
    pub eta_high: f64,
    pub credits_per_minute: f64,
    pub estimated_total_credits: f64,
    pub confidence: f64,
}

/// Compute a project estimate given a request and profile.
pub fn estimate(req: &TaskEstimateRequest, profile: &TaskProfile) -> ProjectEstimate {
    let total_urv_minutes = req.units * profile.urv_minutes_per_unit;

    let effective_urv = if req.total_urv_per_min > 0.0 {
        req.total_urv_per_min * profile.parallelism_factor
    } else {
        1.0
    };

    let eta_minutes = total_urv_minutes / effective_urv;
    let eta_low = eta_minutes * (1.0 - profile.p50_deviation);
    let eta_high = eta_minutes * (1.0 + profile.p50_deviation);

    let credits_per_min = req.total_urv_per_min * BASE_RATE;
    let estimated_total_credits = credits_per_min * eta_minutes;

    // Confidence is higher when we have more devices and a tighter p50 deviation
    let confidence = (1.0 - profile.p50_deviation).max(0.0).min(1.0);

    ProjectEstimate {
        total_urv_minutes,
        eta_minutes,
        eta_low,
        eta_high,
        credits_per_minute: credits_per_min,
        estimated_total_credits,
        confidence,
    }
}

/// Returns built-in task profiles for common workloads.
pub fn default_profiles() -> Vec<TaskProfile> {
    vec![
        TaskProfile {
            task_type: "llm_training_epoch".to_string(),
            urv_minutes_per_unit: 5000.0, // per 1k examples
            unit_name: "1k_examples".to_string(),
            gpu_fraction: 0.85,
            parallelism_factor: 0.9,
            p50_deviation: 0.20,
        },
        TaskProfile {
            task_type: "video_transcode_4k".to_string(),
            urv_minutes_per_unit: 500.0, // per minute of video
            unit_name: "video_minute".to_string(),
            gpu_fraction: 0.70,
            parallelism_factor: 0.85,
            p50_deviation: 0.15,
        },
        TaskProfile {
            task_type: "rust_compile".to_string(),
            urv_minutes_per_unit: 200.0, // per kloc
            unit_name: "kloc".to_string(),
            gpu_fraction: 0.0,
            parallelism_factor: 0.75,
            p50_deviation: 0.25,
        },
        TaskProfile {
            task_type: "image_generation".to_string(),
            urv_minutes_per_unit: 150.0, // per image
            unit_name: "image".to_string(),
            gpu_fraction: 0.90,
            parallelism_factor: 0.95,
            p50_deviation: 0.10,
        },
        TaskProfile {
            task_type: "data_processing".to_string(),
            urv_minutes_per_unit: 50.0, // per GB
            unit_name: "gb".to_string(),
            gpu_fraction: 0.10,
            parallelism_factor: 0.80,
            p50_deviation: 0.30,
        },
    ]
}

/// Tracks progress of a running task and estimates remaining time.
pub struct ProgressEstimator {
    pub total_units: f64,
    pub completed_units: f64,
    pub started_at: DateTime<Utc>,
}

impl ProgressEstimator {
    pub fn new(total_units: f64, started_at: DateTime<Utc>) -> Self {
        ProgressEstimator {
            total_units,
            completed_units: 0.0,
            started_at,
        }
    }

    pub fn update(&mut self, completed: f64) {
        self.completed_units = completed.min(self.total_units);
    }

    pub fn eta_minutes(&self) -> f64 {
        if self.completed_units <= 0.0 {
            return f64::INFINITY;
        }
        let elapsed_secs = (Utc::now() - self.started_at).num_seconds() as f64;
        let rate = self.completed_units / elapsed_secs; // units per second
        if rate <= 0.0 {
            return f64::INFINITY;
        }
        let remaining = self.total_units - self.completed_units;
        remaining / rate / 60.0
    }
}
