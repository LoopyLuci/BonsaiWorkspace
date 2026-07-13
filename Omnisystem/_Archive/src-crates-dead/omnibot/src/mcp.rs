// MCP (Model Context Protocol) client

use serde_json::{json, Value};

pub struct McpClient {
    url: String,
    client: reqwest::Client,
}

impl McpClient {
    pub async fn new(url: String) -> anyhow::Result<Self> {
        Ok(Self {
            url,
            client: reqwest::Client::new(),
        })
    }

    pub async fn call_tool(&self, tool_name: &str, params: &Value) -> anyhow::Result<String> {
        let url = format!("{}/tools/{}/call", self.url, tool_name);

        let response = self.client
            .post(&url)
            .json(&json!({
                "arguments": params
            }))
            .send()
            .await?;

        let body: Value = response.json().await?;

        match body.get("result") {
            Some(Value::String(s)) => Ok(s.clone()),
            Some(v) => Ok(v.to_string()),
            None => Ok("Tool executed successfully".into()),
        }
    }

    pub async fn call_tool_with_args(&self, tool_name: &str, args: serde_json::Map<String, Value>) -> anyhow::Result<String> {
        self.call_tool(tool_name, &Value::Object(args)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_creation() {
        let result = McpClient::new("http://localhost:8000".into()).await;
        assert!(result.is_ok());
    }
}
