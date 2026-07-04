//! Shared daemon state.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use bonsai_ci::OrchestratorActor;
use bonsai_ui_orchestrator::UIOrchestrator;
use creator::CreatorOrchestrator;
use mailbox::AgentMailbox;
use p2p::WebRtcLane;
use query::sql::SqlEngine;
use tool_registry::ToolRegistry;
use p2p_core::lane::TransportLane;
use p2p_core::transfer::{TransferHandle, TransferStatus};
use p2p_crypto::identity::BonsaiIdentity;
use transfer_store::EncryptedStore;

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
    /// Optional UI orchestrator for generative UI workflows
    pub ui_orchestrator: Mutex<Option<Arc<UIOrchestrator>>>,
    /// In-memory SQL engine (per-session; not persisted).
    pub sql: Mutex<SqlEngine>,
    /// Hot-swappable tool/skill registry.
    pub tools: Arc<ToolRegistry>,
    /// Active P2P transport lanes (keyed by lane name).
    pub p2p_lanes: Mutex<HashMap<String, Arc<dyn TransportLane>>>,
    /// WebRTC-specific lane handles (for SDP signaling after offer creation).
    pub webrtc_lanes: Mutex<HashMap<String, Arc<WebRtcLane>>>,
    /// Generative AI creator orchestrator (image/video/3d/audio).
    pub creator: Arc<CreatorOrchestrator>,
}

impl DaemonState {
    pub fn new(token: String, cas: Arc<cas::CasStore>) -> Self {
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
            ui_orchestrator: Mutex::new(None),
            sql: Mutex::new(sql),
            tools: Arc::new(ToolRegistry::new()),
            p2p_lanes: Mutex::new(HashMap::new()),
            webrtc_lanes: Mutex::new(HashMap::new()),
            creator: Arc::new(CreatorOrchestrator::new(cas)),
        }
    }
}
