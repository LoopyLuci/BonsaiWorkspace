use anyhow::Result;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::model_router::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id:            String,
    pub title:         String,
    pub model_id:      String,
    pub created_at:    i64,
    pub updated_at:    i64,
    pub history:       Vec<ChatMessage>,
    pub metadata:      HashMap<String, String>,
}

pub struct SessionStore {
    sessions: DashMap<String, Session>,
    path:     PathBuf,
}

impl SessionStore {
    pub async fn new(path: &str) -> Result<Self> {
        let store = Self {
            sessions: DashMap::new(),
            path:     PathBuf::from(path),
        };
        store.load().await?;
        Ok(store)
    }

    pub async fn create(
        &self, title: &str, model_id: &str, metadata: HashMap<String, String>,
    ) -> Result<Session> {
        let now = Utc::now().timestamp();
        let s   = Session {
            id:         uuid::Uuid::new_v4().to_string(),
            title:      title.to_string(),
            model_id:   model_id.to_string(),
            created_at: now,
            updated_at: now,
            history:    Vec::new(),
            metadata,
        };
        self.sessions.insert(s.id.clone(), s.clone());
        self.persist(&s).await?;
        Ok(s)
    }

    pub fn get(&self, id: &str) -> Option<Session> {
        self.sessions.get(id).map(|s| s.clone())
    }

    pub fn list(&self, limit: usize, since: i64) -> Vec<Session> {
        let lim = if limit == 0 { 100 } else { limit };
        let mut sessions: Vec<Session> = self.sessions
            .iter()
            .filter(|s| s.created_at >= since)
            .map(|s| s.clone())
            .collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions.truncate(lim);
        sessions
    }

    pub async fn append_message(&self, id: &str, msg: ChatMessage) -> Result<bool> {
        if let Some(mut s) = self.sessions.get_mut(id) {
            s.history.push(msg);
            s.updated_at = Utc::now().timestamp();
            let snapshot = s.clone();
            drop(s);
            self.persist(&snapshot).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn delete(&self, id: &str) -> bool {
        self.sessions.remove(id).is_some()
    }

    // ── Persistence (append-log; reload rebuilds latest state) ────

    async fn persist(&self, s: &Session) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let line = serde_json::to_string(s)? + "\n";
        let mut f = OpenOptions::new().create(true).append(true).open(&self.path).await?;
        f.write_all(line.as_bytes()).await?;
        f.sync_data().await?;
        Ok(())
    }

    async fn load(&self) -> Result<()> {
        if !self.path.exists() { return Ok(()); }
        let file  = File::open(&self.path).await?;
        let mut lines = BufReader::new(file).lines();
        // Last write for each id wins (append-log replay)
        while let Some(line) = lines.next_line().await? {
            if let Ok(s) = serde_json::from_str::<Session>(&line) {
                self.sessions.insert(s.id.clone(), s);
            }
        }
        Ok(())
    }
}
