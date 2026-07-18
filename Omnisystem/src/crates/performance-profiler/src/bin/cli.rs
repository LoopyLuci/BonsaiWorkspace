//! CLI that exercises the CPU profiler and flamegraph analyzer.

use performance_profiler::{CpuProfiler, PerformanceAnalyzer, StackFrame};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profiler = CpuProfiler::new();
    profiler.start_sampling().await?;

    profiler
        .record_sample(
            1200,
            vec![StackFrame {
                function_name: "parse_request".to_string(),
                module_name: "server".to_string(),
                line_number: 42,
                offset: 0x1000,
            }],
        )
        .await?;
    profiler
        .record_sample(
            300,
            vec![StackFrame {
                function_name: "serialize_response".to_string(),
                module_name: "server".to_string(),
                line_number: 88,
                offset: 0x2000,
            }],
        )
        .await?;

    let report = profiler.stop_sampling().await?;
    println!(
        "Profile: {} sample(s), {}ms, {:.1}% CPU, {}MB peak",
        report.total_samples, report.duration_ms, report.cpu_time_percent, report.memory_peak_mb
    );
    for hotspot in &report.hotspots {
        println!("  hotspot: {}", hotspot.function_name);
    }

    let samples = profiler.get_samples().await?;
    let analyzer = PerformanceAnalyzer::new();
    let metrics = analyzer.analyze_samples(&samples).await?;
    for metric in &metrics {
        println!("Metric {}: {:.2} {}", metric.name, metric.value, metric.unit);
    }

    let flamegraph = analyzer.generate_flamegraph(&samples).await?;
    for node in &flamegraph {
        println!(
            "Flamegraph node {}: {:.1}% ({} sample(s))",
            node.function_name, node.time_percent, node.sample_count
        );
    }

    Ok(())
}
