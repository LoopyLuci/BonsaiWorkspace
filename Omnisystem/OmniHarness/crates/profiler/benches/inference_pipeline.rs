use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_model_loading(c: &mut Criterion) {
    c.bench_function("load_model", |b| {
        b.iter(|| {
            // Simulate model loading
            let mut tensors = Vec::new();
            for i in 0..black_box(100) {
                tensors.push(vec![i; 10]);
            }
            tensors.len()
        })
    });
}

fn benchmark_inference_execution(c: &mut Criterion) {
    c.bench_function("inference_forward_pass", |b| {
        b.iter(|| {
            // Simulate inference
            let mut output = 0.0;
            for i in 0..black_box(1000) {
                output += i as f64 * 0.001;
            }
            output
        })
    });
}

criterion_group!(benches, benchmark_model_loading, benchmark_inference_execution);
criterion_main!(benches);
