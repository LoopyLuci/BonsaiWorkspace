//! Benchmarks for the Universal Language Layer's registry and bridge —
//! the real hot paths (module registration/lookup, function dispatch
//! registration), not synthetic placeholders.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use universal_language_layer::{Language, LanguageRegistry};

fn bench_register_module(c: &mut Criterion) {
    c.bench_function("registry_register_module", |b| {
        b.iter(|| {
            let mut registry = LanguageRegistry::new();
            registry.register_module(black_box("bench-module"), black_box(Language::Rust));
        });
    });
}

fn bench_lookup_module(c: &mut Criterion) {
    let mut registry = LanguageRegistry::new();
    for i in 0..1000 {
        registry.register_module(&format!("module-{i}"), Language::Rust);
    }

    c.bench_function("registry_get_module", |b| {
        b.iter(|| registry.get_module(black_box("module-500")));
    });
}

fn bench_list_by_language(c: &mut Criterion) {
    let mut registry = LanguageRegistry::new();
    for i in 0..1000 {
        let lang = if i % 2 == 0 { Language::Rust } else { Language::Titan };
        registry.register_module(&format!("module-{i}"), lang);
    }

    c.bench_function("registry_list_by_language", |b| {
        b.iter(|| registry.list_by_language(black_box(Language::Rust)));
    });
}

criterion_group!(benches, bench_register_module, bench_lookup_module, bench_list_by_language);
criterion_main!(benches);
