use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id:         String,
    pub collection: String,
    pub content:    String,
    pub metadata:   HashMap<String, String>,
    pub embedding:  Vec<f32>,
    pub created_at: i64,
}

// ── Store ────────────────────────────────────────────────────────────────────

pub struct VectorStore {
    /// collection → entries
    data: DashMap<String, Vec<VectorEntry>>,
    path: PathBuf,
}

impl VectorStore {
    pub async fn new(path: &str) -> Result<Self> {
        let store = Self {
            data: DashMap::new(),
            path: PathBuf::from(path),
        };
        store.load_from_disk().await?;
        Ok(store)
    }

    // ── Store ─────────────────────────────────────────────────────

    pub async fn store(
        &self,
        collection: &str,
        content: &str,
        metadata: HashMap<String, String>,
        embedding: Option<Vec<f32>>,
    ) -> Result<String> {
        let id  = uuid::Uuid::new_v4().to_string();
        let emb = embedding.unwrap_or_else(|| hash_embed(content));

        let entry = VectorEntry {
            id:         id.clone(),
            collection: collection.to_string(),
            content:    content.to_string(),
            metadata,
            embedding:  emb,
            created_at: chrono::Utc::now().timestamp(),
        };

        self.data
            .entry(collection.to_string())
            .or_insert_with(Vec::new)
            .push(entry.clone());

        self.append_to_disk(&entry).await?;
        Ok(id)
    }

    // ── Retrieve by ID ────────────────────────────────────────────

    pub fn retrieve(&self, collection: &str, id: &str) -> Option<VectorEntry> {
        self.data.get(collection)?.iter().find(|e| e.id == id).cloned()
    }

    // ── Semantic search (cosine similarity) ───────────────────────

    pub fn search_semantic(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        threshold: f32,
    ) -> Vec<(VectorEntry, f32)> {
        let query_emb = hash_embed(query_text);
        let entries   = match self.data.get(collection) {
            Some(e) => e.clone(),
            None    => return Vec::new(),
        };

        let mut scored: Vec<(VectorEntry, f32)> = entries
            .into_iter()
            .filter_map(|e| {
                let score = cosine_similarity(&query_emb, &e.embedding);
                if score >= threshold { Some((e, score)) } else { None }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    // ── Delete ────────────────────────────────────────────────────

    pub fn delete(&self, collection: &str, id: &str) -> bool {
        if let Some(mut entries) = self.data.get_mut(collection) {
            let before = entries.len();
            entries.retain(|e| e.id != id);
            return entries.len() < before;
        }
        false
    }

    // ── List collections ──────────────────────────────────────────

    pub fn list_collections(&self) -> Vec<String> {
        self.data.iter().map(|e| e.key().clone()).collect()
    }

    pub fn collection_size(&self, collection: &str) -> usize {
        self.data.get(collection).map(|e| e.len()).unwrap_or(0)
    }

    // ── Persistence ───────────────────────────────────────────────

    async fn load_from_disk(&self) -> Result<()> {
        if !self.path.exists() { return Ok(()); }
        let file  = File::open(&self.path).await?;
        let mut lines = BufReader::new(file).lines();
        while let Some(line) = lines.next_line().await? {
            if let Ok(entry) = serde_json::from_str::<VectorEntry>(&line) {
                self.data
                    .entry(entry.collection.clone())
                    .or_insert_with(Vec::new)
                    .push(entry);
            }
        }
        Ok(())
    }

    async fn append_to_disk(&self, entry: &VectorEntry) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let line = serde_json::to_string(entry)? + "\n";
        let mut f = OpenOptions::new()
            .create(true).append(true).open(&self.path).await?;
        f.write_all(line.as_bytes()).await?;
        f.sync_data().await?;
        Ok(())
    }
}

// ── Math helpers ──────────────────────────────────────────────────────────────

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 { return 0.0; }
    let dot:  f32 = a[..len].iter().zip(&b[..len]).map(|(x, y)| x * y).sum();
    let na:   f32 = a[..len].iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb:   f32 = b[..len].iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

/// Deterministic hash-based embedding: 128-dim float vector from text.
/// Used for offline/local similarity when no model embedding is available.
fn hash_embed(text: &str) -> Vec<f32> {
    let bytes  = text.as_bytes();
    let mut emb = vec![0.0f32; 128];
    for (i, &b) in bytes.iter().enumerate() {
        let slot  = (fnv1a_step(i as u64, b) % 128) as usize;
        let val   = (b as f32 - 128.0) / 128.0;
        emb[slot] += val;
    }
    // L2 normalize
    let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut emb { *x /= norm; }
    }
    emb
}

fn fnv1a_step(state: u64, byte: u8) -> u64 {
    const PRIME: u64 = 1_099_511_628_211;
    const BASIS: u64 = 14_695_981_039_346_656_037;
    let s = if state == 0 { BASIS } else { state };
    (s ^ byte as u64).wrapping_mul(PRIME)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_store_search() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vec.jsonl").to_string_lossy().to_string();
        let store = VectorStore::new(&path).await.unwrap();

        store.store("default", "The quick brown fox", Default::default(), None).await.unwrap();
        store.store("default", "A fast red fox",      Default::default(), None).await.unwrap();
        store.store("default", "Machine learning AI", Default::default(), None).await.unwrap();

        let results = store.search_semantic("default", "quick fox", 2, 0.0);
        assert!(!results.is_empty());
        assert!(results[0].1 >= results[1].1);
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("v2.jsonl").to_string_lossy().to_string();
        let store = VectorStore::new(&path).await.unwrap();
        let id = store.store("col", "hello world", Default::default(), None).await.unwrap();
        assert!(store.retrieve("col", &id).is_some());
        assert!(store.delete("col", &id));
        assert!(store.retrieve("col", &id).is_none());
    }
}
