//! Tauri commands — Bonsai transfer, identity, and mailbox integration.
//!
//! These commands bridge the frontend to the five transfer crates:
//!   p2p-crypto  (identity, key derivation, encryption)
//!   bonsai-transfer-store   (encrypted at-rest persistence)
//!   p2p-core    (ECF-RG chunked send, transfer status)
//!   bonsai-mailbox          (agent-to-agent signed message delivery)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use mailbox::{AgentMailbox, MailEnvelope};
use transfer_core::{
    lane::InProcessLane,
    scheduler::EcfRgScheduler,
    transfer::{
        Transfer, TransferState as CoreTransferState, TransferStatus, DEFAULT_CHUNK_SIZE,
        MAX_CHUNK_SIZE,
    },
};
use transfer_crypto::{
    identity::BonsaiIdentity,
    kdf::{generate_phrase, kdf_phrase_to_seed, ARGON2_PARAMS_TEST},
};
use transfer_store::EncryptedStore;

// ── Managed state types ───────────────────────────────────────────────────────

/// Shared identity + store state managed by Tauri.
pub struct TransferState {
    /// The user's local BonsaiIdentity (set after create/restore).
    pub identity: Mutex<Option<Arc<BonsaiIdentity>>>,
    /// Encrypted key-value store backed by a single file on disk.
    pub store: EncryptedStore,
    /// Agent mailbox — local delivery hub.
    pub mailbox: AgentMailbox,
    /// In-progress transfers keyed by transfer ID.
    pub transfers: Arc<Mutex<HashMap<String, TransferStatus>>>,
}

impl TransferState {
    pub fn new() -> Self {
        let store_path = EncryptedStore::default_path();
        Self {
            identity: Mutex::new(None),
            store: EncryptedStore::open(store_path, b"bonsai-default-store-key"),
            mailbox: AgentMailbox::new(),
            transfers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

// ── Serializable DTOs ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityDto {
    pub fingerprint: String,
    pub public_key_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStatusDto {
    pub id: String,
    pub direction: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub chunk_count: u64,
    pub chunks_done: u64,
    pub state: String,
    pub bytes_per_sec: f64,
    pub progress_pct: f32,
}

impl From<&TransferStatus> for TransferStatusDto {
    fn from(s: &TransferStatus) -> Self {
        let state_str = match &s.state {
            CoreTransferState::Pending => "pending",
            CoreTransferState::Active => "active",
            CoreTransferState::Paused => "paused",
            CoreTransferState::Complete => "complete",
            CoreTransferState::Failed(_) => "failed",
            CoreTransferState::Cancelled => "cancelled",
        }
        .to_string();
        Self {
            id: s.id.to_string(),
            direction: format!("{:?}", s.direction).to_lowercase(),
            total_bytes: s.total_bytes,
            transferred_bytes: s.transferred_bytes,
            chunk_count: s.chunk_count,
            chunks_done: s.chunks_done,
            state: state_str,
            bytes_per_sec: s.bytes_per_sec,
            progress_pct: s.progress() * 100.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageDto {
    pub id: String,
    pub from_fingerprint: String,
    pub to_fingerprint: String,
    pub topic: String,
    pub payload_text: Option<String>,
    pub created_at_ms: u64,
    pub from_me: bool,
}

impl MessageDto {
    fn from_envelope(env: &MailEnvelope, my_fingerprint: &str) -> Self {
        Self {
            id: env.id.to_string(),
            from_fingerprint: env.from.clone(),
            to_fingerprint: env.to.clone(),
            topic: env.topic.clone(),
            payload_text: String::from_utf8(env.payload.clone()).ok(),
            created_at_ms: env.created_at_ms,
            from_me: env.from == my_fingerprint,
        }
    }
}

// ── Identity commands ─────────────────────────────────────────────────────────

/// Generate a new BIP-39 recovery phrase (12 words). Does not persist anything.
#[tauri::command]
pub async fn transfer_generate_phrase() -> Result<String, String> {
    generate_phrase().map_err(|e| e.to_string())
}

/// Create a new identity from a BIP-39 phrase + passphrase. Persists to the
/// encrypted store using the passphrase as the AES key.
#[tauri::command]
pub async fn transfer_create_identity(
    phrase: String,
    passphrase: String,
    ts: State<'_, TransferState>,
) -> Result<IdentityDto, String> {
    // Use test-speed Argon2 params so the UI doesn't block for 3+ seconds.
    // In production builds swap None → Some(ARGON2_PARAMS_TEST) only in tests.
    let passphrase_opt = if passphrase.is_empty() {
        None
    } else {
        Some(passphrase.as_str())
    };
    let seed = kdf_phrase_to_seed(&phrase, passphrase_opt, Some(ARGON2_PARAMS_TEST))
        .map_err(|e| e.to_string())?;

    let identity = BonsaiIdentity::from_seed(&seed).map_err(|e| e.to_string())?;
    let dto = IdentityDto {
        fingerprint: identity.fingerprint().to_string(),
        public_key_hex: hex::encode(identity.public_key.to_hex()),
    };

    // Persist seed in the store, keyed by a BLAKE3 of the passphrase.
    let store_passphrase = blake3::hash(passphrase.as_bytes());
    let store = EncryptedStore::open(EncryptedStore::default_path(), store_passphrase.as_bytes());
    let payload = serde_json::json!({
        "seed": hex::encode(seed),
        "fingerprint": identity.fingerprint(),
    });
    store.save(&payload).map_err(|e| e.to_string())?;

    *ts.identity.lock().await = Some(Arc::new(identity));
    Ok(dto)
}

/// Unlock identity from the encrypted store using the passphrase.
#[tauri::command]
pub async fn transfer_unlock_identity(
    passphrase: String,
    ts: State<'_, TransferState>,
) -> Result<IdentityDto, String> {
    let store_passphrase = blake3::hash(passphrase.as_bytes());
    let store = EncryptedStore::open(EncryptedStore::default_path(), store_passphrase.as_bytes());
    let payload: serde_json::Value = store.load().map_err(|e| e.to_string())?;

    let seed_hex = payload["seed"].as_str().ok_or("missing seed")?;
    let seed_bytes = hex::decode(seed_hex).map_err(|e| e.to_string())?;
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_bytes);

    let identity = BonsaiIdentity::from_seed(&seed).map_err(|e| e.to_string())?;
    let dto = IdentityDto {
        fingerprint: identity.fingerprint().to_string(),
        public_key_hex: hex::encode(identity.public_key.to_hex()),
    };
    *ts.identity.lock().await = Some(Arc::new(identity));
    Ok(dto)
}

/// Returns the current identity if one is loaded, or null.
#[tauri::command]
pub async fn transfer_get_identity(
    ts: State<'_, TransferState>,
) -> Result<Option<IdentityDto>, String> {
    match ts.identity.lock().await.as_ref() {
        Some(id) => Ok(Some(IdentityDto {
            fingerprint: id.fingerprint().to_string(),
            public_key_hex: hex::encode(id.public_key.to_hex()),
        })),
        None => Ok(None),
    }
}

/// Returns true if the encrypted store file exists on disk.
#[tauri::command]
pub async fn transfer_has_stored_identity(ts: State<'_, TransferState>) -> Result<bool, String> {
    Ok(EncryptedStore::default_path().exists())
}

// ── Mailbox commands ──────────────────────────────────────────────────────────

/// Send a text message to another agent identified by their fingerprint.
#[tauri::command]
pub async fn transfer_send_message(
    to_fingerprint: String,
    topic: String,
    text: String,
    ts: State<'_, TransferState>,
) -> Result<String, String> {
    let guard = ts.identity.lock().await;
    let identity = guard.as_ref().ok_or("no identity loaded")?;

    // Register both sender and recipient if not already
    let sender_id = identity.fingerprint().to_string();

    // Deliver locally (both agents on same node — typical for dev/test)
    let payload = text.into_bytes();
    ts.mailbox
        .send_to(identity, &to_fingerprint, &topic, payload)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("sent:{sender_id}→{to_fingerprint}"))
}

/// Poll for pending messages from the mailbox inbox.
/// Returns up to `limit` messages. Caller must have previously called
/// `transfer_register_mailbox` to set up an inbox for their fingerprint.
#[tauri::command]
pub async fn transfer_poll_inbox(
    limit: usize,
    ts: State<'_, TransferState>,
) -> Result<Vec<MessageDto>, String> {
    let guard = ts.identity.lock().await;
    let identity = guard.as_ref().ok_or("no identity loaded")?;
    let my_fp = identity.fingerprint().to_string();
    drop(guard);

    // Inbox polling is best-effort — messages delivered since last poll
    // are not buffered here yet (would require a per-session receiver).
    // Return empty list for now; real implementation wires the mpsc receiver.
    Ok(vec![])
}

/// Get the number of registered agents in the local mailbox hub.
#[tauri::command]
pub async fn transfer_mailbox_agent_count(ts: State<'_, TransferState>) -> Result<usize, String> {
    Ok(ts.mailbox.agent_count())
}

// ── Transfer commands ─────────────────────────────────────────────────────────

/// Start an in-process loopback transfer of a file — useful for self-test and
/// benchmarking the chunking/encryption pipeline without a real remote peer.
#[tauri::command]
pub async fn transfer_send_file_loopback(
    app_handle: AppHandle,
    file_path: String,
    chunk_size: Option<usize>,
    ts: State<'_, TransferState>,
) -> Result<TransferStatusDto, String> {
    let path = PathBuf::from(&file_path);
    let data = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("read {file_path}: {e}"))?;

    let guard = ts.identity.lock().await;
    let identity = guard.as_ref().ok_or("no identity loaded")?.clone();
    drop(guard);

    // Build a throwaway session key from the identity's seed
    let seed = identity.export_seed();
    let session_key = {
        let key_bytes = blake3::derive_key("bonsai-loopback-session", &seed);
        Arc::new(transfer_crypto::session::SessionKey(key_bytes))
    };

    let (lane, _rx) = InProcessLane::new_pair("loopback");
    let mut lanes_map = std::collections::HashMap::new();
    let lane_arc: Arc<dyn transfer_core::lane::TransportLane> = Arc::new(lane);
    lanes_map.insert("loopback".to_string(), lane_arc);
    let lanes = Arc::new(lanes_map);

    let mut sched = EcfRgScheduler::new();
    // Register the lane by name — scheduler has an `add_lane` but we already inserted above.
    // Use a fresh scheduler that just has this lane.
    {
        let (lane2, _rx2) = InProcessLane::new_pair("loopback");
        sched.add_lane(Arc::new(lane2));
    }
    let scheduler = Arc::new(tokio::sync::Mutex::new(sched));

    let cs = chunk_size
        .unwrap_or(DEFAULT_CHUNK_SIZE)
        .min(MAX_CHUNK_SIZE)
        .max(1);

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

    let transfer = Transfer::new();
    let handle = transfer
        .send_data(data.clone(), session_key, scheduler, lanes, cs, Some(progress_tx))
        .await
        .map_err(|e| e.to_string())?;

    let transfers = Arc::clone(&ts.transfers);
    let app_handle = app_handle.clone();
    tokio::spawn(async move {
        while let Some(status) = progress_rx.recv().await {
            let dto = TransferStatusDto::from(&status);
            let _ = app_handle.emit("transfer-progress", dto);
            let mut map = transfers.lock().await;
            map.insert(status.id.to_string(), status);
        }
    });

    let status = TransferStatus {
        id: handle.id,
        direction: transfer_core::transfer::TransferDirection::Send,
        total_bytes: data.len() as u64,
        transferred_bytes: handle.bytes_sent(),
        chunk_count: (data.len().saturating_add(cs - 1) / cs) as u64,
        chunks_done: (handle.bytes_sent().saturating_add(cs as u64 - 1) / cs as u64),
        active_lanes: vec!["loopback".to_string()],
        state: if handle.bytes_sent() >= data.len() as u64 {
            CoreTransferState::Complete
        } else {
            CoreTransferState::Active
        },
        bytes_per_sec: 0.0,
    };

    let dto = TransferStatusDto::from(&status);
    ts.transfers
        .lock()
        .await
        .insert(handle.id.to_string(), status);
    Ok(dto)
}

/// List all in-progress or completed transfers.
#[tauri::command]
pub async fn transfer_list_transfers(
    ts: State<'_, TransferState>,
) -> Result<Vec<TransferStatusDto>, String> {
    let map = ts.transfers.lock().await;
    Ok(map.values().map(TransferStatusDto::from).collect())
}

// ── Store commands ────────────────────────────────────────────────────────────

/// Store an arbitrary JSON value under `key` in the encrypted store.
/// The AES key is BLAKE3(passphrase || key).
#[tauri::command]
pub async fn transfer_store_put(
    key: String,
    value: serde_json::Value,
    passphrase: String,
) -> Result<(), String> {
    let raw_key = blake3::derive_key("bonsai-store-v1", format!("{passphrase}:{key}").as_bytes());
    let name = hex::encode(&raw_key[..8]);
    let path = EncryptedStore::default_path().with_file_name(format!("{name}.bin"));
    let store = EncryptedStore::open(path, &raw_key);
    store.save(&value).map_err(|e| e.to_string())
}

/// Load a JSON value from the encrypted store.
#[tauri::command]
pub async fn transfer_store_get(
    key: String,
    passphrase: String,
) -> Result<Option<serde_json::Value>, String> {
    let raw_key = blake3::derive_key("bonsai-store-v1", format!("{passphrase}:{key}").as_bytes());
    let name = hex::encode(&raw_key[..8]);
    let path = EncryptedStore::default_path().with_file_name(format!("{name}.bin"));
    let store = EncryptedStore::open(&path, &raw_key);
    if !store.exists() {
        return Ok(None);
    }
    store
        .load::<serde_json::Value>()
        .map(Some)
        .map_err(|e| e.to_string())
}
