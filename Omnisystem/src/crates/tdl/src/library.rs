//! High-level Training Data Library API.

use crate::db::TrainingDataDb;
use crate::error::Result;
use crate::models::{Example, Metadata, Version, VersionInfo};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Export format for datasets.
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// JSON Lines format (one JSON object per line)
    Jsonl,
    /// Apache Parquet format
    Parquet,
}

impl ExportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportFormat::Jsonl => "jsonl",
            ExportFormat::Parquet => "parquet",
        }
    }
}

impl std::str::FromStr for ExportFormat {
    type Err = crate::TdlError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "jsonl" => Ok(ExportFormat::Jsonl),
            "parquet" => Ok(ExportFormat::Parquet),
            _ => Err(crate::TdlError::InvalidFormat(s.to_string())),
        }
    }
}

/// High-level API for managing training data.
pub struct TrainingDataLibrary {
    db: TrainingDataDb,
    data_dir: PathBuf,
}

impl TrainingDataLibrary {
    /// Create or open a training data library.
    pub async fn new(db_path: &Path) -> Result<Self> {
        let db = TrainingDataDb::new(db_path).await?;
        let data_dir = db_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

        Ok(TrainingDataLibrary { db, data_dir })
    }

    /// Create a new version with optional tags.
    pub async fn create_version(
        &self,
        version_string: &str,
        created_by: &str,
        description: &str,
        tags: Vec<String>,
    ) -> Result<Uuid> {
        self.db
            .create_version(version_string, created_by, description, tags)
            .await
    }

    /// Add a training example to a version.
    pub async fn add_example(
        &self,
        version_id: Uuid,
        content: String,
        metadata: Metadata,
        quality_score: f32,
    ) -> Result<Uuid> {
        self.db
            .add_example(version_id, content, metadata, quality_score)
            .await
    }

    /// Get examples filtered by tags.
    pub async fn get_examples(
        &self,
        tags: Vec<String>,
        limit: usize,
    ) -> Result<Vec<Example>> {
        self.db.get_examples_by_tags(tags, limit, 0).await
    }

    /// Get examples with minimum quality score.
    pub async fn search_by_quality(
        &self,
        min_score: f32,
        limit: usize,
    ) -> Result<Vec<Example>> {
        self.db.get_examples_by_quality(min_score, limit, 0).await
    }

    /// Export a version to JSONL or Parquet format.
    pub async fn export_dataset(
        &self,
        version_id: Uuid,
        format: ExportFormat,
        output_path: &Path,
    ) -> Result<PathBuf> {
        let examples = self.db.get_version_examples(version_id).await?;

        match format {
            ExportFormat::Jsonl => self.export_jsonl(&examples, output_path).await,
            ExportFormat::Parquet => self.export_parquet(&examples, output_path).await,
        }
    }

    /// Get version history.
    pub async fn get_version_history(&self) -> Result<Vec<VersionInfo>> {
        self.db.get_version_history().await
    }

    /// Get a single version by ID.
    pub async fn get_version(&self, version_id: Uuid) -> Result<Option<Version>> {
        self.db.get_version(version_id).await
    }

    /// Merge two versions into a new version.
    pub async fn merge_versions(
        &self,
        v1_id: Uuid,
        v2_id: Uuid,
        created_by: &str,
    ) -> Result<Uuid> {
        let v1 = self
            .db
            .get_version(v1_id)
            .await?
            .ok_or_else(|| crate::TdlError::VersionNotFound(v1_id.to_string()))?;
        let v2 = self
            .db
            .get_version(v2_id)
            .await?
            .ok_or_else(|| crate::TdlError::VersionNotFound(v2_id.to_string()))?;

        let merged_version_string = format!("{}+{}", v1.version_string, v2.version_string);
        let merged_version_id = self
            .db
            .create_version(
                &merged_version_string,
                created_by,
                &format!("Merged from {} and {}", v1.version_string, v2.version_string),
                Vec::new(),
            )
            .await?;

        // Copy examples from v1
        let v1_examples = self.db.get_version_examples(v1_id).await?;
        for example in v1_examples {
            self.db
                .add_example(
                    merged_version_id,
                    example.content,
                    example.metadata,
                    example.quality_score,
                )
                .await?;
        }

        // Copy examples from v2
        let v2_examples = self.db.get_version_examples(v2_id).await?;
        for example in v2_examples {
            self.db
                .add_example(
                    merged_version_id,
                    example.content,
                    example.metadata,
                    example.quality_score,
                )
                .await?;
        }

        Ok(merged_version_id)
    }

    /// Export to JSONL format.
    async fn export_jsonl(&self, examples: &[Example], output_path: &Path) -> Result<PathBuf> {
        fs::create_dir_all(output_path.parent().unwrap_or_else(|| Path::new(".")))?;

        let mut jsonl_content = String::new();
        for example in examples {
            let obj = serde_json::json!({
                "id": example.id.to_string(),
                "content": example.content,
                "metadata": example.metadata,
                "quality_score": example.quality_score,
                "created_at": example.created_at.to_rfc3339(),
            });
            jsonl_content.push_str(&serde_json::to_string(&obj)?);
            jsonl_content.push('\n');
        }

        fs::write(output_path, jsonl_content)?;
        Ok(output_path.to_path_buf())
    }

    /// Export to Parquet format.
    async fn export_parquet(&self, examples: &[Example], output_path: &Path) -> Result<PathBuf> {
        use arrow::array::{ArrayRef, StringArray};
        use arrow::datatypes::{DataType, Field, Schema};
        use arrow::record_batch::RecordBatch;
        use parquet::arrow::ArrowWriter;
        use std::fs::File;
        use std::sync::Arc;

        fs::create_dir_all(output_path.parent().unwrap_or_else(|| Path::new(".")))?;

        // Build Arrow arrays
        let ids: Vec<String> = examples.iter().map(|e| e.id.to_string()).collect();
        let contents: Vec<String> = examples.iter().map(|e| e.content.clone()).collect();
        let qualities: Vec<f32> = examples.iter().map(|e| e.quality_score).collect();
        let metadatas: Vec<String> = examples
            .iter()
            .map(|e| serde_json::to_string(&e.metadata).unwrap_or_default())
            .collect();

        let id_array = Arc::new(StringArray::from(ids)) as ArrayRef;
        let content_array = Arc::new(StringArray::from(contents)) as ArrayRef;
        let metadata_array = Arc::new(StringArray::from(metadatas)) as ArrayRef;
        let quality_array = Arc::new(arrow::array::Float32Array::from(qualities)) as ArrayRef;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("metadata", DataType::Utf8, false),
            Field::new("quality_score", DataType::Float32, false),
        ]));

        let batch = RecordBatch::try_new(schema.clone(), vec![
            id_array,
            content_array,
            metadata_array,
            quality_array,
        ])?;

        let file = File::create(output_path)?;
        let mut writer = ArrowWriter::try_new(file, schema, None)?;
        writer.write(&batch)?;
        writer.finish()?;

        Ok(output_path.to_path_buf())
    }
}
