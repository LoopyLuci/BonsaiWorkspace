use anyhow::Result;
use bonsai_ci::{BonsaiCi, CiConfig};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let ws = env::current_dir()?;

    // Optional CAS location via env
    let cas_db = env::var("BONSAI_CAS_DB").ok();
    let cas_blob = env::var("BONSAI_CAS_BLOB_DIR").ok();

    let mut config = CiConfig::default();
    config.workspace_root = ws.clone();

    if let (Some(db), Some(blob)) = (cas_db, cas_blob) {
        let db = PathBuf::from(db);
        let blob = PathBuf::from(blob);
        let cas = Arc::new(cas::CasStore::open(&db, &blob).await?);

        // Optional identity seed hex for signing
        let identity = env::var("BONSAI_IDENTITY_SEED").ok().and_then(|s| {
            let bytes = hex::decode(s).ok()?;
            if bytes.len() != 32 { return None; }
            let mut arr = [0u8;32]; arr.copy_from_slice(&bytes);
            // TODO: wire up real identity from p2p_identity crate
            // p2p_identity::NodeId::from_seed(&arr).ok().map(Arc::new)
            let _ = arr;  // suppress unused warning
            None
        });

        let ci = BonsaiCi::with_cas_and_identity(config, cas, identity);
        let results = ci.run_full_pipeline().await;
        println!("Pipeline results: {:#?}", results);
    } else {
        // No CAS configured: run locally without uploading
        let ci = BonsaiCi::new(config);
        let results = ci.run_full_pipeline().await;
        println!("Pipeline results: {:#?}", results);
    }

    Ok(())
}
