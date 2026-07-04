//! Main Knowledge Extraction Fabric service orchestrator

use crate::{
    activation_extractor::{ActivationExtractor, ActivationExtractorConfig},
    attention_extractor::{AttentionExtractor, AttentionExtractorConfig},
    curator::{Curator, CuratorConfig},
    ingestion::{KnowledgeIngestionPipeline, IngestionConfig, DummyEmbeddingProvider},
    membership_inference::{MembershipInference, MembershipInferenceConfig},
    model_scanner::ModelScanner,
    quality_scorer::QualityScorer,
    synthetic_generator::{SyntheticDataGenerator, SyntheticGeneratorConfig},
    CuratedChunk, ExtractionMethod, ExtractionReport, KefError, KmodPackage, Result,
};
use crossbeam_channel::bounded;
use std::path::Path;
use std::time::Instant;
use tracing::{info, warn};

/// Main Knowledge Extraction Fabric service
pub struct KefService {
    curator_config: CuratorConfig,
    ingestion_config: IngestionConfig,
}

impl KefService {
    /// Create a new KEF service
    pub fn new() -> Self {
        Self {
            curator_config: CuratorConfig::default(),
            ingestion_config: IngestionConfig::default(),
        }
    }

    /// Configure the curator
    pub fn with_curator_config(mut self, config: CuratorConfig) -> Self {
        self.curator_config = config;
        self
    }

    /// Configure ingestion
    pub fn with_ingestion_config(mut self, config: IngestionConfig) -> Self {
        self.ingestion_config = config;
        self
    }

    /// Extract knowledge from a model using specified methods
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to model file
    /// * `methods` - Extraction methods to use
    /// * `output_dir` - Directory to save KDB modules
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails
    pub async fn extract_knowledge(
        &self,
        model_path: &Path,
        methods: Vec<ExtractionMethod>,
        output_dir: &Path,
    ) -> Result<ExtractionReport> {
        let start = Instant::now();
        let mut report = ExtractionReport::new();

        info!("KEF: Starting knowledge extraction from {:?}", model_path);

        // 1. Scan model
        let model_report = ModelScanner::scan(model_path).await?;
        info!(
            "KEF: Model scanned - type: {}, params: {}",
            model_report.model_type, model_report.parameter_count
        );

        // 2. Run extraction methods
        let mut all_chunks = Vec::new();

        for method in &methods {
            info!("KEF: Running extraction method: {}", method);
            match method {
                ExtractionMethod::Synthetic => {
                    let generator = SyntheticDataGenerator::new(SyntheticGeneratorConfig::default());
                    match generator.generate_from_topics().await {
                        Ok(chunks) => {
                            info!("KEF: Synthetic generation yielded {} chunks", chunks.len());
                            all_chunks.extend(
                                chunks
                                    .into_iter()
                                    .map(|content| {
                                        CuratedChunk::new(
                                            content,
                                            model_report.model_name.clone(),
                                            "synthetic".to_string(),
                                        )
                                    })
                                    .collect::<Vec<_>>(),
                            );
                        }
                        Err(e) => {
                            warn!("KEF: Synthetic generation failed: {}", e);
                            report.errors.push(format!("Synthetic: {}", e));
                        }
                    }
                }
                ExtractionMethod::Activation => {
                    let _extractor =
                        ActivationExtractor::new(ActivationExtractorConfig::default());
                    // In production: hook into model forward pass
                    info!("KEF: Activation extraction (placeholder)");
                }
                ExtractionMethod::Attention => {
                    let _extractor = AttentionExtractor::default();
                    // In production: extract attention weights during inference
                    info!("KEF: Attention extraction (placeholder)");
                }
                ExtractionMethod::MembershipInference => {
                    let _inference = MembershipInference::new(MembershipInferenceConfig::default());
                    // In production: run membership inference attack
                    info!("KEF: Membership inference (placeholder)");
                }
            }

            report.methods_used.push(method.to_string());
        }

        report.total_extracted = all_chunks.len();
        info!("KEF: Total chunks extracted: {}", report.total_extracted);

        if all_chunks.is_empty() {
            warn!("KEF: No chunks extracted");
            report.duration_secs = start.elapsed().as_secs_f64();
            return Ok(report);
        }

        // 3. Curator: dedup + PII + quality
        let texts: Vec<String> = all_chunks.iter().map(|c| c.content.clone()).collect();
        let mut curator = Curator::new(self.curator_config.clone());
        let curated = curator.process(texts).await?;

        report.deduplicated = curated.len();
        report.pii_redacted = curated.iter().filter(|c| c.pii_redacted).count();
        report.quality_passed = curated.len();
        report.avg_quality = curated.iter().map(|c| c.quality_scores.aggregate).sum::<f32>()
            / curated.len().max(1) as f32;

        info!(
            "KEF: Curation complete - {} deduplicated, {} PII-redacted, quality: {:.3}",
            report.deduplicated, report.pii_redacted, report.avg_quality
        );

        // 4. Ingest into KDB module
        let embedder = DummyEmbeddingProvider::new(
            self.ingestion_config.embedding_dim,
        );
        let pipeline = KnowledgeIngestionPipeline::new(
            self.ingestion_config.clone(),
            embedder,
        );

        let module = pipeline
            .ingest(
                curated.clone(),
                &format!("extracted_{}", model_report.model_name),
                &model_report.model_type.to_string(),
            )
            .await?;

        // Save module
        std::fs::create_dir_all(output_dir).map_err(|e| KefError::Io(e))?;
        pipeline
            .save_module(&module, &curated, output_dir)
            .await?;

        report.modules.push(module.name.clone());
        report.duration_secs = start.elapsed().as_secs_f64();

        info!(
            "KEF: Extraction complete - {} module(s), {:.2}s elapsed",
            report.modules.len(),
            report.duration_secs
        );

        Ok(report)
    }

    /// Extract with progress reporting via channel
    pub async fn extract_knowledge_with_progress(
        &self,
        model_path: &Path,
        methods: Vec<ExtractionMethod>,
        output_dir: &Path,
    ) -> Result<(ExtractionReport, crossbeam_channel::Receiver<String>)> {
        let (tx, rx) = bounded::<String>(100);

        let _ = tx.try_send("Starting extraction...".to_string());

        let report = self
            .extract_knowledge(model_path, methods, output_dir)
            .await?;

        let _ = tx.try_send(format!("Extraction complete: {:?}", report.modules));

        Ok((report, rx))
    }
}

impl Default for KefService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_service_creation() {
        let service = KefService::new();
        // Should not panic
    }

    #[test]
    fn test_config_builders() {
        let service = KefService::new()
            .with_curator_config(CuratorConfig::default())
            .with_ingestion_config(IngestionConfig::default());

        // Verify builders work
        assert_eq!(service.ingestion_config.embedding_dim, 768);
    }

    #[tokio::test]
    async fn test_extraction_with_nonexistent_model() -> Result<()> {
        let service = KefService::new();
        let output_dir = TempDir::new().map_err(|e| KefError::Io(e))?;

        let result = service
            .extract_knowledge(
                Path::new("/nonexistent/model.gguf"),
                vec![ExtractionMethod::Synthetic],
                output_dir.path(),
            )
            .await;

        // Should fail gracefully
        assert!(result.is_err());

        Ok(())
    }
}
