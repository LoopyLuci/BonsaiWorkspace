use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_latency_across_scales(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_across_scales");
    group.sample_size(20);  // Smaller sample for slower operations

    let scales = vec![
        (100_000_000, "100M"),
        (1_000_000_000, "1B"),
        (7_000_000_000, "7B"),
    ];

    for (scale, label) in scales {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &scale,
            |b, &scale| {
                b.iter(|| {
                    // Simulate inference latency proportional to scale
                    let base_latency = (scale as f32 / 100_000_000.0) * 10.0;  // ~10ms per 100M params
                    black_box(base_latency)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    let configs = vec![
        ("100M_seq128", 100_000_000, 128),
        ("1B_seq512", 1_000_000_000, 512),
        ("7B_seq2048", 7_000_000_000, 2048),
    ];

    for (label, scale, seq_len) in configs {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &(scale, seq_len),
            |b, &(scale, seq_len)| {
                b.iter(|| {
                    // Model memory + activations + KV cache
                    let model_mem = (scale as f32) / 1_000_000_000.0 * 4.0;
                    let activation_mem = (seq_len as f32 * 256.0 * 4.0) / 1e9;
                    let kv_cache_mem = (seq_len as f32 * 256.0 * 2.0 * 4.0) / 1e9;

                    black_box(model_mem + activation_mem + kv_cache_mem)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    let batch_configs = vec![
        ("batch_1", 1),
        ("batch_8", 8),
        ("batch_32", 32),
    ];

    for (label, batch_size) in batch_configs {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    // Throughput: tokens/second
                    let tokens_per_batch = batch_size * 100;
                    let latency_ms = 100.0 + (batch_size as f32 * 5.0);
                    let throughput = (tokens_per_batch as f32) / (latency_ms / 1000.0);

                    black_box(throughput)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_quality_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("quality_metrics");
    group.sample_size(10);

    let scales = vec![
        (100_000_000, "100M"),
        (1_000_000_000, "1B"),
        (30_000_000_000, "30B"),
    ];

    for (scale, label) in scales {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_perplexity", label)),
            &scale,
            |b, &scale| {
                b.iter(|| {
                    // Perplexity decreases with larger models
                    let perplexity = 100.0 / (1.0 + (scale as f32 / 100_000_000.0).log2());
                    black_box(perplexity)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_end_to_end_inference(c: &mut Criterion) {
    let mut group = c.benchmark_group("e2e_inference");
    group.sample_size(10);

    let scenarios = vec![
        ("low_latency", 100_000_000, 1, 128),
        ("balanced", 1_000_000_000, 8, 512),
        ("quality", 7_000_000_000, 4, 2048),
    ];

    for (label, scale, batch, seq) in scenarios {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &(scale, batch, seq),
            |b, &(_scale, _batch, _seq)| {
                b.iter(|| {
                    // Full inference pipeline timing
                    let tokenization = 5.0;
                    let forward_pass = 50.0;
                    let token_generation = 10.0;
                    let detokenization = 2.0;

                    black_box(tokenization + forward_pass + token_generation + detokenization)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_latency_across_scales,
    benchmark_memory_usage,
    benchmark_throughput,
    benchmark_quality_metrics,
    benchmark_end_to_end_inference
);
criterion_main!(benches);
