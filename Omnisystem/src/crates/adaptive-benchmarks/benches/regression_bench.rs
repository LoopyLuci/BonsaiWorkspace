use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_regression_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("regression_detection");
    group.sample_size(20);

    let metric_counts = vec![10, 100, 1000];

    for count in metric_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                b.iter(|| {
                    // Simulate comparing baseline vs current metrics
                    let threshold = 5.0;
                    let mut regressions = 0;

                    for i in 0..count {
                        let baseline = 100.0 + (i as f32);
                        let current = baseline * (1.0 + (i as f32 % 3.0) / 100.0);
                        let regression_pct = ((current - baseline) / baseline * 100.0).abs();

                        if regression_pct > threshold {
                            regressions += 1;
                        }
                    }

                    black_box(regressions)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_latency_regression_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_regression");

    let scale_configs = vec![
        ("100M", 100_000_000),
        ("1B", 1_000_000_000),
        ("7B", 7_000_000_000),
    ];

    for (label, scale) in scale_configs {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &scale,
            |b, &_scale| {
                b.iter(|| {
                    let baseline_latency = 50.0;
                    let current_latency = 52.5;  // 5% increase
                    let threshold_pct = 5.0;

                    let regression_pct = ((current_latency - baseline_latency) / baseline_latency) * 100.0;
                    let is_regression = regression_pct > threshold_pct;

                    black_box(is_regression)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_quality_regression_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("quality_regression");

    let benchmarks = vec![
        ("mmlu", 80.0),
        ("humaneval", 45.0),
        ("perplexity", 25.0),
    ];

    for (bench_name, baseline_score) in benchmarks {
        group.bench_with_input(
            BenchmarkId::from_parameter(bench_name),
            &baseline_score,
            |b, &baseline| {
                b.iter(|| {
                    let current_score = baseline * 0.97;  // 3% decrease
                    let threshold_pct = 5.0;

                    let regression_pct = ((baseline - current_score) / baseline) * 100.0;
                    let is_regression = regression_pct > threshold_pct;

                    black_box(is_regression)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_rollback_decision(c: &mut Criterion) {
    let mut group = c.benchmark_group("rollback_decision");

    let severity_levels = vec![
        ("minor", vec![2.0, 1.5, 3.0]),      // 2% regressions
        ("moderate", vec![8.0, 7.5, 9.0]),  // 8% regressions
        ("severe", vec![25.0, 30.0, 22.0]), // 25%+ regressions
    ];

    for (label, regressions) in severity_levels {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &regressions,
            |b, regs| {
                b.iter(|| {
                    let avg_regression = regs.iter().sum::<f32>() / regs.len() as f32;
                    let should_rollback = avg_regression > 20.0;

                    black_box(should_rollback)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_metric_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("metric_comparison");

    let metric_pairs = vec![
        ("latency", 15.5, 16.2),
        ("throughput", 64.5, 62.3),
        ("memory", 2048.0, 2100.0),
    ];

    for (metric_name, baseline, current) in metric_pairs {
        group.bench_with_input(
            BenchmarkId::from_parameter(metric_name),
            &(baseline, current),
            |b, &(baseline, current)| {
                b.iter(|| {
                    let change_pct = if baseline.abs() > 1e-9 {
                        ((current - baseline) / baseline) * 100.0
                    } else {
                        0.0
                    };

                    black_box(change_pct)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_regression_report_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("report_generation");
    group.sample_size(10);

    let result_counts = vec![50, 500, 5000];

    for count in result_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                b.iter(|| {
                    // Simulate generating comprehensive regression report
                    let mut report_data = Vec::new();

                    for i in 0..count {
                        let baseline = 100.0 + (i as f32);
                        let current = baseline * (1.0 + (i as f32 % 5.0) / 100.0);
                        let change_pct = ((current - baseline) / baseline) * 100.0;

                        report_data.push((
                            format!("metric_{}", i),
                            baseline,
                            current,
                            change_pct,
                        ));
                    }

                    let total_issues = report_data.iter()
                        .filter(|(_, _, _, change)| change.abs() > 5.0)
                        .count();

                    black_box(total_issues)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_regression_detection,
    benchmark_latency_regression_check,
    benchmark_quality_regression_check,
    benchmark_rollback_decision,
    benchmark_metric_comparison,
    benchmark_regression_report_generation
);
criterion_main!(benches);
