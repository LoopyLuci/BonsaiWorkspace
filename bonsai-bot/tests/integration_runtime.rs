use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;

use bonsai_bot::metrics::Metrics;
use tokio_rusqlite::Connection;

#[tokio::test]
async fn integration_runtime_start_stop() -> Result<(), Box<dyn std::error::Error>> {
    // Prepare server state
    let metrics = Arc::new(Metrics::default());
    let platform_states = Arc::new(DashMap::new());
    let db = Arc::new(Connection::open_in_memory().await?);
    bonsai_bot::session::migrate(&db).await?;
    let (tx, _rx) = mpsc::channel::<bonsai_bot::admin_api::BroadcastRequest>(1);

    let admin_token = "itest-token".to_string();
    // start admin server on ephemeral port
    let mut handle = bonsai_bot::admin_api::start(0, metrics, platform_states, db, tx, admin_token.clone())
        .await
        .map_err(|e| format!("failed to start admin server: {}", e))?;

    let client = Client::new();
    let base = format!("http://127.0.0.1:{}", handle.port);

    // Try Python runtime if available
    if which::which("python").is_ok() {
        let start_req = json!({
            "kind": "python",
            "script": "runtimes/python/worker.py",
            "port": 8123,
            "user": "itest",
            "timeout_secs": 6
        });
        let resp = client.post(format!("{}/runtime/start", base))
            .header("authorization", format!("Bearer {}", admin_token))
            .json(&start_req)
            .send().await?;
        assert!(resp.status().is_success(), "start python runtime failed: {}", resp.text().await?);
        let j: serde_json::Value = resp.json().await?;
        let id = j.get("id").and_then(|v| v.as_str()).unwrap().to_string();

        // list runtimes
        let list = client.get(format!("{}/runtime/list", base))
            .header("authorization", format!("Bearer {}", admin_token))
            .send().await?.json::<serde_json::Value>().await?;
        assert!(list["runtimes"].as_array().unwrap().iter().any(|r| r["id"] == id));

        // stop
        let stop_req = json!({"id": id});
        let stop = client.post(format!("{}/runtime/stop", base))
            .header("authorization", format!("Bearer {}", admin_token))
            .json(&stop_req)
            .send().await?;
        assert!(stop.status().is_success(), "stop python runtime failed: {}", stop.text().await?);
    } else {
        eprintln!("python not found; skipping python runtime integration test");
    }

    // Try Babashka runtime if available
    if which::which("bb").is_ok() {
        let start_req = json!({
            "kind": "babashka",
            "script": "runtimes/clojure/bb_runner.clj",
            "user": "itest"
        });
        let resp = client.post(format!("{}/runtime/start", base))
            .header("authorization", format!("Bearer {}", admin_token))
            .json(&start_req)
            .send().await?;
        assert!(resp.status().is_success(), "start bb runtime failed: {}", resp.text().await?);
        let j: serde_json::Value = resp.json().await?;
        let id = j.get("id").and_then(|v| v.as_str()).unwrap().to_string();

        let list = client.get(format!("{}/runtime/list", base))
            .header("authorization", format!("Bearer {}", admin_token))
            .send().await?.json::<serde_json::Value>().await?;
        assert!(list["runtimes"].as_array().unwrap().iter().any(|r| r["id"] == id));

        let stop_req = json!({"id": id});
        let stop = client.post(format!("{}/runtime/stop", base))
            .header("authorization", format!("Bearer {}", admin_token))
            .json(&stop_req)
            .send().await?;
        assert!(stop.status().is_success(), "stop bb runtime failed: {}", stop.text().await?);
    } else {
        eprintln!("bb not found; skipping babashka runtime integration test");
    }

    // shutdown server
    handle.stop().await;
    Ok(())
}
