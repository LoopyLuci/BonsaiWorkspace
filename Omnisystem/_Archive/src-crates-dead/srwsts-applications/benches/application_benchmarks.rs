//! Benchmark suite for SRWSTS Applications

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use srwsts_applications::{ApplicationMetrics, InputSimulator};
use std::sync::Arc;
use std::time::Duration;

fn benchmark_metrics_recording(c: &mut Criterion) {
    c.bench_function("metrics_response_time", |b| {
        let metrics = Arc::new(ApplicationMetrics::new());
        b.to_async(tokio::runtime::Runtime::new().unwrap()).iter(|| async {
            metrics.record_response_time("test", Duration::from_millis(10));
        });
    });

    c.bench_function("metrics_memory", |b| {
        let metrics = Arc::new(ApplicationMetrics::new());
        b.iter(|| {
            metrics.record_memory("heap", black_box(1024 * 1024 * 512));
        });
    });

    c.bench_function("metrics_compilation", |b| {
        let metrics = Arc::new(ApplicationMetrics::new());
        b.iter(|| {
            metrics.record_compilation(Duration::from_millis(100));
        });
    });
}

fn benchmark_input_simulation(c: &mut Criterion) {
    c.bench_function("input_simulator_recording", |b| {
        let simulator = InputSimulator::new();
        b.to_async(tokio::runtime::Runtime::new().unwrap()).iter(|| async {
            use srwsts_applications::InputEvent;
            let event = InputEvent::key_press('a', "window-1");
            simulator.record_event(event).await.ok();
        });
    });
}

criterion_group!(benchmarks, benchmark_metrics_recording, benchmark_input_simulation);
criterion_main!(benchmarks);
