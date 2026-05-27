//! Workstream C — OmnFS: CAS-backed virtual file system
//!
//! Every write is content-addressed (Blake3), encrypted (XChaCha20-Poly1305
//! stub — real key management deferred to bonsai-auth), and versioned.
//! Snapshots are instant (copy-on-write index pointer swap).
//! Rollback replaces the live index with a snapshot.
//!
//! This is a *virtual* FS layer on top of the real OS filesystem.  It does
//! not replace kernel VFS but provides a portable, fully-auditable overlay
//! for AI-managed files.  A FUSE/WinFSP mount is a future extension.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use bonsai_cas::{CasKey, CasStore};

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Index types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inode {
    pub path: String,
    pub content_key: CasKey,
    /// Encryption key id (references bonsai-auth key ring)
    pub key_id: Option<String>,
    pub size_bytes: u64,
    pub created_at: i64,
    pub modified_at: i64,
    pub version: u32,
    pub mime: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified_at: i64,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsSnapshot {
    pub id: String,
    pub tag: String,
    pub created_at: i64,
    pub index_key: CasKey,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsStats {
    pub total_files: usize,
    pub total_bytes: u64,
    pub snapshot_count: usize,
    pub cas_stats: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum OmnFsError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("CAS error: {0}")]
    Cas(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("snapshot not found: {0}")]
    SnapshotNotFound(String),
    #[error("path is a directory: {0}")]
    IsDirectory(String),
}

pub type OmnFsResult<T> = Result<T, OmnFsError>;

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — In-memory index
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default, Serialize, Deserialize, Clone)]
struct FsIndex {
    /// path → Inode
    files: HashMap<String, Inode>,
    /// path → set of child names (for directory listings)
    dirs: HashMap<String, Vec<String>>,
}

impl FsIndex {
    fn lookup(&self, path: &str) -> Option<&Inode> {
        self.files.get(path)
    }

    fn upsert(&mut self, path: &str, inode: Inode) {
        // Register parent directory
        if let Some(parent) = std::path::Path::new(path).parent() {
            let parent_str = parent.to_string_lossy().replace('\\', "/");
            if !parent_str.is_empty() {
                let name = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let children = self.dirs.entry(parent_str).or_default();
                if !children.contains(&name) { children.push(name); }
            }
        }
        self.files.insert(path.to_string(), inode);
    }

    fn remove(&mut self, path: &str) -> bool {
        if self.files.remove(path).is_some() {
            if let Some(parent) = std::path::Path::new(path).parent() {
                let parent_str = parent.to_string_lossy().replace('\\', "/");
                let name = std::path::Path::new(path)
                    .file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if let Some(children) = self.dirs.get_mut(&parent_str) {
                    children.retain(|c| c != &name);
                }
            }
            true
        } else {
            false
        }
    }

    fn list_dir(&self, dir: &str) -> Vec<DirEntry> {
        let dir = dir.trim_end_matches('/');
        let children = match self.dirs.get(dir) {
            Some(c) => c.clone(),
            None => return vec![],
        };
        let mut entries = Vec::new();
        for name in &children {
            let child_path = if dir.is_empty() { name.clone() } else { format!("{dir}/{name}") };
            if let Some(inode) = self.files.get(&child_path) {
                entries.push(DirEntry {
                    name: name.clone(),
                    path: child_path,
                    is_dir: false,
                    size_bytes: inode.size_bytes,
                    modified_at: inode.modified_at,
                    version: inode.version,
                });
            } else {
                // Sub-directory
                entries.push(DirEntry {
                    name: name.clone(),
                    path: child_path,
                    is_dir: true,
                    size_bytes: 0,
                    modified_at: 0,
                    version: 0,
                });
            }
        }
        entries
    }

    fn total_size(&self) -> u64 {
        self.files.values().map(|i| i.size_bytes).sum()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3 — Encryption stub
// ─────────────────────────────────────────────────────────────────────────────
// A real implementation would use the bonsai-auth key ring (XChaCha20-Poly1305).
// This stub passes data through unchanged so the rest of the system compiles.

struct CryptoStub;

impl CryptoStub {
    fn encrypt(&self, data: &[u8], _key_id: Option<&str>) -> Vec<u8> {
        data.to_vec()
    }
    fn decrypt(&self, data: &[u8], _key_id: Option<&str>) -> Vec<u8> {
        data.to_vec()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4 — OmnFS
// ─────────────────────────────────────────────────────────────────────────────

pub struct OmnFS {
    cas: Arc<CasStore>,
    index: RwLock<FsIndex>,
    snapshots: RwLock<Vec<FsSnapshot>>,
    crypto: CryptoStub,
}

impl OmnFS {
    pub fn new(cas: Arc<CasStore>) -> Arc<Self> {
        Arc::new(Self {
            cas,
            index: RwLock::new(FsIndex::default()),
            snapshots: RwLock::new(Vec::new()),
            crypto: CryptoStub,
        })
    }

    // ── Core I/O ─────────────────────────────────────────────────────────────

    /// Read a file by path.  Returns its decrypted bytes.
    pub async fn read(&self, path: &str) -> OmnFsResult<Vec<u8>> {
        let key_id;
        let content_key;
        {
            let idx = self.index.read().await;
            let inode = idx.lookup(path).ok_or_else(|| OmnFsError::NotFound(path.into()))?;
            content_key = inode.content_key.clone();
            key_id = inode.key_id.clone();
        }
        let encrypted = self.cas.get(&content_key).await
            .map_err(|e| OmnFsError::Cas(e.to_string()))?
            .ok_or_else(|| OmnFsError::NotFound(path.into()))?;
        Ok(self.crypto.decrypt(&encrypted, key_id.as_deref()))
    }

    /// Write a file.  Transparently encrypts and stores in CAS; creates a new
    /// version entry.  Returns the CAS key for the content.
    pub async fn write(&self, path: &str, data: &[u8], mime: Option<&str>) -> OmnFsResult<CasKey> {
        let mime = mime.unwrap_or("application/octet-stream");
        let encrypted = self.crypto.encrypt(data, None);
        let key = self.cas.put(&encrypted, mime).await
            .map_err(|e| OmnFsError::Cas(e.to_string()))?;

        let now = Utc::now().timestamp_micros();
        let mut idx = self.index.write().await;
        let (version, created_at) = match idx.lookup(path) {
            Some(existing) => (existing.version + 1, existing.created_at),
            None => (1, now),
        };
        idx.upsert(path, Inode {
            path: path.to_string(),
            content_key: key.clone(),
            key_id: None,
            size_bytes: data.len() as u64,
            created_at,
            modified_at: now,
            version,
            mime: mime.to_string(),
            tags: vec![],
        });
        debug!("[omnfs] wrote {path} (v{version}, {} bytes)", data.len());
        Ok(key)
    }

    /// Delete a file
    pub async fn delete(&self, path: &str) -> OmnFsResult<()> {
        if self.index.write().await.remove(path) {
            info!("[omnfs] deleted {path}");
            Ok(())
        } else {
            Err(OmnFsError::NotFound(path.into()))
        }
    }

    /// Check if a path exists
    pub async fn exists(&self, path: &str) -> bool {
        self.index.read().await.lookup(path).is_some()
    }

    /// Get inode metadata
    pub async fn stat(&self, path: &str) -> OmnFsResult<Inode> {
        self.index.read().await.lookup(path)
            .cloned()
            .ok_or_else(|| OmnFsError::NotFound(path.into()))
    }

    /// List a directory
    pub async fn list_dir(&self, dir: &str) -> Vec<DirEntry> {
        self.index.read().await.list_dir(dir)
    }

    /// Copy a file within OmnFS
    pub async fn copy(&self, src: &str, dst: &str) -> OmnFsResult<()> {
        let data = self.read(src).await?;
        let inode = self.stat(src).await?;
        self.write(dst, &data, Some(&inode.mime)).await?;
        Ok(())
    }

    // ── Snapshots ────────────────────────────────────────────────────────────

    /// Create a named snapshot of the current index state.
    pub async fn snapshot(&self, tag: &str) -> OmnFsResult<FsSnapshot> {
        let idx = self.index.read().await;
        let bytes = serde_json::to_vec(&*idx)?;
        let key = self.cas.put(&bytes, "application/json").await
            .map_err(|e| OmnFsError::Cas(e.to_string()))?;
        let snap = FsSnapshot {
            id: Uuid::new_v4().to_string(),
            tag: tag.to_string(),
            created_at: Utc::now().timestamp_micros(),
            index_key: key,
            entry_count: idx.files.len(),
        };
        self.snapshots.write().await.push(snap.clone());
        info!("[omnfs] snapshot '{tag}' ({} files)", snap.entry_count);
        Ok(snap)
    }

    /// Rollback to a named snapshot.  All subsequent writes are lost.
    pub async fn rollback(&self, tag: &str) -> OmnFsResult<()> {
        let key = {
            let snaps = self.snapshots.read().await;
            snaps.iter().rev()
                .find(|s| s.tag == tag)
                .map(|s| s.index_key.clone())
                .ok_or_else(|| OmnFsError::SnapshotNotFound(tag.into()))?
        };
        let bytes = self.cas.get(&key).await
            .map_err(|e| OmnFsError::Cas(e.to_string()))?
            .ok_or_else(|| OmnFsError::SnapshotNotFound(tag.into()))?;
        let restored: FsIndex = serde_json::from_slice(&bytes)?;
        *self.index.write().await = restored;
        info!("[omnfs] rolled back to snapshot '{tag}'");
        Ok(())
    }

    /// List all snapshots
    pub async fn list_snapshots(&self) -> Vec<FsSnapshot> {
        self.snapshots.read().await.clone()
    }

    // ── Stats ────────────────────────────────────────────────────────────────

    pub async fn stats(&self) -> FsStats {
        let idx = self.index.read().await;
        let snap_count = self.snapshots.read().await.len();
        let cas_stats = self.cas.stats().await
            .map(|s| serde_json::to_value(s).unwrap_or_default())
            .unwrap_or_default();
        FsStats {
            total_files: idx.files.len(),
            total_bytes: idx.total_size(),
            snapshot_count: snap_count,
            cas_stats,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct WriteRequest {
    pub path: String,
    pub data_base64: String,
    pub mime: Option<String>,
}

#[tauri::command]
pub async fn omnfs_read(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<String, String> {
    let bytes = state.omnfs.read(&path).await.map_err(|e| e.to_string())?;
    Ok(base64_encode(&bytes))
}

#[tauri::command]
pub async fn omnfs_write(
    state: tauri::State<'_, crate::AppState>,
    req: WriteRequest,
) -> Result<String, String> {
    let data = base64_decode(&req.data_base64).map_err(|e| e.to_string())?;
    let key = state.omnfs.write(&req.path, &data, req.mime.as_deref()).await
        .map_err(|e| e.to_string())?;
    Ok(key.hex())
}

#[tauri::command]
pub async fn omnfs_delete(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<(), String> {
    state.omnfs.delete(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn omnfs_stat(
    state: tauri::State<'_, crate::AppState>,
    path: String,
) -> Result<Inode, String> {
    state.omnfs.stat(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn omnfs_list_dir(
    state: tauri::State<'_, crate::AppState>,
    dir: String,
) -> Result<Vec<DirEntry>, String> {
    Ok(state.omnfs.list_dir(&dir).await)
}

#[tauri::command]
pub async fn omnfs_snapshot(
    state: tauri::State<'_, crate::AppState>,
    tag: String,
) -> Result<FsSnapshot, String> {
    state.omnfs.snapshot(&tag).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn omnfs_rollback(
    state: tauri::State<'_, crate::AppState>,
    tag: String,
) -> Result<(), String> {
    state.omnfs.rollback(&tag).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn omnfs_list_snapshots(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<FsSnapshot>, String> {
    Ok(state.omnfs.list_snapshots().await)
}

#[tauri::command]
pub async fn omnfs_stats(
    state: tauri::State<'_, crate::AppState>,
) -> Result<FsStats, String> {
    Ok(state.omnfs.stats().await)
}

// ─────────────────────────────────────────────────────────────────────────────
// § 6 — Base64 helpers (avoid heavy dep)
// ─────────────────────────────────────────────────────────────────────────────

fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() * 4 / 3) + 4);
    let mut i = 0;
    while i + 2 < data.len() {
        let b = ((data[i] as u32) << 16) | ((data[i+1] as u32) << 8) | (data[i+2] as u32);
        out.push(CHARS[((b >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((b >> 12) & 0x3f) as usize] as char);
        out.push(CHARS[((b >> 6)  & 0x3f) as usize] as char);
        out.push(CHARS[( b        & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let b = (data[i] as u32) << 16;
        out.push(CHARS[((b >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((b >> 12) & 0x3f) as usize] as char);
        out.push_str("==");
    } else if rem == 2 {
        let b = ((data[i] as u32) << 16) | ((data[i+1] as u32) << 8);
        out.push(CHARS[((b >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((b >> 12) & 0x3f) as usize] as char);
        out.push(CHARS[((b >> 6)  & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62), b'/' => Ok(63), b'=' => Ok(0),
            _ => Err(format!("invalid base64 char: {c}")),
        }
    }
    let s = s.trim().as_bytes();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < s.len() {
        let (a, b, c, d) = (val(s[i])?, val(s[i+1])?, val(s[i+2])?, val(s[i+3])?);
        let n = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | (d as u32);
        out.push((n >> 16) as u8);
        if s[i+2] != b'=' { out.push((n >> 8) as u8); }
        if s[i+3] != b'=' { out.push(n as u8); }
        i += 4;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        let original = b"Hello, OmnFS!";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn fs_index_basic() {
        let mut idx = FsIndex::default();
        let inode = Inode {
            path: "docs/readme.md".into(),
            content_key: CasKey::from_bytes(b"test"),
            key_id: None, size_bytes: 42, created_at: 0, modified_at: 0,
            version: 1, mime: "text/markdown".into(), tags: vec![],
        };
        idx.upsert("docs/readme.md", inode);
        assert!(idx.lookup("docs/readme.md").is_some());
        let entries = idx.list_dir("docs");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "readme.md");
    }

    #[test]
    fn fs_index_remove() {
        let mut idx = FsIndex::default();
        let inode = Inode {
            path: "a/b.txt".into(), content_key: CasKey::from_bytes(b"x"),
            key_id: None, size_bytes: 1, created_at: 0, modified_at: 0,
            version: 1, mime: "text/plain".into(), tags: vec![],
        };
        idx.upsert("a/b.txt", inode);
        assert!(idx.remove("a/b.txt"));
        assert!(idx.lookup("a/b.txt").is_none());
        assert!(!idx.remove("a/b.txt")); // idempotent
    }
}
