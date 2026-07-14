use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mobile_ffi::{Decoder, DecoderConfig};

fn benchmark_throughput(c: &mut Criterion) {
    c.bench_function("decode_60fps_1080p", |b| {
        b.iter_batched(
            || {
                let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
                Decoder::new(config).unwrap()
            },
            |mut decoder| {
                let nal_data = black_box(vec![0u8; 2048]);
                for i in 0..60 {
                    let timestamp = (i as i64) * 33_333;
                    let _ = decoder.decode_frame(&nal_data, timestamp);
                }
                let metrics = decoder.metrics();
                assert_eq!(metrics.frames_decoded, 60);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("decode_30fps_4k", |b| {
        b.iter_batched(
            || {
                let config = DecoderConfig::new("video/avc", 3840, 2160).unwrap();
                Decoder::new(config).unwrap()
            },
            |mut decoder| {
                let nal_data = black_box(vec![0u8; 8192]);
                for i in 0..30 {
                    let timestamp = (i as i64) * 33_333;
                    let _ = decoder.decode_frame(&nal_data, timestamp);
                }
                let metrics = decoder.metrics();
                assert_eq!(metrics.frames_decoded, 30);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("metrics_collection", |b| {
        b.iter_batched(
            || {
                let config = DecoderConfig::new("video/avc", 1920, 1080).unwrap();
                let mut decoder = Decoder::new(config).unwrap();
                let nal_data = vec![0u8; 2048];
                for i in 0..10 {
                    let _ = decoder.decode_frame(&nal_data, (i as i64) * 33_333);
                }
                decoder
            },
            |decoder| {
                let metrics = black_box(decoder.metrics());
                assert!(metrics.avg_decode_latency_us > 0);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, benchmark_throughput);
criterion_main!(benches);
