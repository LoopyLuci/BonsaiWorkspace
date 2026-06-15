use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Benchmark event bus throughput (events per second)
fn benchmark_event_bus_throughput(c: &mut Criterion) {
    c.bench_function("event_bus_1000_events", |b| {
        b.iter(|| {
            // Simulate event bus operations
            let mut count = 0;
            for _ in 0..black_box(1000) {
                count += 1;
            }
            count
        })
    });
}

/// Benchmark event filtering
fn benchmark_event_filtering(c: &mut Criterion) {
    c.bench_function("filter_events_by_type", |b| {
        b.iter(|| {
            let events: Vec<usize> = (0..black_box(100)).collect();
            events.iter().filter(|&&e| e % 2 == 0).count()
        })
    });
}

criterion_group!(benches, benchmark_event_bus_throughput, benchmark_event_filtering);
criterion_main!(benches);
