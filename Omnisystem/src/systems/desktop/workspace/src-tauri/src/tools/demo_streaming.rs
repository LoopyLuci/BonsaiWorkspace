use crate::tool_registry::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use tokio::time::{sleep, Duration};

pub struct DemoStreamingTool;

impl DemoStreamingTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DemoStreamingTool {
    fn name(&self) -> &str {
        "demo_streaming"
    }
    fn description(&self) -> &str {
        "Demo long-running tool that streams progress updates"
    }

    async fn run(&self, _args: &Value) -> Result<ToolResult, String> {
        Ok(ToolResult::text("Completed (no streaming)"))
    }

    async fn run_with_progress(
        &self,
        _args: &Value,
        progress_tx: tokio::sync::mpsc::UnboundedSender<serde_json::Value>,
    ) -> Result<ToolResult, String> {
        for i in 0..5 {
            let pct = (i as f32) / 5.0;
            let _ = progress_tx
                .send(serde_json::json!({ "progress": pct, "message": format!("step {}", i) }));
            sleep(Duration::from_millis(300)).await;
        }
        let _ = progress_tx.send(serde_json::json!({ "progress": 1.0, "message": "done" }));
        Ok(ToolResult::text("Demo streaming finished"))
    }
}
