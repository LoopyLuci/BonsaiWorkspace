//! Shared daemon state.

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;

use bonsai_transfer_crypto::identity::BonsaiIdentity;
use bonsai_transfer_store::EncryptedStore;
use bonsai_mailbox::AgentMailbox;
use bonsai_query::sql::SqlEngine;
use bonsai_transfer_core::transfer::{TransferStatus, TransferHandle};
use bonsai_ci::OrchestratorActor;
use bonsai_tool_registry::ToolRegistry;

pub struct DaemonState {
    /// Auth token — compared on every WebSocket connection handshake.
    pub token: String,
    /// Currently loaded identity (set after identity.create or identity.unlock).
    pub identity: Mutex<Option<Arc<BonsaiIdentity>>>,
    /// Encrypted persistence store (file-backed).
    pub store: EncryptedStore,
    /// Agent mailbox for local inter-agent messaging.
    pub mailbox: AgentMailbox,
    /// In-memory transfer status map (id -> last known status)
    pub transfers: Mutex<HashMap<String, TransferStatus>>,
    /// Active transfer handles for cancellation (id -> handle)
    pub transfer_handles: Mutex<HashMap<String, TransferHandle>>,
    /// Optional CI orchestrator (Phase 1 lightweight actor)
    pub orchestrator: Mutex<Option<OrchestratorActor>>,
    /// In-memory SQL engine (per-session; not persisted).
    pub sql: Mutex<SqlEngine>,
    /// Hot-swappable tool/skill registry.
    pub tools: Arc<ToolRegistry>,
}

impl DaemonState {
    pub fn new(token: String) -> Self {
        let store_path = EncryptedStore::default_path();
        let sql = SqlEngine::in_memory().expect("SQLite in-memory");

        Self {
            token,
            identity: Mutex::new(None),
            store: EncryptedStore::open(store_path, b"bonsai-daemon-store-v1"),
            mailbox: AgentMailbox::new(),
            transfers: Mutex::new(HashMap::new()),
            transfer_handles: Mutex::new(HashMap::new()),
            orchestrator: Mutex::new(None),
            sql: Mutex::new(sql),
            tools: Arc::new(ToolRegistry::new()),
        }
    }
}
