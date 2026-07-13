use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
struct TokenEnvelope {
    capabilities: Vec<String>,
}

pub fn require_capability(required: &str) -> Result<()> {
    let raw = std::env::var("BONSAI_CAP_TOKEN")
        .map_err(|_| anyhow!("missing BONSAI_CAP_TOKEN for privileged command"))?;
    let capabilities = parse_capabilities(&raw)?;

    if capabilities.contains("DevKitCap:*") || capabilities.contains(required) {
        Ok(())
    } else {
        Err(anyhow!("missing capability: {required}"))
    }
}

fn parse_capabilities(raw: &str) -> Result<HashSet<String>> {
    if raw.trim().is_empty() {
        return Err(anyhow!("capability token is empty"));
    }

    if let Ok(env) = serde_json::from_str::<TokenEnvelope>(raw) {
        return Ok(env.capabilities.into_iter().collect());
    }

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(items) = json.get("capabilities").and_then(|v| v.as_array()) {
            let mut set = HashSet::new();
            for item in items {
                if let Some(s) = item.as_str() {
                    set.insert(s.to_string());
                }
            }
            return Ok(set);
        }
    }

    let parsed: HashSet<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    if parsed.is_empty() {
        Err(anyhow!("failed to parse capabilities from BONSAI_CAP_TOKEN"))
    } else {
        Ok(parsed)
    }
}
