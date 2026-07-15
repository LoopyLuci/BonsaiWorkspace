use anyhow::Result;

pub async fn passthrough_jsonrpc(mcp_url: &str, payload: serde_json::Value) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let response = client
        .post(mcp_url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;
    Ok(response.json().await?)
}
