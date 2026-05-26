//! HTTP client for the Bonsai Workspace management API (`/api/v1/*`).
//!
//! The workspace exposes every major capability over REST using the same
//! `Authorization: Bearer <pair_token>` scheme shown in Settings →
//! Desktop Connection. Set `workspace_pair_token` in bonsai-bot-config.json.

use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;
use serde_json::{json, Value};

pub struct MgmtClient {
    http:  Client,
    base:  String,   // e.g. "http://127.0.0.1:11372/api/v1"
    token: String,
}

impl MgmtClient {
    pub fn new(workspace_api_url: &str, pair_token: String) -> Arc<Self> {
        let base = format!("{}/api/v1", workspace_api_url.trim_end_matches('/'));
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("mgmt reqwest client");
        Arc::new(Self { http, base, token: pair_token })
    }

    fn auth(&self) -> String { format!("Bearer {}", self.token) }

    async fn get(&self, path: &str) -> Result<Value, String> {
        let resp = self.http
            .get(format!("{}/{}", self.base, path))
            .header("authorization", self.auth())
            .send().await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        if status.is_success() {
            Ok(body)
        } else {
            Err(body["error"].as_str().unwrap_or(&status.to_string()).to_string())
        }
    }

    async fn post(&self, path: &str, payload: Value) -> Result<Value, String> {
        let resp = self.http
            .post(format!("{}/{}", self.base, path))
            .header("authorization", self.auth())
            .json(&payload)
            .send().await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        if status.is_success() {
            Ok(body)
        } else {
            Err(body["error"].as_str().unwrap_or(&status.to_string()).to_string())
        }
    }

    pub async fn swarm_submit(&self, prompt: &str) -> Result<Value, String> {
        self.post("swarm/submit", json!({
            "messages": [{"role": "user", "content": prompt}]
        })).await
    }

    pub async fn list_agents(&self) -> Result<Value, String> {
        self.get("agents/list").await
    }

    pub async fn agent_message(&self, agent_id: &str, content: &str) -> Result<Value, String> {
        self.post("agents/message", json!({
            "agentId": agent_id,
            "message": {"role": "user", "content": content}
        })).await
    }

    pub async fn get_features(&self) -> Result<Value, String> {
        self.get("features").await
    }

    pub async fn set_feature(&self, key: &str, value: bool) -> Result<(), String> {
        self.post("features", json!({ key: value })).await.map(|_| ())
    }

    pub async fn list_models(&self) -> Result<Value, String> {
        self.get("models/list").await
    }

    pub async fn load_model(&self, model_id: &str) -> Result<Value, String> {
        self.post("models/load", json!({ "model_id": model_id })).await
    }

    pub async fn queue_status(&self) -> Result<Value, String> {
        self.get("queue/status").await
    }

    /// Returns true if the management API is reachable and the token is valid.
    pub async fn is_reachable(&self) -> bool {
        self.get("features").await.is_ok()
    }

    // ── Chess / Go game commands ────────────────────────────────────────────────

    pub async fn chess_new(&self, player_name: &str, human_color: &str, strength: &str) -> Result<Value, String> {
        self.post("chess/new", json!({
            "player_name": player_name,
            "human_color": human_color,
            "ai_strength": strength,
        })).await
    }

    pub async fn chess_move(&self, game_id: &str, notation: &str) -> Result<Value, String> {
        self.post("chess/move", json!({ "game_id": game_id, "notation": notation })).await
    }

    pub async fn chess_resign(&self, game_id: &str) -> Result<Value, String> {
        self.post("chess/resign", json!({ "game_id": game_id })).await
    }

    pub async fn chess_status(&self, game_id: &str) -> Result<Value, String> {
        self.get(&format!("chess/game/{game_id}")).await
    }

    pub async fn go_new(&self, player_name: &str, human_color: &str, board_size: u8, komi: f32) -> Result<Value, String> {
        self.post("go/new", json!({
            "player_name": player_name,
            "human_color": human_color,
            "board_size": board_size,
            "komi": komi,
        })).await
    }

    pub async fn go_move(&self, game_id: &str, gtp: &str) -> Result<Value, String> {
        self.post("go/move", json!({ "game_id": game_id, "gtp": gtp })).await
    }

    pub async fn go_resign(&self, game_id: &str) -> Result<Value, String> {
        self.post("go/resign", json!({ "game_id": game_id })).await
    }

    pub async fn go_status(&self, game_id: &str) -> Result<Value, String> {
        self.get(&format!("go/game/{game_id}")).await
    }

    pub async fn puzzle_daily(&self) -> Result<Value, String> {
        self.get("puzzle/daily").await
    }

    pub async fn puzzle_check(&self, puzzle_id: &str, uci_move: &str) -> Result<Value, String> {
        self.post("puzzle/check", json!({ "puzzle_id": puzzle_id, "uci_move": uci_move })).await
    }

    pub async fn tournament_list(&self) -> Result<Value, String> {
        self.get("tournament/list").await
    }

    pub async fn tournament_create(&self, name: &str, agents: &[&str]) -> Result<Value, String> {
        self.post("tournament/create", json!({
            "name": name,
            "game_type": "chess",
            "agent_ids": agents,
            "agent_names": agents,
        })).await
    }
}
