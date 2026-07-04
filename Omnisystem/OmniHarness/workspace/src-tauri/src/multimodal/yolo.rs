//! YOLOv8 object detection, pose estimation, and instance segmentation.
//!
//! Wraps a thin Python sidecar (`yolo_worker.py`) that loads the YOLO model
//! once, stays warm, and processes images via JSON stdin/stdout.  The same
//! worker handles both the general YOLOv8 models (Ultralytics) and the
//! stock-market pattern variant (`foduucom/stockmarket-pattern-detection-yolov8`).
//!
//! ## Model placement (offline)
//! General detection:
//!   - `$YOLO_MODEL_PATH`
//!   - `~/.bonsai/models/yolo/yolov8n.pt`   (nano, 3.2 MB)
//!   - `~/.bonsai/models/yolo/yolov8s.onnx` (ONNX export also supported)
//!
//! Stock-market patterns:
//!   - `~/.bonsai/models/yolo/stockmarket-pattern-yolov8.pt`
//!
//! Worker:
//!   - `~/.bonsai/sidecars/yolo_worker.py`
//!   - `sidecars/yolo_worker.py`

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use crate::tool_registry::{Tool, ToolResult};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub class_name: String,
    pub confidence: f32,
    pub class_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseKeypoint {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub objects: Vec<BoundingBox>,
    pub object_count: usize,
    /// Overlay image with boxes drawn, base64 PNG (optional).
    pub overlay_b64: Option<String>,
    pub model: String,
    pub inference_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseResult {
    pub persons: Vec<Vec<PoseKeypoint>>,
    pub person_count: usize,
    pub overlay_b64: Option<String>,
    pub model: String,
    pub inference_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentResult {
    pub instances: Vec<BoundingBox>,
    pub masks_b64: Vec<String>,
    pub instance_count: usize,
    pub model: String,
    pub inference_ms: u64,
}

// ── Discovery ─────────────────────────────────────────────────────────────────

fn find_yolo_model(variant: &str) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("YOLO_MODEL_PATH") {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let base = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/models/yolo");
    let names: &[&str] = match variant {
        "stock" => &[
            "stockmarket-pattern-yolov8.pt",
            "stockmarket-pattern-yolov8.onnx",
        ],
        "pose" => &["yolov8n-pose.pt", "yolov8s-pose.pt", "yolov8n-pose.onnx"],
        "seg" => &["yolov8n-seg.pt", "yolov8s-seg.pt", "yolov8n-seg.onnx"],
        _ => &["yolov8n.pt", "yolov8s.pt", "yolov8n.onnx", "yolov8s.onnx"],
    };
    for name in names {
        let p = base.join(name);
        if p.exists() {
            return Some(p);
        }
        let alt = PathBuf::from("sidecars/yolo").join(name);
        if alt.exists() {
            return Some(alt);
        }
    }
    None
}

fn find_yolo_worker() -> Option<PathBuf> {
    let candidates = [
        dirs::home_dir()
            .unwrap_or_default()
            .join(".bonsai/sidecars/yolo_worker.py"),
        PathBuf::from("sidecars/yolo_worker.py"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub fn is_available() -> bool {
    find_yolo_worker().is_some() && find_yolo_model("detect").is_some()
}

// ── Worker call helper ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct YoloRequest<'a> {
    task: &'a str, // "detect" | "pose" | "segment" | "chart"
    image_path: &'a str,
    model_path: &'a str,
    confidence: f32,
    classes: Option<Vec<String>>,
    draw_overlay: bool,
}

async fn call_worker(req: &YoloRequest<'_>) -> Result<Value, String> {
    let worker =
        find_yolo_worker().ok_or("yolo_worker.py not found. Place it in ~/.bonsai/sidecars/")?;
    let python = which_python()?;
    let payload = serde_json::to_string(req).map_err(|e| e.to_string())?;

    let mut child = tokio::process::Command::new(&python)
        .arg(&worker)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to start yolo_worker: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.as_bytes())
            .await
            .map_err(|e| format!("stdin write: {e}"))?;
    }

    let out = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())
        .await
        .map_err(|_| "yolo_worker timed out (120s)".to_string())?
        .map_err(|e| format!("yolo_worker error: {e}"))?;

    if !out.status.success() {
        return Err(format!("yolo_worker exited {:?}", out.status.code()));
    }

    serde_json::from_slice(&out.stdout).map_err(|e| format!("yolo_worker output parse error: {e}"))
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

fn not_installed_error(model_name: &str, hf_path: &str) -> String {
    format!(
        "{model_name} not found. Download from huggingface.co/{hf_path} \
         and place in ~/.bonsai/models/yolo/"
    )
}

// ── Tool: detect_objects ──────────────────────────────────────────────────────

pub struct DetectObjectsTool {
    _marker: (),
}

impl DetectObjectsTool {
    pub fn new() -> Self {
        Self { _marker: () }
    }
}

#[async_trait]
impl Tool for DetectObjectsTool {
    fn name(&self) -> &str {
        "detect_objects"
    }

    fn description(&self) -> &str {
        "Detect objects in an image using YOLOv8. \
         Args: {image_path: string, confidence?: number (0-1), classes?: string[], draw_overlay?: bool}. \
         Returns: objects[] (x,y,width,height,class_name,confidence), overlay_b64."
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }

        let model = find_yolo_model("detect")
            .ok_or_else(|| not_installed_error("YOLOv8", "Ultralytics/YOLOv8"))?;

        let confidence = args["confidence"].as_f64().unwrap_or(0.25) as f32;
        let classes: Option<Vec<String>> = args["classes"].as_array().map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        });
        let draw_overlay = args["draw_overlay"].as_bool().unwrap_or(false);

        info!(image = image_path, "[yolo] detect_objects");

        let t0 = std::time::Instant::now();
        let result = call_worker(&YoloRequest {
            task: "detect",
            image_path,
            model_path: &model.to_string_lossy(),
            confidence,
            classes,
            draw_overlay,
        })
        .await?;

        let boxes: Vec<BoundingBox> =
            serde_json::from_value(result["objects"].clone()).unwrap_or_default();
        let count = boxes.len();

        let det = DetectionResult {
            objects: boxes,
            object_count: count,
            overlay_b64: result["overlay_b64"].as_str().map(|s| s.to_string()),
            model: model
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("yolov8")
                .to_string(),
            inference_ms: t0.elapsed().as_millis() as u64,
        };

        Ok(ToolResult::json(
            &serde_json::to_value(&det).unwrap_or_default(),
        ))
    }
}

// ── Tool: estimate_pose ───────────────────────────────────────────────────────

pub struct EstimatePoseTool {
    _marker: (),
}

impl EstimatePoseTool {
    pub fn new() -> Self {
        Self { _marker: () }
    }
}

#[async_trait]
impl Tool for EstimatePoseTool {
    fn name(&self) -> &str {
        "estimate_pose"
    }

    fn description(&self) -> &str {
        "Estimate human body pose keypoints using YOLOv8-pose. \
         Args: {image_path: string, confidence?: number, draw_overlay?: bool}. \
         Returns: persons[] with keypoints (nose, eyes, shoulders, elbows, wrists, hips, knees, ankles)."
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }

        let model = find_yolo_model("pose")
            .ok_or_else(|| not_installed_error("YOLOv8-pose", "Ultralytics/YOLOv8"))?;

        let t0 = std::time::Instant::now();
        let result = call_worker(&YoloRequest {
            task: "pose",
            image_path,
            model_path: &model.to_string_lossy(),
            confidence: args["confidence"].as_f64().unwrap_or(0.25) as f32,
            classes: None,
            draw_overlay: args["draw_overlay"].as_bool().unwrap_or(false),
        })
        .await?;

        let persons: Vec<Vec<PoseKeypoint>> =
            serde_json::from_value(result["persons"].clone()).unwrap_or_default();
        let count = persons.len();

        let pose = PoseResult {
            persons,
            person_count: count,
            overlay_b64: result["overlay_b64"].as_str().map(|s| s.to_string()),
            model: model
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("yolov8-pose")
                .to_string(),
            inference_ms: t0.elapsed().as_millis() as u64,
        };

        Ok(ToolResult::json(
            &serde_json::to_value(&pose).unwrap_or_default(),
        ))
    }
}

// ── Tool: segment_objects ─────────────────────────────────────────────────────

pub struct SegmentObjectsTool {
    _marker: (),
}

impl SegmentObjectsTool {
    pub fn new() -> Self {
        Self { _marker: () }
    }
}

#[async_trait]
impl Tool for SegmentObjectsTool {
    fn name(&self) -> &str {
        "segment_objects"
    }

    fn description(&self) -> &str {
        "Instance segmentation using YOLOv8-seg. \
         Args: {image_path: string, confidence?: number}. \
         Returns: instances[] with bounding boxes, masks_b64[] (base64 PNG masks per instance)."
    }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let image_path = args["image_path"].as_str().ok_or("Missing 'image_path'")?;
        if !std::path::Path::new(image_path).exists() {
            return Err(format!("Image not found: {image_path}"));
        }

        let model = find_yolo_model("seg")
            .ok_or_else(|| not_installed_error("YOLOv8-seg", "Ultralytics/YOLOv8"))?;

        let t0 = std::time::Instant::now();
        let result = call_worker(&YoloRequest {
            task: "segment",
            image_path,
            model_path: &model.to_string_lossy(),
            confidence: args["confidence"].as_f64().unwrap_or(0.25) as f32,
            classes: None,
            draw_overlay: false,
        })
        .await?;

        let instances: Vec<BoundingBox> =
            serde_json::from_value(result["instances"].clone()).unwrap_or_default();
        let masks: Vec<String> = result["masks_b64"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let count = instances.len();

        let seg = SegmentResult {
            instances,
            masks_b64: masks,
            instance_count: count,
            model: model
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("yolov8-seg")
                .to_string(),
            inference_ms: t0.elapsed().as_millis() as u64,
        };

        Ok(ToolResult::json(
            &serde_json::to_value(&seg).unwrap_or_default(),
        ))
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn detect_objects_cmd(
    image_path: String,
    confidence: Option<f32>,
    draw_overlay: Option<bool>,
) -> Result<DetectionResult, String> {
    let model = find_yolo_model("detect")
        .ok_or("YOLOv8 model not found. Place yolov8n.pt in ~/.bonsai/models/yolo/")?;

    let t0 = std::time::Instant::now();
    let result = call_worker(&YoloRequest {
        task: "detect",
        image_path: &image_path,
        model_path: &model.to_string_lossy(),
        confidence: confidence.unwrap_or(0.25),
        classes: None,
        draw_overlay: draw_overlay.unwrap_or(false),
    })
    .await?;

    let boxes: Vec<BoundingBox> =
        serde_json::from_value(result["objects"].clone()).unwrap_or_default();
    let count = boxes.len();
    Ok(DetectionResult {
        objects: boxes,
        object_count: count,
        overlay_b64: result["overlay_b64"].as_str().map(|s| s.to_string()),
        model: model
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("yolov8")
            .to_string(),
        inference_ms: t0.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn detect_chart_patterns(image_path: String) -> Result<DetectionResult, String> {
    let model = find_yolo_model("stock")
        .ok_or("Stock-market pattern model not found. Place stockmarket-pattern-yolov8.pt in ~/.bonsai/models/yolo/")?;

    let t0 = std::time::Instant::now();
    let result = call_worker(&YoloRequest {
        task: "detect",
        image_path: &image_path,
        model_path: &model.to_string_lossy(),
        confidence: 0.30,
        classes: None,
        draw_overlay: true,
    })
    .await?;

    let boxes: Vec<BoundingBox> =
        serde_json::from_value(result["objects"].clone()).unwrap_or_default();
    let count = boxes.len();
    Ok(DetectionResult {
        objects: boxes,
        object_count: count,
        overlay_b64: result["overlay_b64"].as_str().map(|s| s.to_string()),
        model: "stockmarket-pattern-yolov8".to_string(),
        inference_ms: t0.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn yolo_available() -> Result<bool, String> {
    Ok(is_available())
}
