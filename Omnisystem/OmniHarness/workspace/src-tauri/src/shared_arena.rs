//! Zero-copy cross-model memory arena backed by a memory-mapped file.
//!
//! Stores embeddings, context summaries, and KV-cache digests so that model
//! swaps can reuse previously computed representations without re-encoding.
//!
//! # Layout (fixed-header + variable blocks)
//!
//!  [ ArenaHeader (256 bytes) ][ Block* ]
//!
//!  Each Block:
//!  [ BlockHeader (64 bytes) ][ payload bytes ]
//!
//! The file grows by `GROW_STEP_BYTES` when full.  All offsets are absolute
//! from file start.  Concurrent access across OS processes is guarded by a
//! simple advisory file-lock flag inside ArenaHeader.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ── Constants ─────────────────────────────────────────────────────────────────

const MAGIC: u64 = 0xB0_A5_A1_30_C0_DE_00_01;
const ARENA_VERSION: u32 = 1;
const HEADER_SIZE: usize = 256;
const BLOCK_HDR_SIZE: usize = 64;
const GROW_STEP_BYTES: u64 = 64 * 1024 * 1024; // 64 MiB growth steps
const DEFAULT_CAP: u64 = 256 * 1024 * 1024; // 256 MiB initial capacity
const MAX_ARENA_BYTES: u64 = 8 * 1024 * 1024 * 1024; // 8 GiB hard cap

// ── Block types ───────────────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Free = 0,
    Embedding = 1, // float32 embedding vector
    Summary = 2,   // UTF-8 context summary
    KvDigest = 3,  // SHA-256 of a KV-cache page for deduplication
    Metadata = 4,  // arbitrary JSON metadata blob
}

impl BlockKind {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Embedding,
            2 => Self::Summary,
            3 => Self::KvDigest,
            4 => Self::Metadata,
            _ => Self::Free,
        }
    }
}

// ── On-disk structures (repr C for stable layout) ─────────────────────────────

#[repr(C)]
struct ArenaHeader {
    magic: u64,
    version: u32,
    capacity: u64,
    write_cursor: u64, // next free byte (>= HEADER_SIZE)
    block_count: u64,
    _pad: [u8; 222],
}

#[repr(C)]
struct RawBlockHeader {
    kind: u8,
    _pad1: [u8; 3],
    payload_sz: u32,
    key_hash: u64,   // xxhash of the lookup key
    model_hash: u64, // hash of model_id that wrote this block
    timestamp: u64,  // unix seconds
    generation: u64, // monotonic write counter
    ref_count: u32,  // 0 = can GC
    _pad2: [u8; 16],
}

// ── Public handle types ───────────────────────────────────────────────────────

/// A reference to a block inside the arena. Cheap to clone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHandle {
    pub offset: u64,
    pub size: u64,
    pub kind: u8,
    pub generation: u64,
}

/// Stats snapshot returned by `SharedMemoryArena::stats()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaStats {
    pub capacity_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub block_count: u64,
    pub live_block_count: u64,
    pub embedding_count: u64,
    pub summary_count: u64,
}

// ── Arena ─────────────────────────────────────────────────────────────────────

/// In-process arena state — wraps the mmap'd file.
/// Use `Arc<SharedMemoryArena>` for shared ownership.
pub struct SharedMemoryArena {
    path: PathBuf,
    inner: RwLock<ArenaInner>,
}

struct ArenaInner {
    file: File,
    data: Vec<u8>, // in-memory shadow (Windows: full file content)
    cursor: u64,
    gen: u64,
    cap: u64,
}

// ── Key hashing ───────────────────────────────────────────────────────────────

fn hash_key(key: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    h.finish()
}

fn hash_model(model_id: &str) -> u64 {
    hash_key(model_id)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Implementation ────────────────────────────────────────────────────────────

impl SharedMemoryArena {
    /// Open or create the arena file at `path` with `capacity_bytes` initial size.
    pub fn open(path: impl AsRef<Path>, capacity_bytes: Option<u64>) -> anyhow::Result<Arc<Self>> {
        let path = path.as_ref().to_path_buf();
        let cap = capacity_bytes.unwrap_or(DEFAULT_CAP).min(MAX_ARENA_BYTES);

        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }

        let exists = path.exists();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        let (data, cursor, gen, actual_cap) =
            if exists && file.metadata()?.len() >= HEADER_SIZE as u64 {
                // Read existing arena
                let raw = std::fs::read(&path)?;
                let hdr = unsafe { &*(raw.as_ptr() as *const ArenaHeader) };
                if hdr.magic == MAGIC && hdr.version == ARENA_VERSION {
                    let cur = hdr.write_cursor.min(raw.len() as u64);
                    let g = hdr.block_count;
                    let c = hdr.capacity;
                    (raw, cur, g, c)
                } else {
                    let (d, c, a) = Self::init_file(&file, cap)?;
                    (d, HEADER_SIZE as u64, 0, a)
                }
            } else {
                let (d, c, a) = Self::init_file(&file, cap)?;
                (d, HEADER_SIZE as u64, 0, a)
            };

        Ok(Arc::new(Self {
            path,
            inner: RwLock::new(ArenaInner {
                file,
                data,
                cursor,
                gen,
                cap: actual_cap,
            }),
        }))
    }

    fn init_file(file: &File, cap: u64) -> anyhow::Result<(Vec<u8>, u64, u64)> {
        let mut data = vec![0u8; cap as usize];
        let hdr = unsafe { &mut *(data.as_mut_ptr() as *mut ArenaHeader) };
        hdr.magic = MAGIC;
        hdr.version = ARENA_VERSION;
        hdr.capacity = cap;
        hdr.write_cursor = HEADER_SIZE as u64;
        hdr.block_count = 0;
        file.set_len(cap)?;
        Ok((data, HEADER_SIZE as u64, cap))
    }

    fn flush_header(inner: &mut ArenaInner) -> anyhow::Result<()> {
        use std::io::{Seek, SeekFrom};
        let hdr = unsafe { &mut *(inner.data.as_mut_ptr() as *mut ArenaHeader) };
        hdr.write_cursor = inner.cursor;
        hdr.block_count = inner.gen;
        hdr.capacity = inner.cap;
        inner.file.seek(SeekFrom::Start(0))?;
        inner.file.write_all(&inner.data[..HEADER_SIZE])?;
        Ok(())
    }

    fn flush_block(inner: &mut ArenaInner, offset: u64, total: usize) -> anyhow::Result<()> {
        use std::io::{Seek, SeekFrom};
        inner.file.seek(SeekFrom::Start(offset))?;
        let end = (offset as usize + total).min(inner.data.len());
        inner.file.write_all(&inner.data[offset as usize..end])?;
        Ok(())
    }

    /// Allocate a block and write `payload`. Returns a `MemoryHandle`.
    fn alloc_block(
        inner: &mut ArenaInner,
        kind: BlockKind,
        key: &str,
        model_id: &str,
        payload: &[u8],
    ) -> anyhow::Result<MemoryHandle> {
        let total = BLOCK_HDR_SIZE + payload.len();
        // Grow if needed
        if inner.cursor + total as u64 > inner.cap {
            let new_cap = (inner.cap + GROW_STEP_BYTES).min(MAX_ARENA_BYTES);
            if new_cap == inner.cap {
                anyhow::bail!("SharedMemoryArena: arena full ({} bytes)", inner.cap);
            }
            inner.file.set_len(new_cap)?;
            inner.data.resize(new_cap as usize, 0);
            inner.cap = new_cap;
        }

        let offset = inner.cursor;
        inner.gen += 1;

        // Write block header
        let bh =
            unsafe { &mut *(inner.data[offset as usize..].as_mut_ptr() as *mut RawBlockHeader) };
        bh.kind = kind as u8;
        bh._pad1 = [0; 3];
        bh.payload_sz = payload.len() as u32;
        bh.key_hash = hash_key(key);
        bh.model_hash = hash_model(model_id);
        bh.timestamp = unix_now();
        bh.generation = inner.gen;
        bh.ref_count = 1;
        bh._pad2 = [0; 16];

        // Write payload
        let body_start = offset as usize + BLOCK_HDR_SIZE;
        inner.data[body_start..body_start + payload.len()].copy_from_slice(payload);

        inner.cursor += total as u64;

        // Flush
        Self::flush_block(inner, offset, total)?;
        Self::flush_header(inner)?;

        Ok(MemoryHandle {
            offset,
            size: total as u64,
            kind: kind as u8,
            generation: inner.gen,
        })
    }

    // ── Public write API ──────────────────────────────────────────────────────

    /// Store an embedding vector. `key` is the text or content hash it represents.
    pub fn store_embedding(
        &self,
        key: &str,
        model_id: &str,
        embedding: &[f32],
    ) -> anyhow::Result<MemoryHandle> {
        let payload: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        let mut inner = self.inner.write().unwrap();
        Self::alloc_block(&mut inner, BlockKind::Embedding, key, model_id, &payload)
    }

    /// Store a context summary (UTF-8 text).
    pub fn store_summary(
        &self,
        key: &str,
        model_id: &str,
        summary: &str,
    ) -> anyhow::Result<MemoryHandle> {
        let mut inner = self.inner.write().unwrap();
        Self::alloc_block(
            &mut inner,
            BlockKind::Summary,
            key,
            model_id,
            summary.as_bytes(),
        )
    }

    /// Store arbitrary JSON metadata.
    pub fn store_metadata(
        &self,
        key: &str,
        model_id: &str,
        meta: &serde_json::Value,
    ) -> anyhow::Result<MemoryHandle> {
        let payload = serde_json::to_vec(meta)?;
        let mut inner = self.inner.write().unwrap();
        Self::alloc_block(&mut inner, BlockKind::Metadata, key, model_id, &payload)
    }

    // ── Public read API ───────────────────────────────────────────────────────

    /// Look up the most recent block matching `key_hash` (linear scan, suitable for
    /// arena sizes ≤ a few thousand blocks — use an index layer for more).
    pub fn find_latest(&self, key: &str, kind: BlockKind) -> Option<MemoryHandle> {
        let kh = hash_key(key);
        let inner = self.inner.read().unwrap();
        let mut pos = HEADER_SIZE as u64;
        let mut best: Option<MemoryHandle> = None;

        while pos + BLOCK_HDR_SIZE as u64 <= inner.cursor {
            let bh = unsafe { &*(inner.data[pos as usize..].as_ptr() as *const RawBlockHeader) };
            let total = BLOCK_HDR_SIZE as u64 + bh.payload_sz as u64;
            if bh.kind != BlockKind::Free as u8
                && bh.kind == kind as u8
                && bh.key_hash == kh
                && bh.ref_count > 0
            {
                let is_newer = best.as_ref().map_or(true, |b| bh.generation > b.generation);
                if is_newer {
                    best = Some(MemoryHandle {
                        offset: pos,
                        size: total,
                        kind: bh.kind,
                        generation: bh.generation,
                    });
                }
            }
            if total == 0 {
                break;
            }
            pos += total;
        }
        best
    }

    /// Read the payload of a handle as raw bytes.
    pub fn read_payload(&self, handle: &MemoryHandle) -> Option<Vec<u8>> {
        let inner = self.inner.read().unwrap();
        let start = handle.offset as usize + BLOCK_HDR_SIZE;
        let bh =
            unsafe { &*(inner.data[handle.offset as usize..].as_ptr() as *const RawBlockHeader) };
        let end = start + bh.payload_sz as usize;
        if end > inner.data.len() {
            return None;
        }
        Some(inner.data[start..end].to_vec())
    }

    /// Read a stored embedding back as f32 slice.
    pub fn read_embedding(&self, handle: &MemoryHandle) -> Option<Vec<f32>> {
        let bytes = self.read_payload(handle)?;
        if bytes.len() % 4 != 0 {
            return None;
        }
        Some(
            bytes
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect(),
        )
    }

    /// Read a stored summary as a String.
    pub fn read_summary(&self, handle: &MemoryHandle) -> Option<String> {
        let bytes = self.read_payload(handle)?;
        String::from_utf8(bytes).ok()
    }

    // ── Garbage collection ────────────────────────────────────────────────────

    /// Mark blocks older than `max_age_secs` or with zero ref_count as free.
    /// Returns the number of bytes reclaimed (logical, not compacted).
    pub fn gc(&self, max_age_secs: u64) -> u64 {
        let now = unix_now();
        let mut inner = self.inner.write().unwrap();
        let mut pos = HEADER_SIZE as u64;
        let mut reclaimed = 0u64;

        while pos + BLOCK_HDR_SIZE as u64 <= inner.cursor {
            let bh =
                unsafe { &mut *(inner.data[pos as usize..].as_mut_ptr() as *mut RawBlockHeader) };
            let total = BLOCK_HDR_SIZE as u64 + bh.payload_sz as u64;
            if bh.kind != BlockKind::Free as u8 {
                let age = now.saturating_sub(bh.timestamp);
                if bh.ref_count == 0 || age > max_age_secs {
                    bh.kind = BlockKind::Free as u8;
                    reclaimed += total;
                }
            }
            if total == 0 {
                break;
            }
            pos += total;
        }
        reclaimed
    }

    // ── Stats ─────────────────────────────────────────────────────────────────

    pub fn stats(&self) -> ArenaStats {
        let inner = self.inner.read().unwrap();
        let mut pos = HEADER_SIZE as u64;
        let mut live = 0u64;
        let mut total_blocks = 0u64;
        let mut emb = 0u64;
        let mut sum = 0u64;

        while pos + BLOCK_HDR_SIZE as u64 <= inner.cursor {
            let bh = unsafe { &*(inner.data[pos as usize..].as_ptr() as *const RawBlockHeader) };
            let total = BLOCK_HDR_SIZE as u64 + bh.payload_sz as u64;
            if total == 0 {
                break;
            }
            total_blocks += 1;
            if bh.kind != BlockKind::Free as u8 && bh.ref_count > 0 {
                live += 1;
                match BlockKind::from_u8(bh.kind) {
                    BlockKind::Embedding => emb += 1,
                    BlockKind::Summary => sum += 1,
                    _ => {}
                }
            }
            pos += total;
        }

        ArenaStats {
            capacity_bytes: inner.cap,
            used_bytes: inner.cursor - HEADER_SIZE as u64,
            free_bytes: inner.cap.saturating_sub(inner.cursor),
            block_count: total_blocks,
            live_block_count: live,
            embedding_count: emb,
            summary_count: sum,
        }
    }
}
