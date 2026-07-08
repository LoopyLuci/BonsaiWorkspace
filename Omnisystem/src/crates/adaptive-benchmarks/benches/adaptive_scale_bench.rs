use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_layer_masking(c: &mut Criterion) {
    let mut group = c.benchmark_group("layer_masking");

    let layer_counts = vec![10, 50, 100];

    for count in layer_counts {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.iter(|| {
                let mask = black_box((0..count).map(|i| i % 2 == 0).collect::<Vec<_>>());
                let active = mask.iter().filter(|&&m| m).count();
                black_box(active)
            });
        });
    }

    group.finish();
}

fn benchmark_width_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("width_scaling");

    let scaling_ratios = vec![
        (256, 128),
        (512, 256),
        (1024, 512),
        (4096, 2048),
    ];

    for (from, to) in scaling_ratios {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}->{}", from, to)),
            &(from, to),
            |b, &(from, to)| {
                b.iter(|| {
                    // Simulate dimension truncation
                    let projected = (0..to).map(|_| 1.0f32).collect::<Vec<_>>();
                    black_box(projected)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_expert_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("expert_routing");

    let token_counts = vec![128, 512, 2048];
    let expert_counts = vec![4, 8, 16];

    for tokens in &token_counts {
        for experts in &expert_counts {
            let param = format!("tokens_{}_experts_{}", tokens, experts);
            group.bench_with_input(
                BenchmarkId::from_parameter(param),
                &(*tokens, *experts),
                |b, &(tokens, experts)| {
                    b.iter(|| {
                        let routes: Vec<usize> = (0..tokens)
                            .map(|i| (i * 7) % experts)
                            .collect();
                        black_box(routes)
                    });
                },
            );
        }
    }

    group.finish();
}

fn benchmark_kv_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("kv_cache");

    let cache_sizes = vec![512, 2048, 4096];

    for size in cache_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate KV cache lookup and update
                    let cache = black_box(vec![1.0f32; size]);
                    let updated = cache.iter().map(|v| v * 1.01).collect::<Vec<_>>();
                    black_box(updated)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_scale_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_transitions");

    let transitions = vec![
        (100_000_000, 500_000_000),
        (500_000_000, 1_000_000_000),
        (1_000_000_000, 7_000_000_000),
    ];

    for (from, to) in transitions {
        let param = format!("{:?}->{:?}", from / 1_000_000, to / 1_000_000);
        group.bench_with_input(
            BenchmarkId::from_parameter(param),
            &(from, to),
            |b, _| {
                b.iter(|| {
                    // Simulate scale transition: load params, adjust masks, etc.
                    let scaling_factor = 1.5f32;
                    black_box(scaling_factor * 1.1)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_layer_masking,
    benchmark_width_scaling,
    benchmark_expert_routing,
    benchmark_kv_cache_operations,
    benchmark_scale_transitions
);
criterion_main!(benches);
