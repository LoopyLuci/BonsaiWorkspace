//! Multi-modal tool integrations for BonsAI.
//!
//! Each sub-module wraps an external model (GGUF sidecar, Python worker, ONNX)
//! as a `Tool` that registers in the `ToolRegistry`.  All model loading is
//! **demand-only** and **100 % offline** — nothing is fetched from the network
//! unless the user explicitly triggers a download through the UI.
//!
//! # Phase 1 (shipped)
//! - `kokoro`        — Kokoro-82M TTS (upgrade/replacement for Piper)
//! - `depth`         — Depth-Anything-V2-GGUF monocular depth estimation
//! - `yolo`          — YOLOv8 object/pose/segmentation detection
//! - `opencv_tools`  — OpenCV 4.12 core vision toolkit (12 tools)
//! - `pixai_tagger`  — PixAI ONNX image tagger (13 000+ Danbooru tags)
//! - `nu_markdown`   — NuMarkdown-8B-Thinking document OCR → Markdown
//! - `image_edit`    — Qwen-Image-Edit rapid + multi-angle LoRA image editing
//! - `video_gen`     — Sulphur-2-base video generation
//! - `threed_gen`    — TRELLIS.2-4B 3D asset generation

pub mod depth;
pub mod image_edit;
pub mod kokoro;
pub mod nu_markdown;
pub mod opencv_tools;
pub mod pixai_tagger;
pub mod threed_gen;
pub mod video_gen;
pub mod yolo;

use crate::tool_registry::ToolRegistryState;

/// Register all multi-modal tools into the given registry.
pub async fn register_all(state: &ToolRegistryState) {
    // Phase 1 — TTS
    state
        .registry
        .register(Box::new(kokoro::KokoroTtsTool::new()))
        .await;
    state
        .registry
        .register(Box::new(kokoro::ListVoicesTool))
        .await;
    // Phase 1 — Depth estimation
    state
        .registry
        .register(Box::new(depth::DepthEstimationTool::new()))
        .await;
    // Phase 1 — YOLO detection
    state
        .registry
        .register(Box::new(yolo::DetectObjectsTool::new()))
        .await;
    state
        .registry
        .register(Box::new(yolo::EstimatePoseTool::new()))
        .await;
    state
        .registry
        .register(Box::new(yolo::SegmentObjectsTool::new()))
        .await;
    // Phase 2 — OpenCV 4.12 vision toolkit
    state
        .registry
        .register(Box::new(opencv_tools::ConvertColorTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::ResizeImageTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::BlurImageTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::DetectEdgesTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::FindContoursTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::DetectFacesTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::ApplyThresholdTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::ApplyMorphologyTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::AnalyzeHistogramTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::DrawAnnotationsTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::WarpPerspectiveTool))
        .await;
    state
        .registry
        .register(Box::new(opencv_tools::OpenCvPipelineTool))
        .await;
    // Phase 2 — PixAI image tagger
    state
        .registry
        .register(Box::new(pixai_tagger::TagImageTool))
        .await;
    state
        .registry
        .register(Box::new(pixai_tagger::DescribeImageAnimeTool))
        .await;
    // Phase 2 — NuMarkdown document OCR
    state
        .registry
        .register(Box::new(nu_markdown::ImageToMarkdownTool))
        .await;
    state
        .registry
        .register(Box::new(nu_markdown::ExtractDocStructureTool))
        .await;
    // Phase 2 — Qwen image editing
    state
        .registry
        .register(Box::new(image_edit::EditImageTool))
        .await;
    state
        .registry
        .register(Box::new(image_edit::GenerateImageRapidTool))
        .await;
    state
        .registry
        .register(Box::new(image_edit::GenerateMultiviewTool))
        .await;
    // Phase 2 — Sulphur-2 video generation
    state
        .registry
        .register(Box::new(video_gen::GenerateVideoTool))
        .await;
    state
        .registry
        .register(Box::new(video_gen::EnhanceVideoPromptTool))
        .await;
    // Phase 2 — TRELLIS.2-4B 3D generation
    state
        .registry
        .register(Box::new(threed_gen::Generate3dModelTool))
        .await;
    state
        .registry
        .register(Box::new(threed_gen::Generate3dFromTextTool))
        .await;
}
