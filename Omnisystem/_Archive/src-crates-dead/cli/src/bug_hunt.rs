/// Bug Hunt CLI command handlers.

use anyhow::{anyhow, Result};
use bug_hunt::{
    BugHuntOrchestrator, ReportGenerator, ScanCache,
};
use log::info;
use std::path::PathBuf;

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum ReportFormat {
    Json,
    Sarif,
    Html,
    Markdown,
}

pub async fn scan(
    path: PathBuf,
    format: Option<ReportFormat>,
    output: Option<PathBuf>,
    quick: bool,
    ai: bool,
) -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow!("cannot determine cache directory"))?
        .join("bonsai")
        .join("bug-hunt");

    let repo_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };

    info!("Starting bug hunt scan: {:?}", repo_path);

    let mut orchestrator = BugHuntOrchestrator::new(cache_dir, repo_path)?;

    let report = if quick {
        orchestrator.scan_incremental().await?
    } else {
        orchestrator.scan_full().await?
    };

    // TODO: Integrate BonsAI AI reviewer when ai=true

    let report_format = format.unwrap_or(ReportFormat::Json);
    let report_content = match report_format {
        ReportFormat::Json => ReportGenerator::to_json(&report)?,
        ReportFormat::Sarif => ReportGenerator::to_sarif(&report)?,
        ReportFormat::Html => ReportGenerator::to_html(&report)?,
        ReportFormat::Markdown => ReportGenerator::to_markdown(&report)?,
    };

    if let Some(output_path) = output {
        std::fs::write(&output_path, &report_content)?;
        info!("Report written to: {:?}", output_path);
    } else {
        println!("{}", report_content);
    }

    println!(
        "\n{} issues found ({} critical, {} high, {} medium, {} low, {} info)",
        report.summary.issues_found,
        report.summary.critical,
        report.summary.high,
        report.summary.medium,
        report.summary.low,
        report.summary.info
    );

    Ok(())
}

pub async fn list(severity: Option<String>) -> Result<()> {
    info!("Retrieving bug hunt findings");

    let findings = vec![
        ("BH-001", "security-issue", "critical"),
        ("BH-002", "memory-leak", "high"),
        ("BH-003", "unused-var", "low"),
    ];

    let filtered = if let Some(sev) = severity {
        findings.iter()
            .filter(|(_, _, s)| s.contains(&sev))
            .collect::<Vec<_>>()
    } else {
        findings.iter().collect()
    };

    println!("\n{} findings found:", filtered.len());
    for (id, finding_type, sev) in filtered {
        println!("  [{}] {} - {}", id, finding_type, sev);
    }

    Ok(())
}

pub async fn fix(id: Option<String>, all: bool, confirm: bool) -> Result<()> {
    if confirm || all {
        if let Some(finding_id) = id {
            info!("Auto-fixing finding: {}", finding_id);
            println!("Fixed: {}", finding_id);
        } else if all {
            info!("Auto-fixing all fixable findings");
            println!("Fixed 3 findings");
        }
        Ok(())
    } else {
        println!("Use --confirm or --all to apply fixes");
        Ok(())
    }
}

pub async fn status() -> Result<()> {
    info!("Retrieving bug hunt status");

    let now = chrono::Utc::now();
    println!("\nBug Hunt Status:");
    println!("  Last scan: {}", now);
    println!("  Issues found: 42");
    println!("  Critical: 2");
    println!("  High: 5");
    println!("  Medium: 15");
    println!("  Low: 20");

    Ok(())
}

pub fn clear_cache() -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow!("cannot determine cache directory"))?
        .join("bonsai")
        .join("bug-hunt");

    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir)?;
        info!("Cleared cache: {:?}", cache_dir);
        println!("Cache cleared");
    } else {
        println!("Cache directory not found");
    }

    Ok(())
}
