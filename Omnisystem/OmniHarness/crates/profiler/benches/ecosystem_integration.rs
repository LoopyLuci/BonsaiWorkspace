use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Benchmark ecosystem integration initialization
fn benchmark_ecosystem_init(c: &mut Criterion) {
    c.bench_function("ecosystem_init", |b| {
        b.iter(|| {
            // Simulate ecosystem initialization
            let mut sum = 0;
            for i in 0..black_box(1000) {
                sum += i;
            }
            sum
        })
    });
}

/// Benchmark event publishing
fn benchmark_event_publish(c: &mut Criterion) {
    c.bench_function("event_publish_1000", |b| {
        b.iter(|| {
            for _ in 0..black_box(1000) {
                // Simulate event publishing
                let _event = black_box(42);
            }
        })
    });
}

/// Benchmark metric collection
fn benchmark_metric_collection(c: &mut Criterion) {
    c.bench_function("metric_collection", |b| {
        b.iter(|| {
            let mut metrics = Vec::new();
            for i in 0..black_box(100) {
                metrics.push(black_box(i as f64));
            }
            metrics.iter().sum::<f64>()
        })
    });
}

criterion_group!(benches, benchmark_ecosystem_init, benchmark_event_publish, benchmark_metric_collection);
criterion_main!(benches);
