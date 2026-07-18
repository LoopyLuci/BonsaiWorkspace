use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub workspace_path: Option<PathBuf>,
    pub api_port: u16,
    pub dream_agent_port: u16,
    /// Minutes of user inactivity before triggering an opportunistic consolidation.
    pub idle_trigger_mins: u64,
}

impl Config {
    pub fn from_env_or_defaults() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("com.bonsai.workspace");

        Self {
            db_path: std::env::var("BONSAI_MEMORY_DB")
                .map(PathBuf::from)
                .unwrap_or_else(|_| data_dir.join("memory_nodes.db")),

            workspace_path: std::env::var("BONSAI_WORKSPACE_PATH")
                .ok()
                .map(PathBuf::from),

            api_port: std::env::var("BONSAI_API_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(11369),

            dream_agent_port: std::env::var("BONSAI_DREAM_AGENT_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8082),

            idle_trigger_mins: std::env::var("BONSAI_IDLE_TRIGGER_MINS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        }
    }

    pub fn dream_agent_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.dream_agent_port)
    }

    pub fn app_api_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.api_port)
    }
}
