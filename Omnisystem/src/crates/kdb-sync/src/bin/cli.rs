//! KDB-Sync CLI: simulates Phase A ETL output from a handful of projects
//! for the same lint rule, collects it into RuleMetrics, and aggregates it
//! into consensus statistics -- exercising the real pipeline end to end.

use kdb_sync::etl::RuleConfidenceMetrics;
use kdb_sync::{MetricsCollector, ProjectMetrics, RuleMetric, RuleMetricsAggregator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rule_id = "unused-import";

    // Simulate Phase A ETL output from three different projects/domains.
    let samples = [
        ("proj-web-1", "rust", "web", 120u32, 8u32, 4u32, 118u32, 0.97),
        ("proj-web-2", "rust", "web", 95u32, 5u32, 2u32, 90u32, 0.95),
        ("proj-sys-1", "rust", "systems", 40u32, 18u32, 10u32, 30u32, 0.60),
    ];

    let mut aggregator_input = Vec::new();
    for (project_id, language, domain, tp, fp, dismissed, fixes, fix_rate) in samples {
        let mut collector = MetricsCollector::new(project_id.to_string(), language.to_string(), domain.to_string());
        let etl = RuleConfidenceMetrics::new(tp, fp, dismissed, fixes, fix_rate);
        let metric = RuleMetric::from_etl_metrics(
            rule_id.to_string(),
            project_id.to_string(),
            language.to_string(),
            domain.to_string(),
            &etl,
            250,
            12_000_000,
        );
        collector.add_metric(metric.clone());

        let summary = collector.summary();
        println!(
            "{project_id} ({domain}): confidence={:.2} fp_rate={:.2} (from {} total findings)",
            summary.avg_confidence,
            summary.avg_fp_rate,
            etl.total_findings()
        );

        aggregator_input.push((
            project_id.to_string(),
            language.to_string(),
            domain.to_string(),
            metric,
        ));
    }

    let aggregator = RuleMetricsAggregator::new();
    for (project_id, language, domain, metric) in aggregator_input {
        aggregator.add_project_metrics(
            rule_id.to_string(),
            ProjectMetrics {
                project_id,
                language,
                domain,
                confidence: metric.confidence,
                fp_rate: metric.fp_rate,
                tp_rate: metric.tp_rate,
                dismissal_rate: metric.dismissal_rate,
                project_size: 250,
            },
        )?;
    }

    let aggregated = aggregator
        .get_aggregated_metrics(rule_id)
        .await?
        .expect("metrics were just added");

    println!("\n=== Aggregated: {rule_id} ===");
    println!(
        "confidence: mean={:.2} std={:.2} range=[{:.2}, {:.2}]",
        aggregated.confidence_mean, aggregated.confidence_std, aggregated.confidence_min, aggregated.confidence_max
    );
    println!("consensus score: {:.2}", aggregated.consensus_score);
    println!("recommended severity: {}", aggregated.recommended_severity);
    println!("variants by domain:");
    for variant in &aggregated.variants {
        println!(
            "  {} ({} projects): confidence={:.2} fp_rate={:.2} -> {}",
            variant.domain, variant.project_count, variant.confidence_mean, variant.fp_rate, variant.recommended_severity
        );
    }

    Ok(())
}
