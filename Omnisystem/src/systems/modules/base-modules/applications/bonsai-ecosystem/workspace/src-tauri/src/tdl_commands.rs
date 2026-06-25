//! Tauri commands for Training Data Library (TDL) operations.

use tdl::{ExportFormat, Metadata, TrainingDataLibrary};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub version_string: String,
    pub created_by: String,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddExampleRequest {
    pub version_id: String,
    pub content: String,
    pub source: Option<String>,
    pub author: Option<String>,
    pub domain: Option<String>,
    pub tags: Vec<String>,
    pub language: Option<String>,
    pub quality_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportDatasetRequest {
    pub version_id: String,
    pub format: String,
    pub output_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub min_quality: f32,
    pub limit: usize,
}

/// Initialize the Training Data Library.
///
/// # Errors
///
/// Returns error if database cannot be opened.
#[tauri::command]
pub async fn tdl_init(db_path: String) -> Result<serde_json::Value, String> {
    let path = Path::new(&db_path);
    let library = TrainingDataLibrary::new(path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "status": "initialized",
        "db_path": db_path
    }))
}

/// Create a new training data version.
#[tauri::command]
pub async fn tdl_create_version(
    db_path: String,
    request: CreateVersionRequest,
) -> Result<serde_json::Value, String> {
    let path = Path::new(&db_path);
    let library = TrainingDataLibrary::new(path)
        .await
        .map_err(|e| e.to_string())?;

    let version_id = library
        .create_version(
            &request.version_string,
            &request.created_by,
            &request.description,
            request.tags,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "version_id": version_id.to_string(),
        "version_string": request.version_string
    }))
}

/// Add an example to a version.
#[tauri::command]
pub async fn tdl_add_example(
    db_path: String,
    request: AddExampleRequest,
) -> Result<serde_json::Value, String> {
    let path = Path::new(&db_path);
    let library = TrainingDataLibrary::new(path)
        .await
        .map_err(|e| e.to_string())?;

    let version_id =
        Uuid::parse_str(&request.version_id).map_err(|e| format!("invalid version_id: {}", e))?;

    let mut metadata = Metadata::new();
    if let Some(source) = request.source {
        metadata.source = Some(source);
    }
    if let Some(author) = request.author {
        metadata.author = Some(author);
    }
    if let Some(domain) = request.domain {
        metadata.domain = Some(domain);
    }
    if let Some(language) = request.language {
        metadata.language = Some(language);
    }
    metadata.tags = request.tags;

    let example_id = library
        .add_example(version_id, request.content, metadata, request.quality_score)
        .await
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "example_id": example_id.to_string(),
        "version_id": request.version_id
    }))
}

/// Get examples filtered by minimum quality score.
#[tauri::command]
pub async fn tdl_search_by_quality(
    db_path: String,
    request: SearchRequest,
) -> Result<serde_json::Value, String> {
    let path = Path::new(&db_path);
    let library = TrainingDataLibrary::new(path)
        .await
        .map_err(|e| e.to_string())?;

    let examples = library
        .search_by_quality(request.min_quality, request.limit)
        .await
        .map_err(|e| e.to_string())?;

    let results: Vec<_> = examples
        .iter()
        .map(|e| {
            json!({
                "id": e.id.to_string(),
                "content": e.content,
                "quality_score": e.quality_score,
                "metadata": e.metadata,
                "created_at": e.created_at.to_rfc3339()
            })
        })
        .collect();

    Ok(json!({
        "count": results.len(),
        "examples": results
    }))
}

/// Get version history.
#[tauri::command]
pub async fn tdl_get_version_history(db_path: String) -> Result<serde_json::Value, String> {
    let path = Path::new(&db_path);
    let library = TrainingDataLibrary::new(path)
        .await
        .map_err(|e| e.to_string())?;

    let versions = library
        .get_version_history()
        .await
        .map_err(|e| e.to_string())?;

    let results: Vec<_> = versions
        .iter()
        .map(|v| {
            json!({
                "id": v.id.to_string(),
                "version_string": v.version_string,
                "example_count": v.example_count,
                "created_by": v.created_by,
                "created_at": v.created_at.to_rfc3339(),
                "avg_quality_score": v.avg_quality_score
            })
        })
        .collect();

    Ok(json!({
        "versions": results,
        "count": results.len()
    }))
}

/// Export a dataset to JSONL or Parquet format.
#[tauri::command]
pub async fn tdl_export_dataset(
    db_path: String,
    request: ExportDatasetRequest,
) -> Result<serde_json::Value, String> {
    let path = Path::new(&db_path);
    let library = TrainingDataLibrary::new(path)
        .await
        .map_err(|e| e.to_string())?;

    let version_id =
        Uuid::parse_str(&request.version_id).map_err(|e| format!("invalid version_id: {}", e))?;

    let format: ExportFormat = request
        .format
        .parse()
        .map_err(|e: tdl::TdlError| e.to_string())?;

    let output_path = Path::new(&request.output_path);
    let result_path = library
        .export_dataset(version_id, format, output_path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "output_path": result_path.to_string_lossy().to_string(),
        "format": request.format,
        "size_bytes": std::fs::metadata(&result_path)
            .ok()
            .map(|m| m.len())
            .unwrap_or(0)
    }))
}

/// Merge two versions into a new one.
#[tauri::command]
pub async fn tdl_merge_versions(
    db_path: String,
    v1_id: String,
    v2_id: String,
    created_by: String,
) -> Result<serde_json::Value, String> {
    let path = Path::new(&db_path);
    let library = TrainingDataLibrary::new(path)
        .await
        .map_err(|e| e.to_string())?;

    let v1 = Uuid::parse_str(&v1_id).map_err(|e| format!("invalid v1_id: {}", e))?;
    let v2 = Uuid::parse_str(&v2_id).map_err(|e| format!("invalid v2_id: {}", e))?;

    let merged_id = library
        .merge_versions(v1, v2, &created_by)
        .await
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "merged_id": merged_id.to_string(),
        "v1_id": v1_id,
        "v2_id": v2_id
    }))
}
