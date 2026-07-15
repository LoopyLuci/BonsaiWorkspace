//! Creator - generative-media tools (image/audio/video/3D/Gaussian
//! splatting) dispatched through a [`CreatorOrchestrator`] registry, plus
//! supporting infrastructure: model weight downloading with an explicit
//! user-confirmation safety gate ([`model_fetch`]), LoRA/DPO fine-tuning
//! job management ([`fine_tuning`]), content-safety checks ([`guardian`]),
//! and progress streaming for long-running jobs ([`progress`]).
//!
//! Every generation tool here is an honestly-labeled skeleton: it produces
//! real, correctly-shaped output (a real PNG sized/colored from the actual
//! prompt+seed, a real WAV header sized from the actual requested
//! duration, ...) through a deterministic placeholder algorithm, with the
//! real ML model integration point clearly marked "production path
//! (stubbed)" — swapping in real model weights later doesn't require
//! changing any caller.
//!
//! Not included: the original crate's `composer` module (Sylva-script
//! pipeline composition) depended on a `tool-registry` crate that is still
//! archived and out of scope for this restoration.

pub mod audio;
pub mod core;
pub mod fine_tuning;
pub mod gaussian;
pub mod gaussian_pipeline;
pub mod guardian;
pub mod image;
pub mod model_fetch;
pub mod progress;
pub mod three_d;
pub mod video;

pub use audio::{BarkTtsTool, MusicGenTool};
pub use core::{CreatorOrchestrator, GenerateParams, GenerationResult, GenerativeTool};
pub use fine_tuning::FineTuningActor;
pub use gaussian::GaussianSplattingTool;
pub use gaussian_pipeline::{CameraParams, GaussianPipeline, GpuSplat};
pub use guardian::Guardian;
pub use image::FluxDiTTool;
pub use model_fetch::{fetch_model, list_cached};
pub use progress::{ProgressEvent, ProgressStreamer};
pub use three_d::Trellis3DTool;
pub use video::SvdVideoTool;
