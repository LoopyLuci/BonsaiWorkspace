//! Simple capability query layer (keyword match) for Phase 0

use std::sync::Arc;

use bonsai_capability_registry::{CapabilityEntry, CapabilityManifest, UniversalCapabilityRegistry};

#[derive(Debug, Clone)]
pub struct ScoredCapability {
    pub entry: CapabilityEntry,
    pub score: f32,
}

pub struct CapabilityQuery {
    registry: Arc<UniversalCapabilityRegistry>,
}

impl CapabilityQuery {
    pub fn new(registry: Arc<UniversalCapabilityRegistry>) -> Self {
        Self { registry }
    }

    pub async fn get_summary(&self) -> String {
        let m = self.registry.get_manifest().await;
        m.summary
    }

    pub async fn get_by_category(&self, category: &str) -> Vec<CapabilityEntry> {
        let m = self.registry.get_manifest().await;
        m.capabilities.get(category).cloned().unwrap_or_default()
    }

    /// Primitive search: token presence in trigger_phrases, name, or description.
    pub async fn search(&self, query: &str, categories: Option<&[String]>, top_k: usize) -> Vec<ScoredCapability> {
        let q = query.to_lowercase();
        let m = self.registry.get_manifest().await;
        let mut out: Vec<ScoredCapability> = Vec::new();
        for (cat, list) in m.capabilities.iter() {
            if let Some(cats) = categories {
                if !cats.contains(cat) { continue; }
            }
            for entry in list.iter() {
                let mut score = 0f32;
                if entry.name.to_lowercase().contains(&q) { score += 2.0; }
                if entry.description.as_ref().map(|d| d.to_lowercase().contains(&q)).unwrap_or(false) { score += 1.0; }
                for t in entry.trigger_phrases.iter() {
                    if t.to_lowercase().contains(&q) {
                        score += 3.0;
                    }
                }
                if score > 0.0 {
                    out.push(ScoredCapability { entry: entry.clone(), score });
                }
            }
        }
        out.sort_by(|a,b| b.score.partial_cmp(&a.score).unwrap());
        out.truncate(top_k);
        out
    }
}
