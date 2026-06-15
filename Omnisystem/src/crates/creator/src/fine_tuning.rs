//! FineTuningActor — LoRA / DPO fine-tuning of generative models.
//!
//! Preference data (image pairs, captions) is fetched from CAS; the actor
//! runs the training loop (candle or external sidecar) and writes the
//! resulting LoRA adapter weights back to CAS.

use cas::{CasKey, CasStore};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

pub struct FineTuningActor {
    pub cas: Arc<CasStore>,
}

impl FineTuningActor {
    pub fn new(cas: Arc<CasStore>) -> Self {
        Self { cas }
    }

    /// Start a LoRA/DPO fine-tuning job.
    ///
    /// - `base_model`: name of the checkpoint to fine-tune (e.g. "flux.1-dev")
    /// - `preference_cas_keys`: CAS keys of preference datasets (image pairs, captions)
    /// - `epochs`: number of full training epochs (1–50)
    ///
    /// Returns the CAS key of the trained adapter when done.
    pub async fn start_lora_job(
        &self,
        base_model: String,
        preference_cas_keys: Vec<CasKey>,
        epochs: u32,
    ) -> anyhow::Result<CasKey> {
        if preference_cas_keys.is_empty() {
            return Err(anyhow::anyhow!("no preference data provided"));
        }
        if epochs == 0 || epochs > 50 {
            return Err(anyhow::anyhow!("epochs must be 1–50"));
        }

        // Verify all dataset keys exist before we start.
        for key in &preference_cas_keys {
            if !self
                .cas
                .exists(key)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?
            {
                return Err(anyhow::anyhow!(
                    "preference dataset key not found: {}",
                    key.hex()
                ));
            }
        }

        info!(
            "LoRA job: model={base_model} epochs={epochs} datasets={}",
            preference_cas_keys.len()
        );

        // 1-hour hard cap on any single training run.
        let adapter_key = timeout(
            Duration::from_secs(3600),
            self.run_training(base_model, preference_cas_keys, epochs),
        )
        .await
        .map_err(|_| anyhow::anyhow!("training timed out after 1 hour"))??;

        Ok(adapter_key)
    }

    /// DPO variant — takes a (winner, loser) pair format in the dataset.
    pub async fn start_dpo_job(
        &self,
        base_model: String,
        dataset_key: CasKey,
        epochs: u32,
    ) -> anyhow::Result<CasKey> {
        self.start_lora_job(base_model, vec![dataset_key], epochs)
            .await
    }

    async fn run_training(
        &self,
        base_model: String,
        keys: Vec<CasKey>,
        epochs: u32,
    ) -> anyhow::Result<CasKey> {
        // === Production path (stubbed) ===
        // 1. Load base model weights from local model cache
        // 2. Load preference datasets from CAS
        // 3. Run LoRA training with candle or subprocess
        // 4. Store adapter weights in CAS

        for epoch in 1..=epochs {
            tokio::time::sleep(Duration::from_millis(200)).await;
            info!(
                "  epoch {epoch}/{epochs} — base={base_model} datasets={}",
                keys.len()
            );
        }

        // Placeholder adapter bytes tagged with training metadata
        let adapter_data = format!(
            "LoRA adapter | base={base_model} epochs={epochs} datasets={}",
            keys.iter().map(|k| k.hex()).collect::<Vec<_>>().join(",")
        );
        let adapter_key = self
            .cas
            .put(adapter_data.as_bytes(), "application/octet-stream")
            .await
            .map_err(|e| anyhow::anyhow!("CAS put adapter: {e}"))?;

        info!("LoRA training complete → adapter key={}", adapter_key.hex());
        Ok(adapter_key)
    }
}
