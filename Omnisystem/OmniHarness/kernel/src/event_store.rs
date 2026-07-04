use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use tracing::{error, info};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemEvent {
    pub id:            String,
    pub timestamp_utc: i64,
    pub module_source: String,
    pub event_type:    String,
    pub payload_json:  String,
    pub previous_hash: String,
    pub current_hash:  String,
    pub session_id:    String,
}

impl SystemEvent {
    pub fn compute_hash(
        ts: i64, module: &str, etype: &str, payload: &str, prev: &str,
    ) -> String {
        let mut h = Sha256::new();
        h.update(ts.to_string().as_bytes());
        h.update(b"|");
        h.update(module.as_bytes());
        h.update(b"|");
        h.update(etype.as_bytes());
        h.update(b"|");
        h.update(payload.as_bytes());
        h.update(b"|");
        h.update(prev.as_bytes());
        format!("{:x}", h.finalize())
    }
}

// ── Genesis hash (SHA-256 of empty string) ───────────────────────────────────
pub const GENESIS_HASH: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

// ── Store ────────────────────────────────────────────────────────────────────

pub struct PersistentEventStore {
    path:        PathBuf,
    tip:         Arc<RwLock<String>>,
    depth:       Arc<RwLock<u64>>,
    req_count:   Arc<RwLock<u64>>,
}

impl PersistentEventStore {
    pub async fn new(path: &str) -> Result<Self> {
        let p = PathBuf::from(path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).await?;
        }

        let (tip, depth) = if p.exists() {
            info!("[EventStore] Recovering from {}…", path);
            let (t, d) = Self::recover_verify(&p).await?;
            info!("[EventStore] Recovered. Tip: {}  Depth: {}", t, d);
            (t, d)
        } else {
            (GENESIS_HASH.to_string(), 0u64)
        };

        Ok(Self {
            path:      p,
            tip:       Arc::new(RwLock::new(tip)),
            depth:     Arc::new(RwLock::new(depth)),
            req_count: Arc::new(RwLock::new(0)),
        })
    }

    // ── Append ───────────────────────────────────────────────────

    pub async fn append_event(
        &self,
        module: &str, etype: &str, payload: &str, session_id: &str,
    ) -> Result<SystemEvent> {
        let mut tip = self.tip.write().await;
        let mut depth = self.depth.write().await;

        let ts   = Utc::now().timestamp();
        let hash = SystemEvent::compute_hash(ts, module, etype, payload, &tip);
        let id   = uuid::Uuid::new_v4().to_string();

        let ev = SystemEvent {
            id:            id.clone(),
            timestamp_utc: ts,
            module_source: module.to_string(),
            event_type:    etype.to_string(),
            payload_json:  payload.to_string(),
            previous_hash: tip.clone(),
            current_hash:  hash.clone(),
            session_id:    session_id.to_string(),
        };

        self.write_line(&ev).await?;
        *tip   = hash;
        *depth += 1;

        let mut rc = self.req_count.write().await;
        *rc += 1;

        Ok(ev)
    }

    // ── External event from mesh ─────────────────────────────────

    pub async fn append_external_event(&self, ev: SystemEvent) -> Result<()> {
        let mut tip = self.tip.write().await;

        if ev.previous_hash != *tip {
            return Err(anyhow!(
                "Fork detected: expected tip {} but event has prev {}",
                *tip, ev.previous_hash
            ));
        }

        let expected = SystemEvent::compute_hash(
            ev.timestamp_utc, &ev.module_source,
            &ev.event_type, &ev.payload_json, &ev.previous_hash,
        );
        if expected != ev.current_hash {
            return Err(anyhow!("Signature mismatch for external event {}", ev.id));
        }

        self.write_line(&ev).await?;
        let mut depth = self.depth.write().await;
        *tip   = ev.current_hash.clone();
        *depth += 1;
        Ok(())
    }

    // ── Verify full chain ─────────────────────────────────────────

    pub async fn verify_chain(&self) -> Result<bool> {
        if !self.path.exists() { return Ok(true); }
        let (_, _) = Self::recover_verify(&self.path).await?;
        Ok(true)
    }

    // ── Query ─────────────────────────────────────────────────────

    pub async fn query_events(
        &self,
        module: &str, etype: &str, since_ts: i64, limit: usize,
    ) -> Result<Vec<SystemEvent>> {
        if !self.path.exists() { return Ok(vec![]); }
        let file   = File::open(&self.path).await
            .map_err(|e| anyhow!("Open failed: {}", e))?;
        let mut lines  = BufReader::new(file).lines();
        let mut result = Vec::new();
        let lim = if limit == 0 { 1000 } else { limit };

        while let Some(line) = lines.next_line().await? {
            if result.len() >= lim { break; }
            if let Ok(ev) = serde_json::from_str::<SystemEvent>(&line) {
                if ev.timestamp_utc < since_ts { continue; }
                if !module.is_empty() && ev.module_source != module { continue; }
                if !etype.is_empty() && ev.event_type != etype { continue; }
                result.push(ev);
            }
        }
        Ok(result)
    }

    pub async fn current_tip(&self) -> String {
        self.tip.read().await.clone()
    }

    pub async fn chain_depth(&self) -> u64 {
        *self.depth.read().await
    }

    pub async fn request_count(&self) -> u64 {
        *self.req_count.read().await
    }

    // ── Internal helpers ──────────────────────────────────────────

    async fn write_line(&self, ev: &SystemEvent) -> Result<()> {
        let line = serde_json::to_string(ev)? + "\n";
        let mut f = OpenOptions::new()
            .create(true).append(true).open(&self.path).await?;
        f.write_all(line.as_bytes()).await?;
        f.sync_data().await?;
        Ok(())
    }

    async fn recover_verify(path: &PathBuf) -> Result<(String, u64)> {
        let file  = File::open(path).await?;
        let mut lines = BufReader::new(file).lines();
        let mut prev  = GENESIS_HASH.to_string();
        let mut depth = 0u64;

        while let Some(line) = lines.next_line().await? {
            let ev: SystemEvent = serde_json::from_str(&line)
                .map_err(|e| anyhow!("Deserialize error: {}", e))?;

            if ev.previous_hash != prev {
                return Err(anyhow!(
                    "Chain broken at event {} — expected prev {} got {}",
                    ev.id, prev, ev.previous_hash
                ));
            }
            let expected = SystemEvent::compute_hash(
                ev.timestamp_utc, &ev.module_source,
                &ev.event_type, &ev.payload_json, &ev.previous_hash,
            );
            if expected != ev.current_hash {
                return Err(anyhow!("Hash mismatch at event {}", ev.id));
            }
            prev   = ev.current_hash;
            depth += 1;
        }
        Ok((prev, depth))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_append_and_verify() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.jsonl").to_string_lossy().to_string();
        let store = PersistentEventStore::new(&path).await.unwrap();
        let e1 = store.append_event("test", "A", "{}", "s1").await.unwrap();
        let e2 = store.append_event("test", "B", "{}", "s1").await.unwrap();
        assert_eq!(e2.previous_hash, e1.current_hash);
        assert!(store.verify_chain().await.unwrap());
        assert_eq!(store.chain_depth().await, 2);
    }

    #[tokio::test]
    async fn test_recovery() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rec.jsonl").to_string_lossy().to_string();
        {
            let s = PersistentEventStore::new(&path).await.unwrap();
            s.append_event("t", "A", "{}", "").await.unwrap();
            s.append_event("t", "B", "{}", "").await.unwrap();
        }
        {
            let s = PersistentEventStore::new(&path).await.unwrap();
            assert_eq!(s.chain_depth().await, 2);
            assert!(s.verify_chain().await.unwrap());
            s.append_event("t", "C", "{}", "").await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_tamper_detected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tamper.jsonl");
        let path_str = path.to_string_lossy().to_string();
        {
            let s = PersistentEventStore::new(&path_str).await.unwrap();
            s.append_event("t", "A", "{}", "").await.unwrap();
        }
        // tamper file
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        let modified = content.replace("\"A\"", "\"TAMPERED\"");
        tokio::fs::write(&path, modified).await.unwrap();
        let result = PersistentEventStore::new(&path_str).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_filter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("q.jsonl").to_string_lossy().to_string();
        let s = PersistentEventStore::new(&path).await.unwrap();
        s.append_event("mod_a", "Click", "{}", "").await.unwrap();
        s.append_event("mod_b", "Click", "{}", "").await.unwrap();
        s.append_event("mod_a", "Hover", "{}", "").await.unwrap();
        let results = s.query_events("mod_a", "", 0, 100).await.unwrap();
        assert_eq!(results.len(), 2);
    }
}
