/// Integration with the Survival System.
/// 
/// When a Bonsai component crashes, the Survival System can trigger a targeted
/// scan to identify the root cause and apply automatic fixes.

use anyhow::{anyhow, Result};
use log::{info, warn};
use std::path::PathBuf;
use crate::{BugHuntOrchestrator, Finding, Severity};

/// Request context for Survival System to initiate a scan.
#[derive(Debug, Clone)]
pub struct SurvivalScanRequest {
    /// Path to the component that crashed.
    pub component_path: PathBuf,
    /// Error message or panic message.
    pub error_context: String,
    /// Optional stack trace snippet for better targeting.
    pub backtrace_snippet: Option<String>,
}

/// Response from Sweeper to Survival System.
#[derive(Debug, Clone)]
pub struct SurvivalScanResponse {
    /// Findings that likely relate to the crash.
    pub related_findings: Vec<Finding>,
    /// The top candidate fix (if any).
    pub recommended_fix: Option<Finding>,
    /// Confidence (0-1) that this fix will resolve the issue.
    pub fix_confidence: f32,
}

/// Trigger a targeted bug hunt scan on a component after a crash.
/// 
/// This is called by the Survival System to diagnose issues and find fixes.
pub async fn scan_on_crash(req: SurvivalScanRequest) -> Result<SurvivalScanResponse> {
    info!("Survival System triggered scan for component: {:?}", req.component_path);

    // Create an orchestrator scoped to the affected component.
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow!("Could not determine cache directory"))?
        .join("bonsai/bug-hunt");

    let orchestrator = BugHuntOrchestrator::new(cache_dir, req.component_path)?;

    // Run a quick (static-only) scan for speed.
    let report = orchestrator.scan_incremental().await?;

    // Filter findings related to the error context.
    let mut related = report
        .issues
        .iter()
        .filter(|f| {
            // Match if the error message appears in the finding message/suggestion
            f.message.to_lowercase().contains(&req.error_context.to_lowercase())
                || f.suggestion
                    .as_ref()
                    .map(|s| s.to_lowercase().contains(&req.error_context.to_lowercase()))
                    .unwrap_or(false)
                || (req.backtrace_snippet.as_ref()
                    .map(|bt| f.file_path.to_string_lossy().contains(bt))
                    .unwrap_or(false))
        })
        .cloned()
        .collect::<Vec<_>>();

    // Sort by severity and confidence.
    related.sort_by(|a, b| {
        let severity_cmp = (b.severity as u8).cmp(&(a.severity as u8));
        if severity_cmp != std::cmp::Ordering::Equal {
            severity_cmp
        } else {
            b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let recommended_fix = related
        .iter()
        .find(|f| f.suggested_diff.is_some() && f.confidence > 0.75)
        .cloned();

    let fix_confidence = recommended_fix.as_ref().map(|f| f.confidence).unwrap_or(0.0);

    info!(
        "Survival scan complete: {} related findings, recommended fix confidence: {:.2}",
        related.len(),
        fix_confidence
    );

    Ok(SurvivalScanResponse {
        related_findings: related,
        recommended_fix,
        fix_confidence,
    })
}

/// Record a successful auto-fix back to the Survival Knowledge Base.
/// 
/// This allows the Survival System to skip the full scan next time
/// if it encounters a similar crash pattern.
pub async fn record_fix_in_survival_kb(
    component_name: &str,
    error_pattern: &str,
    applied_fix: &str,
) -> Result<()> {
    info!(
        "Recording fix for component '{}' in Survival KB: error='{}' fix='{}'",
        component_name, error_pattern, applied_fix
    );

    // In a real implementation, this would persist to the Survival System's KB.
    // For now, we just log it; the Survival System stores it.
    // Example storage format: (component, error_hash, fix_hash, timestamp)

    Ok(())
}

/// Check if we have a known fix for this error pattern in the Survival KB.
pub async fn lookup_known_fix(error_pattern: &str) -> Result<Option<String>> {
    // Query Survival KB for a matching pattern.
    // In production, this queries the Survival System's persistent store.
    info!("Looking up known fix for error pattern: {}", error_pattern);
    Ok(None) // Placeholder; Survival System handles actual lookup.
}
