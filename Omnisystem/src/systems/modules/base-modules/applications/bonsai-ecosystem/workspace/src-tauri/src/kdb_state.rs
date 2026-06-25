use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
// rusqlite::Connection is !Sync, so KdbStore must live behind a Mutex (not RwLock).
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use kdb::{KdbRetriever, KdbStore, Result as KdbResult};

pub struct KdbAppState {
    /// Mutex because rusqlite::Connection is !Sync.
    pub store: Arc<Mutex<KdbStore>>,
    /// RwLock is fine here; KdbRetriever uses std::sync::RwLock internally.
    pub retriever: Arc<RwLock<KdbRetriever>>,
    /// package_id -> list of module names loaded from that package
    pub loaded_package_modules: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl KdbAppState {
    pub fn open(data_dir: &PathBuf) -> KdbResult<Self> {
        let kdb_dir = data_dir.join("kdb");
        let store = KdbStore::open(&kdb_dir)?;
        // 384-dim matches all-MiniLM-L6-v2 / nomic-embed-text; top_k=5 per module.
        let retriever = KdbRetriever::new(384, 5);
        Ok(KdbAppState {
            store: Arc::new(Mutex::new(store)),
            retriever: Arc::new(RwLock::new(retriever)),
            loaded_package_modules: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}
