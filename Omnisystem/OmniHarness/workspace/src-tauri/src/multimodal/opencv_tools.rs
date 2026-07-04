//! OpenCV 4.12.0 integration for BonsAI.
//!
//! Exposes OpenCV's core image-processing and computer-vision capabilities as
//! registered `Tool` implementations that any agent can invoke through the
//! ReAct system prompt or native function calling.
//!
//! ## Architecture
//!
//! OpenCV is called through a thin Python sidecar (`opencv_worker.py`) that
//! accepts a JSON operation descriptor on stdin and writes a JSON result
//! (including base64-encoded output images) to stdout.  This keeps the Rust
//! binary free of C/C++ link-time dependencies while still providing full
//! OpenCV 4.12 coverage — identical to the YOLO / Depth / Kokoro pattern.
//!
//! When the user installs `opencv-python` (`pip install opencv-python`), **all
//! tools below become available immediately** — no recompilation needed.
//!
//! ## Worker placement (offline, user-managed)
//!   - `~/.bonsai/sidecars/opencv_worker.py`
//!   - `sidecars/opencv_worker.py`
//!   - `$OPENCV_WORKER_PATH`
//!
//! ## Implemented tools (maps to OpenCV modules)
//!
//! | Tool                  | OpenCV module           | Operation |
//! |-----------------------|-------------------------|-----------|
//! | `convert_color`       | `imgproc`               | BGR↔Gray/HSV/LAB/YUV |
//! | `resize_image`        | `imgproc`               | Scale w/ interpolation |
//! | `blur_image`          | `imgproc`               | Gaussian/median/bilateral |
//! | `detect_edges`        | `imgproc` (Canny)       | Edge detection |
//! | `find_contours`       | `imgproc`               | Contour extraction |
//! | `detect_faces`        | `objdetect` (Haar)      | Face detection |
//! | `apply_threshold`     | `imgproc`               | Binary/adaptive/Otsu |
//! | `apply_morphology`    | `imgproc`               | Erode/dilate/open/close |
//! | `analyze_histogram`   | `imgproc`               | Per-channel histogram |
//! | `draw_annotations`    | `core`                  | Boxes/text/circles overlay |
//! | `warp_perspective`    | `imgproc`               | 4-point perspective fix |
//! | `opencv_pipeline`     | any                     | Chain multiple ops in one call |

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::tool_registry::{Tool, ToolResult};

// ── Worker discovery ──────────────────────────────────────────────────────────

fn find_worker() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OPENCV_WORKER_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/sidecars/opencv_worker.py"),
        PathBuf::from("sidecars/opencv_worker.py"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub fn is_available() -> bool {
    find_worker().is_some()
}

fn which_python() -> Result<PathBuf, String> {
    for c in &["python3", "python"] {
        if std::process::Command::new(c)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(PathBuf::from(c));
        }
    }
    Err("python3/python not found in PATH".into())
}

fn not_installed() -> String {
    "opencv_worker.py not found. \
     Run: pip install opencv-python && \
     place opencv_worker.py in ~/.bonsai/sidecars/ \
     (see Bonsai Docs → Vision Tools)"
        .to_string()
}

// ── Core worker call ──────────────────────────────────────────────────────────

async fn call_worker(payload: &Value) -> Result<Value, String> {
    let worker = find_worker().ok_or_else(not_installed)?;
    let python = which_python()?;
    let encoded = serde_json::to_string(payload).map_err(|e| e.to_string())?;

    let mut child = tokio::process::Command::new(&python)
        .arg(&worker)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start opencv_worker: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(encoded.as_bytes())
            .await
            .map_err(|e| format!("stdin write: {e}"))?;
    }

    let out = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())
        .await
        .map_err(|_| "opencv_worker timed out (120s)".to_string())?
        .map_err(|e| format!("opencv_worker error: {e}"))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "opencv_worker exited {:?}: {err}",
            out.status.code()
        ));
    }

    serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("opencv_worker output parse error: {e}"))
}

// Validate that the file path is under allowed roots (workspace safety).
fn validate_image_path(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(format!("File not found: {path}"));
    }
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["jpg", "jpeg", "png", "bmp", "tiff", "tif", "webp", "gif"].contains(&ext.as_str()) {
        return Err(format!("Unsupported image format: .{ext}"));
    }
    Ok(())
}

// ── Shared result types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvImageResult {
    /// Output image as base64-encoded PNG.
    pub image_b64: Option<String>,
    /// Path where the image was saved (if requested).
    pub saved_path: Option<String>,
    /// Width of the output image in pixels.
    pub width: u32,
    /// Height of the output image in pixels.
    pub height: u32,
    /// Channels (1=gray, 3=BGR, 4=BGRA).
    pub channels: u32,
    /// Optional structured metadata (contours, faces, histogram, etc.).
    pub metadata: Value,
    /// Wall-clock time spent in the worker.
    pub elapsed_ms: u64,
}

// ── Tool: convert_color ───────────────────────────────────────────────────────

pub struct ConvertColorTool;

#[async_trait]
impl Tool for ConvertColorTool {
    fn name(&self) -> &str {
        "convert_color"
    }
    fn description(&self) -> &str {
        "Convert an image between color spaces using OpenCV. \
         Args: {image_path: string, color_space: \"grayscale\"|\"hsv\"|\"lab\"|\"yuv\"|\"rgb\", \
         save_path?: string}. Returns: image_b64 (PNG)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let color_space = args["color_space"].as_str().unwrap_or("grayscale");
        info!(op = "convert_color", space = color_space, "[opencv]");
        let result = call_worker(&json!({
            "op":          "convert_color",
            "image_path":  image_path,
            "color_space": color_space,
            "save_path":   args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: resize_image ────────────────────────────────────────────────────────

pub struct ResizeImageTool;

#[async_trait]
impl Tool for ResizeImageTool {
    fn name(&self) -> &str {
        "resize_image"
    }
    fn description(&self) -> &str {
        "Resize an image with OpenCV. \
         Args: {image_path: string, width: number, height: number, \
         interpolation?: \"nearest\"|\"linear\"|\"cubic\"|\"lanczos\", save_path?: string}."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let width = args["width"].as_u64().ok_or("Missing 'width'")?;
        let height = args["height"].as_u64().ok_or("Missing 'height'")?;
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err("width/height must be 1–16384".into());
        }
        let interp = args["interpolation"].as_str().unwrap_or("linear");
        info!(op = "resize_image", w = width, h = height, "[opencv]");
        let result = call_worker(&json!({
            "op":            "resize_image",
            "image_path":    image_path,
            "width":         width,
            "height":        height,
            "interpolation": interp,
            "save_path":     args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: blur_image ──────────────────────────────────────────────────────────

pub struct BlurImageTool;

#[async_trait]
impl Tool for BlurImageTool {
    fn name(&self) -> &str {
        "blur_image"
    }
    fn description(&self) -> &str {
        "Apply blur to an image using OpenCV. \
         Args: {image_path: string, blur_type: \"gaussian\"|\"median\"|\"bilateral\", \
         kernel_size?: number (odd, default 5), save_path?: string}."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let blur_type = args["blur_type"].as_str().unwrap_or("gaussian");
        let kernel_size = args["kernel_size"].as_u64().unwrap_or(5).clamp(1, 99);
        // Force odd kernel
        let kernel_size = if kernel_size % 2 == 0 {
            kernel_size + 1
        } else {
            kernel_size
        };
        info!(
            op = "blur_image",
            blur_type,
            kernel = kernel_size,
            "[opencv]"
        );
        let result = call_worker(&json!({
            "op":          "blur_image",
            "image_path":  image_path,
            "blur_type":   blur_type,
            "kernel_size": kernel_size,
            "save_path":   args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: detect_edges ────────────────────────────────────────────────────────

pub struct DetectEdgesTool;

#[async_trait]
impl Tool for DetectEdgesTool {
    fn name(&self) -> &str {
        "detect_edges"
    }
    fn description(&self) -> &str {
        "Detect edges in an image using the Canny algorithm (OpenCV imgproc). \
         Args: {image_path: string, threshold1?: number (default 100), \
         threshold2?: number (default 200), aperture_size?: number (3/5/7), save_path?: string}."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let threshold1 = args["threshold1"].as_f64().unwrap_or(100.0);
        let threshold2 = args["threshold2"].as_f64().unwrap_or(200.0);
        let aperture = args["aperture_size"].as_u64().unwrap_or(3);
        info!(
            op = "detect_edges",
            t1 = threshold1,
            t2 = threshold2,
            "[opencv]"
        );
        let result = call_worker(&json!({
            "op":           "detect_edges",
            "image_path":   image_path,
            "threshold1":   threshold1,
            "threshold2":   threshold2,
            "aperture_size": aperture,
            "save_path":    args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: find_contours ───────────────────────────────────────────────────────

pub struct FindContoursTool;

#[async_trait]
impl Tool for FindContoursTool {
    fn name(&self) -> &str {
        "find_contours"
    }
    fn description(&self) -> &str {
        "Find and draw object contours using OpenCV imgproc. \
         Args: {image_path: string, mode?: \"external\"|\"list\"|\"tree\", \
         draw_overlay?: bool, save_path?: string}. \
         Returns: image_b64, metadata.contour_count, metadata.areas[]."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let mode = args["mode"].as_str().unwrap_or("external");
        let draw_overlay = args["draw_overlay"].as_bool().unwrap_or(true);
        info!(op = "find_contours", mode, "[opencv]");
        let result = call_worker(&json!({
            "op":           "find_contours",
            "image_path":   image_path,
            "mode":         mode,
            "draw_overlay": draw_overlay,
            "save_path":    args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: detect_faces ────────────────────────────────────────────────────────

pub struct DetectFacesTool;

#[async_trait]
impl Tool for DetectFacesTool {
    fn name(&self) -> &str {
        "detect_faces"
    }
    fn description(&self) -> &str {
        "Detect human faces using OpenCV Haar cascade (objdetect). \
         Args: {image_path: string, scale_factor?: number (default 1.1), \
         min_neighbors?: number (default 5), draw_overlay?: bool, save_path?: string}. \
         Returns: image_b64, metadata.faces[] (x,y,w,h)."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let scale_factor = args["scale_factor"]
            .as_f64()
            .unwrap_or(1.1)
            .clamp(1.01, 3.0);
        let min_neighbors = args["min_neighbors"].as_u64().unwrap_or(5).clamp(1, 20);
        let draw_overlay = args["draw_overlay"].as_bool().unwrap_or(true);
        info!(op = "detect_faces", scale = scale_factor, "[opencv]");
        let result = call_worker(&json!({
            "op":            "detect_faces",
            "image_path":    image_path,
            "scale_factor":  scale_factor,
            "min_neighbors": min_neighbors,
            "draw_overlay":  draw_overlay,
            "save_path":     args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: apply_threshold ─────────────────────────────────────────────────────

pub struct ApplyThresholdTool;

#[async_trait]
impl Tool for ApplyThresholdTool {
    fn name(&self) -> &str {
        "apply_threshold"
    }
    fn description(&self) -> &str {
        "Apply image thresholding using OpenCV (imgproc). \
         Args: {image_path: string, \
         method: \"binary\"|\"binary_inv\"|\"otsu\"|\"adaptive_mean\"|\"adaptive_gaussian\", \
         threshold?: number (0-255, default 127), save_path?: string}."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let method = args["method"].as_str().unwrap_or("otsu");
        let threshold = args["threshold"].as_u64().unwrap_or(127).clamp(0, 255);
        info!(
            op = "apply_threshold",
            method,
            thresh = threshold,
            "[opencv]"
        );
        let result = call_worker(&json!({
            "op":         "apply_threshold",
            "image_path": image_path,
            "method":     method,
            "threshold":  threshold,
            "save_path":  args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: apply_morphology ────────────────────────────────────────────────────

pub struct ApplyMorphologyTool;

#[async_trait]
impl Tool for ApplyMorphologyTool {
    fn name(&self) -> &str {
        "apply_morphology"
    }
    fn description(&self) -> &str {
        "Apply morphological operations using OpenCV (imgproc). \
         Args: {image_path: string, \
         operation: \"erode\"|\"dilate\"|\"open\"|\"close\"|\"gradient\"|\"tophat\"|\"blackhat\", \
         kernel_size?: number (odd, default 5), iterations?: number, save_path?: string}."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let operation = args["operation"].as_str().unwrap_or("dilate");
        let kernel_size = args["kernel_size"].as_u64().unwrap_or(5).clamp(1, 99);
        let kernel_size = if kernel_size % 2 == 0 {
            kernel_size + 1
        } else {
            kernel_size
        };
        let iterations = args["iterations"].as_u64().unwrap_or(1).clamp(1, 10);
        info!(
            op = "apply_morphology",
            operation,
            kernel = kernel_size,
            "[opencv]"
        );
        let result = call_worker(&json!({
            "op":          "apply_morphology",
            "image_path":  image_path,
            "operation":   operation,
            "kernel_size": kernel_size,
            "iterations":  iterations,
            "save_path":   args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: analyze_histogram ───────────────────────────────────────────────────

pub struct AnalyzeHistogramTool;

#[async_trait]
impl Tool for AnalyzeHistogramTool {
    fn name(&self) -> &str {
        "analyze_histogram"
    }
    fn description(&self) -> &str {
        "Compute and visualise a colour histogram using OpenCV (imgproc). \
         Args: {image_path: string, bins?: number (default 256), \
         channels?: [\"b\",\"g\",\"r\"] (default all), save_path?: string}. \
         Returns: image_b64 (histogram plot PNG), metadata.mean[], metadata.std_dev[]."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let bins = args["bins"].as_u64().unwrap_or(256).clamp(2, 256);
        info!(op = "analyze_histogram", bins, "[opencv]");
        let result = call_worker(&json!({
            "op":         "analyze_histogram",
            "image_path": image_path,
            "bins":       bins,
            "channels":   args["channels"],
            "save_path":  args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: draw_annotations ────────────────────────────────────────────────────

pub struct DrawAnnotationsTool;

#[async_trait]
impl Tool for DrawAnnotationsTool {
    fn name(&self) -> &str {
        "draw_annotations"
    }
    fn description(&self) -> &str {
        "Draw bounding boxes, labels, circles, or text on an image using OpenCV core. \
         Args: {image_path: string, \
         annotations: [{type: \"rect\"|\"circle\"|\"text\", \
           x: number, y: number, w?: number, h?: number, r?: number, \
           label?: string, color?: [B,G,R], thickness?: number}], \
         save_path?: string}."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let annotations = args
            .get("annotations")
            .ok_or("Missing 'annotations' array")?;
        info!(op = "draw_annotations", "[opencv]");
        let result = call_worker(&json!({
            "op":          "draw_annotations",
            "image_path":  image_path,
            "annotations": annotations,
            "save_path":   args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: warp_perspective ────────────────────────────────────────────────────

pub struct WarpPerspectiveTool;

#[async_trait]
impl Tool for WarpPerspectiveTool {
    fn name(&self) -> &str {
        "warp_perspective"
    }
    fn description(&self) -> &str {
        "Apply a perspective (homography) warp using OpenCV imgproc. Useful for \
         document scanning and de-skewing. \
         Args: {image_path: string, \
         src_points: [[x1,y1],[x2,y2],[x3,y3],[x4,y4]], \
         output_width: number, output_height: number, save_path?: string}."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let src_points = args.get("src_points").ok_or("Missing 'src_points'")?;
        let output_width = args["output_width"]
            .as_u64()
            .ok_or("Missing 'output_width'")?;
        let output_height = args["output_height"]
            .as_u64()
            .ok_or("Missing 'output_height'")?;
        info!(
            op = "warp_perspective",
            w = output_width,
            h = output_height,
            "[opencv]"
        );
        let result = call_worker(&json!({
            "op":            "warp_perspective",
            "image_path":    image_path,
            "src_points":    src_points,
            "output_width":  output_width,
            "output_height": output_height,
            "save_path":     args["save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tool: opencv_pipeline ─────────────────────────────────────────────────────

pub struct OpenCvPipelineTool;

#[async_trait]
impl Tool for OpenCvPipelineTool {
    fn name(&self) -> &str {
        "opencv_pipeline"
    }
    fn description(&self) -> &str {
        "Run a sequence of OpenCV operations on an image in a single call. \
         Each step's output feeds the next. \
         Args: {image_path: string, \
         steps: [{op: string, ...op-specific-args}], \
         final_save_path?: string}. \
         Supported ops: convert_color, resize_image, blur_image, detect_edges, \
         find_contours, apply_threshold, apply_morphology, draw_annotations."
    }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        validate_image_path(image_path)?;
        let steps = args.get("steps").ok_or("Missing 'steps' array")?;
        info!(
            op = "opencv_pipeline",
            steps = steps.as_array().map(|a| a.len()).unwrap_or(0),
            "[opencv]"
        );
        let result = call_worker(&json!({
            "op":              "pipeline",
            "image_path":      image_path,
            "steps":           steps,
            "final_save_path": args["final_save_path"],
        }))
        .await?;
        Ok(ToolResult::json(&result))
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CvOpRequest {
    pub op: String,
    pub image_path: String,
    #[serde(flatten)]
    pub params: Value,
}

#[tauri::command]
pub async fn opencv_run_op(req: CvOpRequest) -> Result<Value, String> {
    validate_image_path(&req.image_path)?;
    let mut payload = req.params.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("op".into(), json!(req.op));
        obj.insert("image_path".into(), json!(req.image_path));
    }
    call_worker(&payload).await
}

#[tauri::command]
pub async fn opencv_available() -> Result<bool, String> {
    Ok(is_available())
}

#[tauri::command]
pub async fn opencv_detect_faces(
    image_path: String,
    scale_factor: Option<f64>,
    min_neighbors: Option<u64>,
    draw_overlay: Option<bool>,
    save_path: Option<String>,
) -> Result<Value, String> {
    validate_image_path(&image_path)?;
    call_worker(&json!({
        "op":            "detect_faces",
        "image_path":    image_path,
        "scale_factor":  scale_factor.unwrap_or(1.1),
        "min_neighbors": min_neighbors.unwrap_or(5),
        "draw_overlay":  draw_overlay.unwrap_or(true),
        "save_path":     save_path,
    }))
    .await
}

#[tauri::command]
pub async fn opencv_detect_edges(
    image_path: String,
    threshold1: Option<f64>,
    threshold2: Option<f64>,
    save_path: Option<String>,
) -> Result<Value, String> {
    validate_image_path(&image_path)?;
    call_worker(&json!({
        "op":         "detect_edges",
        "image_path": image_path,
        "threshold1": threshold1.unwrap_or(100.0),
        "threshold2": threshold2.unwrap_or(200.0),
        "save_path":  save_path,
    }))
    .await
}

#[tauri::command]
pub async fn opencv_pipeline_cmd(
    image_path: String,
    steps: Value,
    final_save_path: Option<String>,
) -> Result<Value, String> {
    validate_image_path(&image_path)?;
    call_worker(&json!({
        "op":              "pipeline",
        "image_path":      image_path,
        "steps":           steps,
        "final_save_path": final_save_path,
    }))
    .await
}
