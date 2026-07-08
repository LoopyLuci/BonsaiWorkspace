use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_kl_divergence_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("kl_divergence");

    let sample_sizes = vec![100, 1000, 10000];

    for size in sample_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate KL divergence calculation
                    let p_dist: Vec<f32> = (0..size).map(|i| (i as f32) / (size as f32)).collect();
                    let q_dist: Vec<f32> = (0..size).map(|i| ((i + 1) as f32) / (size as f32)).collect();

                    let kl: f32 = p_dist.iter().zip(q_dist.iter())
                        .map(|(&p, &q)| {
                            if p > 0.0 && q > 0.0 {
                                p * (p / q).ln()
                            } else {
                                0.0
                            }
                        })
                        .sum();

                    black_box(kl)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_output_consistency(c: &mut Criterion) {
    let mut group = c.benchmark_group("output_consistency");

    let scales = vec![
        (100_000_000, 1_000_000_000),
        (500_000_000, 7_000_000_000),
        (1_000_000_000, 30_000_000_000),
    ];

    for (small_scale, large_scale) in scales {
        let label = format!("{}M_to_{}B", small_scale / 1_000_000, large_scale / 1_000_000_000);
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &(small_scale, large_scale),
            |b, &(_small, _large)| {
                b.iter(|| {
                    // Simulate comparing outputs from different scales
                    let output_small = (0..100).map(|i| (i as f32) / 100.0).collect::<Vec<_>>();
                    let output_large = (0..100).map(|i| ((i + 1) as f32) / 100.0).collect::<Vec<_>>();

                    let similarity = output_small.iter().zip(output_large.iter())
                        .map(|(a, b)| 1.0 - (a - b).abs())
                        .sum::<f32>() / output_small.len() as f32;

                    black_box(similarity)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_hallucination_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("hallucination_detection");

    let prompt_lengths = vec![50, 200, 1000];

    for prompt_len in prompt_lengths {
        group.bench_with_input(
            BenchmarkId::from_parameter(prompt_len),
            &prompt_len,
            |b, &prompt_len| {
                b.iter(|| {
                    // Simulate hallucination scoring
                    let prompt_tokens: Vec<String> = (0..prompt_len)
                        .map(|i| format!("token_{}", i))
                        .collect();

                    let output_tokens: Vec<String> = (0..prompt_len + 50)
                        .map(|i| format!("token_{}", i))
                        .collect();

                    let hallucination_count = output_tokens.iter()
                        .filter(|t| !prompt_tokens.contains(t))
                        .count();

                    let hallucination_rate = hallucination_count as f32 / output_tokens.len() as f32;

                    black_box(hallucination_rate)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_subset_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("subset_validation");

    let mask_sizes = vec![50, 100, 200];

    for size in mask_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Validate that smaller subset is a strict subset
                    let full_mask: Vec<bool> = (0..size).map(|_| true).collect();
                    let subset_mask: Vec<bool> = (0..size).map(|i| i % 2 == 0).collect();

                    let is_subset = subset_mask.iter().zip(full_mask.iter())
                        .all(|(&s, &f)| !s || f);

                    black_box(is_subset)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_semantic_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("semantic_similarity");
    group.sample_size(20);

    let scales = vec![
        ("100M", 100_000_000),
        ("1B", 1_000_000_000),
        ("7B", 7_000_000_000),
    ];

    for (label, _scale) in scales {
        group.bench_with_input(
            BenchmarkId::from_parameter(label),
            &label,
            |b, _| {
                b.iter(|| {
                    // Simulate computing semantic similarity between outputs
                    let embedding_dim = 768;
                    let output1: Vec<f32> = (0..embedding_dim)
                        .map(|i| (i as f32).sin())
                        .collect();

                    let output2: Vec<f32> = (0..embedding_dim)
                        .map(|i| (i as f32 + 0.1).sin())
                        .collect();

                    // Cosine similarity
                    let dot_product: f32 = output1.iter().zip(output2.iter())
                        .map(|(a, b)| a * b)
                        .sum();

                    let mag1: f32 = output1.iter().map(|v| v * v).sum::<f32>().sqrt();
                    let mag2: f32 = output2.iter().map(|v| v * v).sum::<f32>().sqrt();

                    let similarity = if mag1 > 0.0 && mag2 > 0.0 {
                        dot_product / (mag1 * mag2)
                    } else {
                        0.0
                    };

                    black_box(similarity)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_kl_divergence_computation,
    benchmark_output_consistency,
    benchmark_hallucination_detection,
    benchmark_subset_validation,
    benchmark_semantic_similarity
);
criterion_main!(benches);
