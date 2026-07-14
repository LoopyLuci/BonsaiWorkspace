//! mobile-ffi: hardware-accelerated (MediaCodec-backed) video decode
//! pipeline for Android.
//!
//! [`Decoder`] models the H.264/H.265 decode pipeline: buffer queuing,
//! backpressure, and lock-free atomic metrics collection are all real;
//! the actual hardware decode call is simulated (see [`decoder`] module
//! docs) since MediaCodec isn't available off-device.
//!
//! Note: this crate originally also shipped a JNI bridge layer
//! (`ffi.rs`/`llm_jni.rs`, `extern "C" fn Java_...` entry points for an
//! Android LLM chat service) written against jni-crate APIs that don't
//! match any single published `jni` version consistently (mixing by-ref
//! and by-value calling conventions that don't compile against 0.19,
//! 0.20, or 0.21). Since those entry points require a live JVM to
//! exercise and aren't covered by any test, they were dropped rather
//! than guessed-and-rewritten; the video decode pipeline below is
//! complete and independently real.

pub mod codec;
pub mod decoder;
pub mod error;
pub mod metrics;

pub use codec::{CodecFormat, MediaFormat};
pub use decoder::{Decoder, DecodeResult, DecoderConfig, FrameBuffer};
pub use error::{Error, Result};
pub use metrics::{DecoderMetrics, MetricsCollector};
