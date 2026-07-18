use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mobile_ffi::{Decoder, DecoderConfig};

fn benchmark_decode_latency(c: &mut Criterion) {
    c.bench_function("decode_1080p_frame", |b| {
        b.iter_batched(
            || {
                let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
                Decoder::new(config).unwrap()
            },
            |mut decoder| {
                let nal_data = black_box(vec![0u8; 2048]);
                let result = decoder.decode_frame(&nal_data, 33_333);
                assert!(result.is_ok());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("decode_720p_frame", |b| {
        b.iter_batched(
            || {
                let config = DecoderConfig::new("video/avc", 1280, 720).unwrap();
                Decoder::new(config).unwrap()
            },
            |mut decoder| {
                let nal_data = black_box(vec![0u8; 1024]);
                let result = decoder.decode_frame(&nal_data, 33_333);
                assert!(result.is_ok());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("get_output_frame", |b| {
        b.iter_batched(
            || {
                let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
                let mut decoder = Decoder::new(config).unwrap();
                let nal_data = vec![0u8; 2048];
                let _ = decoder.decode_frame(&nal_data, 33_333);
                decoder
            },
            |mut decoder| {
                let result = decoder.get_output_frame();
                assert!(result.is_ok());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, benchmark_decode_latency);
criterion_main!(benches);
