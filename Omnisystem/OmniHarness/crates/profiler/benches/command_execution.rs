use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_command_parsing(c: &mut Criterion) {
    c.bench_function("parse_command", |b| {
        b.iter(|| {
            let cmd = black_box("EcosystemRunFullPipeline");
            !cmd.is_empty()
        })
    });
}

fn benchmark_command_routing(c: &mut Criterion) {
    c.bench_function("route_command_to_handler", |b| {
        b.iter(|| {
            let handlers = vec![
                "ci_handler", "bug_hunt_handler", "lint_handler",
                "survival_handler", "kdb_handler", "etl_handler",
            ];
            black_box(handlers).iter().find(|h| h.contains("handler")).is_some()
        })
    });
}

criterion_group!(benches, benchmark_command_parsing, benchmark_command_routing);
criterion_main!(benches);
