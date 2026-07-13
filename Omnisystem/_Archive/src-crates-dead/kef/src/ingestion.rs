//! Ingestion pipeline: convert curated chunks into KDB modules

use crate::{CuratedChunk, KefError, KmodPackage, Result};
use hnsw::{HnswIndex, HnswIndexBuilder, Distance};
use chrono::Utc;
use serde_json::json;
use std::path::Path;
use uuid::Uuid;
use zstd::Encoder;

/// Configuration for ingestion
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Embedding dimension (typically 384, 768, or 1536)
    pub embedding_dim: usize,
    /// HNSW construction parameters
    pub hnsw_m: usize,
    pub hnsw_ef_construction: usize,
    /// Compression: use zstd for values.txt
    pub compress_values: bool,
    /// Batch size for embedding
    pub batch_size: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            embedding_dim: 768,
            hnsw_m: 16,
            hnsw_ef_construction: 200,
            compress_values: true,
            batch_size: 32,
        }
    }
}

/// Default embedding provider (returns random embeddings for testing)
pub struct DummyEmbeddingProvider {
    dim: usize,
}

impl DummyEmbeddingProvider {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    /// Embed a single chunk
    pub fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        // Placeholder: return random vector
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Ok((0..self.dim).map(|_| rng.gen()).collect())
    }

    /// Embed a batch of chunks
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|text| self.embed(text)).collect()
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.dim
    }
}

/// Ingestion pipeline: chunks -> embeddings -> HNSW -> KDB module
pub struct KnowledgeIngestionPipeline {
    config: IngestionConfig,
    embedder: DummyEmbeddingProvider,
}

impl KnowledgeIngestionPipeline {
    /// Create a new ingestion pipeline
    pub fn new(config: IngestionConfig, embedder: DummyEmbeddingProvider) -> Self {
        Self { config, embedder }
    }

    /// Ingest curated chunks into a KDB module
    pub async fn ingest(
        &self,
        chunks: Vec<CuratedChunk>,
        module_name: &str,
        domain: &str,
    ) -> Result<KmodPackage> {
        if chunks.is_empty() {
            return Err(KefError::IngestionFailed("no chunks to ingest".to_string()));
        }

        // Extract text content
        let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();

        // Batch embedding
        let embeddings = self.embed_batch(&texts).await?;

        // Verify embedding dimension
        if !embeddings.is_empty() && embeddings[0].len() != self.config.embedding_dim {
            return Err(KefError::DimensionMismatch {
                expected: self.config.embedding_dim,
                got: embeddings[0].len(),
            });
        }

        // Build HNSW index
        let index = self.build_hnsw(&embeddings)?;

        // Create module package
        let module = KmodPackage {
            id: Uuid::new_v4(),
            name: module_name.to_string(),
            version: "1.0.0".to_string(),
            domain: domain.to_string(),
            entry_count: chunks.len(),
            embedding_dim: self.config.embedding_dim,
            created_at: Utc::now(),
            description: format!(
                "Knowledge module extracted from {} sources",
                chunks.len()
            ),
            extraction_methods: vec!["kef".to_string()],
            avg_quality_score: chunks.iter().map(|c| c.quality_scores.aggregate).sum::<f32>()
                / chunks.len() as f32,
        };

        tracing::info!(
            "Ingestion complete: {} entries, {} dim, quality={:.3}",
            module.entry_count,
            module.embedding_dim,
            module.avg_quality_score
        );

        Ok(module)
    }

    /// Embed a batch of texts
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();

        // Process in batches
        for chunk in texts.chunks(self.config.batch_size) {
            let batch_embeddings = self.embedder.embed_batch(chunk)?;
            embeddings.extend(batch_embeddings);
        }

        Ok(embeddings)
    }

    /// Build HNSW index from embeddings
    fn build_hnsw(&self, embeddings: &[Vec<f32>]) -> Result<HnswIndex> {
        if embeddings.is_empty() {
            return Err(KefError::IngestionFailed(
                "cannot build index from empty embeddings".to_string(),
            ));
        }

        let mut builder = HnswIndexBuilder::new(self.config.embedding_dim, Distance::Cosine);
        builder.ef_construction = self.config.hnsw_ef_construction;
        builder.m = self.config.hnsw_m;

        for (id, embedding) in embeddings.iter().enumerate() {
            builder.add_point(id, embedding)?;
        }

        let index = builder.build()?;
        Ok(index)
    }

    /// Save module to disk
    pub async fn save_module(
        &self,
        module: &KmodPackage,
        chunks: &[CuratedChunk],
        output_dir: &Path,
    ) -> Result<()> {
        // Create module directory
        let module_dir = output_dir.join(&module.id.to_string());
        std::fs::create_dir_all(&module_dir)
            .map_err(|e| KefError::Io(e))?;

        // Save manifest
        let manifest = serde_json::to_string_pretty(&json!({
            "id": module.id,
            "name": module.name,
            "version": module.version,
            "domain": module.domain,
            "entry_count": module.entry_count,
            "embedding_dim": module.embedding_dim,
            "created_at": module.created_at,
            "description": module.description,
            "extraction_methods": module.extraction_methods,
            "avg_quality_score": module.avg_quality_score,
        }))
        .map_err(|e| KefError::SerdeJson(e))?;

        std::fs::write(
            module_dir.join("manifest.json"),
            manifest,
        )
        .map_err(|e| KefError::Io(e))?;

        // Save values (compressed or not)
        let values_content = chunks
            .iter()
            .map(|c| c.content.clone())
            .collect::<Vec<_>>()
            .join("\n");

        if self.config.compress_values {
            let values_path = module_dir.join("values.txt.zst");
            let file = std::fs::File::create(&values_path)
                .map_err(|e| KefError::Io(e))?;
            let mut encoder = Encoder::new(file)
                .map_err(|e| KefError::Compression(e.to_string()))?;
            use std::io::Write;
            encoder.write_all(values_content.as_bytes())
                .map_err(|e| KefError::Io(e))?;
            encoder.finish()
                .map_err(|e| KefError::Compression(e.to_string()))?;
        } else {
            std::fs::write(
                module_dir.join("values.txt"),
                &values_content,
            )
            .map_err(|e| KefError::Io(e))?;
        }

        // Save metadata
        let metadata = chunks
            .iter()
            .map(|c| {
                json!({
                    "quality_scores": c.quality_scores,
                    "pii_redacted": c.pii_redacted,
                    "extraction_method": c.extraction_method,
                    "tags": c.tags,
                })
            })
            .collect::<Vec<_>>();

        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| KefError::SerdeJson(e))?;

        std::fs::write(
            module_dir.join("metadata.json"),
            metadata_json,
        )
        .map_err(|e| KefError::Io(e))?;

        tracing::info!("Module saved to: {}", module_dir.display());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_embedder() {
        let embedder = DummyEmbeddingProvider::new(768);
        assert_eq!(embedder.dimension(), 768);
    }

    #[tokio::test]
    async fn test_embed_batch() -> Result<()> {
        let embedder = DummyEmbeddingProvider::new(256);
        let texts = vec!["hello", "world"];
        let embeddings = embedder.embed_batch(&texts)?;

        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 256);

        Ok(())
    }

    #[tokio::test]
    async fn test_ingestion_pipeline() -> Result<()> {
        let config = IngestionConfig::default();
        let embedder = DummyEmbeddingProvider::new(config.embedding_dim);
        let pipeline = KnowledgeIngestionPipeline::new(config, embedder);

        let chunk = CuratedChunk {
            content: "Test knowledge content".to_string(),
            quality_scores: crate::QualityScores {
                relevance: 0.8,
                accuracy: 0.9,
                clarity: 0.85,
                uniqueness: 0.7,
                aggregate: 0.8,
            },
            pii_redacted: false,
            extraction_method: "test".to_string(),
            source_model: "test-model".to_string(),
            extracted_at: Utc::now(),
            tags: vec![],
        };

        let module = pipeline.ingest(vec![chunk], "test_module", "test_domain").await?;

        assert_eq!(module.entry_count, 1);
        assert_eq!(module.embedding_dim, 768);

        Ok(())
    }
}
