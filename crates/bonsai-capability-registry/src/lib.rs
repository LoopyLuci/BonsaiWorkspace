//! Universal Capability Registry
//!
//! Minimal, dependency-light implementation for Phase 0.

pub mod trust_score;
pub use trust_score::{
    TrustScore, ProofToken, DeploymentGate, GateResult,
    GATE_DEV, GATE_STAGING, GATE_PRODUCTION, GATE_SAFETY_CRITICAL,
    effect_penalty,
};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

/// Effects that tools may declare they require.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BonsaiEffect {
    FileIO,
    NetworkIO,
    ModelInference,
    GpuAccess,
    ShellExec,
    Spawn,
    ReadUserData,
    WriteUserData,
    Telemetry,
    Crypto,
    AudioCapture,
    VideoCapture,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EffectRow {
    pub effects: Vec<BonsaiEffect>,
}

impl EffectRow {
    pub fn permits(&self, effect: &BonsaiEffect) -> bool {
        self.effects.contains(effect)
    }

    pub fn union(&self, other: &EffectRow) -> EffectRow {
        let mut effects = self.effects.clone();
        for e in &other.effects {
            if !effects.contains(e) {
                effects.push(e.clone());
            }
        }
        EffectRow { effects }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEntry {
    pub name: String,
    pub id: String,
    pub category: String,
    pub description: Option<String>,
    pub trigger_phrases: Vec<String>,
    pub capability_tags: Vec<String>,
    pub parameters: serde_json::Value,
    pub examples: Vec<serde_json::Value>,
    pub requires_model: Option<String>,
    pub effect_row: EffectRow,
    pub trust_level: String,
    pub availability: Option<serde_json::Value>,
    pub version: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub manifest_version: String,
    pub generated_at: String,
    pub summary: String,
    pub capabilities: HashMap<String, Vec<CapabilityEntry>>,
    pub generation: u64,
    pub checksum: String,
}

impl CapabilityManifest {
    pub fn new_empty() -> Self {
        Self {
            manifest_version: "1.0".to_string(),
            generated_at: format!("{}", chrono::Utc::now()),
            summary: String::new(),
            capabilities: HashMap::new(),
            generation: 0,
            checksum: String::new(),
        }
    }
}

/// CapabilitySource: trait for sources that can provide capabilities.
pub trait CapabilitySource: Send + Sync {
    fn source_id(&self) -> &str;
    fn source_type(&self) -> &str;
    fn generate_entries(&self) -> Vec<CapabilityEntry>;
    /// Optional change feed receiver: returns a broadcast::Receiver that fires
    /// when the source content changes. May return None for static sources.
    fn subscribe_to_changes(&self) -> Option<broadcast::Receiver<()>> {
        None
    }
}

pub struct UniversalCapabilityRegistry {
    sources: RwLock<Vec<Box<dyn CapabilitySource>>>,
    manifest: RwLock<CapabilityManifest>,
    update_tx: broadcast::Sender<()>,
    generation: AtomicU64,
}

impl UniversalCapabilityRegistry {
    pub fn new() -> Arc<Self> {
        let (tx, _rx) = broadcast::channel(16);
        Arc::new(Self {
            sources: RwLock::new(Vec::new()),
            manifest: RwLock::new(CapabilityManifest::new_empty()),
            update_tx: tx,
            generation: AtomicU64::new(0),
        })
    }

    /// Register a capability source and trigger a regeneration.
    pub async fn register(&self, source: Box<dyn CapabilitySource>) {
        self.sources.write().await.push(source);
        self.regenerate().await;
    }

    /// Regenerate the manifest by querying all sources.
    pub async fn regenerate(&self) {
        let mut manifest = CapabilityManifest::new_empty();
        let sources = self.sources.read().await;
        for s in sources.iter() {
            for entry in s.generate_entries() {
                manifest
                    .capabilities
                    .entry(entry.category.clone())
                    .or_default()
                    .push(entry);
            }
        }
        manifest.generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        // compute checksum
        match serde_json::to_vec(&manifest) {
            Ok(b) => manifest.checksum = blake3::hash(&b).to_hex().to_string(),
            Err(_) => manifest.checksum = String::new(),
        }
        manifest.summary = self.generate_summary(&manifest);
        // swap in
        *self.manifest.write().await = manifest;
        // notify listeners
        let _ = self.update_tx.send(());
    }

    pub async fn get_manifest(&self) -> CapabilityManifest {
        self.manifest.read().await.clone()
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.update_tx.subscribe()
    }

    fn generate_summary(&self, manifest: &CapabilityManifest) -> String {
        // Minimal summary: count categories and total capabilities
        let mut total = 0usize;
        for v in manifest.capabilities.values() {
            total += v.len();
        }
        format!(
            "Bonsai Capability Manifest v{}: {} categories, {} capabilities",
            manifest.manifest_version,
            manifest.capabilities.len(),
            total
        )
    }
}
