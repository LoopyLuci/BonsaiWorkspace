//! Generalized cloud-provider model catalog.
//!
//! Every provider listed here exposes a real, public "list models" HTTP
//! endpoint. This module knows how to call each one and normalize the
//! response into `ProviderModel` — the catalog/metadata concern only.
//!
//! Chat *execution* (actually sending a message and getting a completion)
//! is a separate concern with its own per-provider wire-protocol quirks;
//! `opencode_go.rs` is the one provider with a working chat client so far.
//! Nothing here assumes a model returned by `fetch_models` is chat-capable
//! through this module.
//!
//! API keys are stored per-provider in the OS keychain via `SecretsStore`
//! under the account name `"{provider_id}_api_key"` — the same convention
//! `model_data_generator.rs` already assumed for its `api_key_secret_name`.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStyle {
    /// `Authorization: Bearer <key>` — OpenAI and most OpenAI-compatible APIs.
    BearerHeader,
    /// `x-api-key: <key>` — Anthropic's native API.
    XApiKeyHeader,
    /// `?key=<key>` query parameter — Google's Generative Language API.
    QueryParamKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListShape {
    /// `{ "data": [{ "id": "..." }, ...] }` — OpenAI and most OpenAI-compatible APIs.
    OpenAiData,
    /// `{ "data": [{ "id": "...", "display_name": "..." }, ...] }` — Anthropic.
    AnthropicData,
    /// `{ "models": [{ "name": "models/x", "displayName": "...", "inputTokenLimit": N, "outputTokenLimit": N }] }` — Gemini.
    GeminiModels,
    /// A bare JSON array of `{ "id": "..." }` objects — Together AI.
    RawArray,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderDef {
    pub id: &'static str,
    pub display_name: &'static str,
    pub base_url: &'static str,
    pub models_path: &'static str,
    pub auth: AuthStyle,
    pub shape: ListShape,
    /// Extra static headers needed beyond auth (e.g. Anthropic's version header).
    pub extra_headers: &'static [(&'static str, &'static str)],
}

/// The known provider registry. Adding a provider that exposes a standard
/// list-models endpoint is a one-line addition here — no other code needs
/// to change (secrets, commands, and the frontend all iterate this list).
pub fn known_providers() -> &'static [ProviderDef] {
    &[
        ProviderDef {
            id: "anthropic",
            display_name: "Anthropic (Claude)",
            base_url: "https://api.anthropic.com/v1",
            models_path: "/models",
            auth: AuthStyle::XApiKeyHeader,
            shape: ListShape::AnthropicData,
            extra_headers: &[("anthropic-version", "2023-06-01")],
        },
        ProviderDef {
            id: "openai",
            display_name: "OpenAI (ChatGPT)",
            base_url: "https://api.openai.com/v1",
            models_path: "/models",
            auth: AuthStyle::BearerHeader,
            shape: ListShape::OpenAiData,
            extra_headers: &[],
        },
        ProviderDef {
            id: "gemini",
            display_name: "Google Gemini",
            base_url: "https://generativelanguage.googleapis.com/v1beta",
            models_path: "/models",
            auth: AuthStyle::QueryParamKey,
            shape: ListShape::GeminiModels,
            extra_headers: &[],
        },
        ProviderDef {
            id: "deepseek",
            display_name: "DeepSeek",
            base_url: "https://api.deepseek.com/v1",
            models_path: "/models",
            auth: AuthStyle::BearerHeader,
            shape: ListShape::OpenAiData,
            extra_headers: &[],
        },
        ProviderDef {
            id: "groq",
            display_name: "Groq",
            base_url: "https://api.groq.com/openai/v1",
            models_path: "/models",
            auth: AuthStyle::BearerHeader,
            shape: ListShape::OpenAiData,
            extra_headers: &[],
        },
        ProviderDef {
            id: "mistral",
            display_name: "Mistral",
            base_url: "https://api.mistral.ai/v1",
            models_path: "/models",
            auth: AuthStyle::BearerHeader,
            shape: ListShape::OpenAiData,
            extra_headers: &[],
        },
        ProviderDef {
            id: "together",
            display_name: "Together AI",
            base_url: "https://api.together.xyz/v1",
            models_path: "/models",
            auth: AuthStyle::BearerHeader,
            shape: ListShape::RawArray,
            extra_headers: &[],
        },
        ProviderDef {
            id: "openrouter",
            display_name: "OpenRouter",
            base_url: "https://openrouter.ai/api/v1",
            models_path: "/models",
            auth: AuthStyle::BearerHeader,
            shape: ListShape::OpenAiData,
            extra_headers: &[],
        },
        ProviderDef {
            id: "xai",
            display_name: "xAI (Grok)",
            base_url: "https://api.x.ai/v1",
            models_path: "/models",
            auth: AuthStyle::BearerHeader,
            shape: ListShape::OpenAiData,
            extra_headers: &[],
        },
        ProviderDef {
            id: "opencode-zen",
            display_name: "OpenCode Zen",
            base_url: "https://opencode.ai/zen/v1",
            models_path: "/models",
            auth: AuthStyle::BearerHeader,
            shape: ListShape::OpenAiData,
            extra_headers: &[],
        },
    ]
}

pub fn find_provider(id: &str) -> Option<&'static ProviderDef> {
    known_providers().iter().find(|p| p.id.eq_ignore_ascii_case(id))
}

/// Account name under which this provider's API key is stored in `SecretsStore`.
pub fn secret_account(provider_id: &str) -> String {
    format!("{}_api_key", provider_id.to_lowercase())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: String,
    pub name: String,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))
}

/// GET the provider's live model catalog using the given API key.
pub async fn fetch_models(def: &ProviderDef, api_key: &str) -> Result<Vec<ProviderModel>, String> {
    let client = http_client()?;
    let url = format!("{}{}", def.base_url, def.models_path);
    let mut req = client.get(&url);
    req = match def.auth {
        AuthStyle::BearerHeader => req.bearer_auth(api_key),
        AuthStyle::XApiKeyHeader => req.header("x-api-key", api_key),
        AuthStyle::QueryParamKey => req.query(&[("key", api_key)]),
    };
    for (k, v) in def.extra_headers {
        req = req.header(*k, *v);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("{} models request failed: {e}", def.display_name))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("{} returned {status}: {text}", def.display_name));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse {} models response: {e}", def.display_name))?;

    parse_models(def.shape, &v)
}

fn parse_models(shape: ListShape, v: &serde_json::Value) -> Result<Vec<ProviderModel>, String> {
    match shape {
        ListShape::OpenAiData => {
            let arr = v["data"]
                .as_array()
                .ok_or("expected a `data` array in the models response")?;
            Ok(arr
                .iter()
                .filter_map(|m| {
                    let id = m["id"].as_str()?.to_string();
                    Some(ProviderModel {
                        name: id.clone(),
                        id,
                        context_window: None,
                        max_output_tokens: None,
                    })
                })
                .collect())
        }
        ListShape::AnthropicData => {
            let arr = v["data"]
                .as_array()
                .ok_or("expected a `data` array in the models response")?;
            Ok(arr
                .iter()
                .filter_map(|m| {
                    let id = m["id"].as_str()?.to_string();
                    let name = m["display_name"].as_str().unwrap_or(&id).to_string();
                    Some(ProviderModel {
                        id,
                        name,
                        context_window: None,
                        max_output_tokens: None,
                    })
                })
                .collect())
        }
        ListShape::GeminiModels => {
            let arr = v["models"]
                .as_array()
                .ok_or("expected a `models` array in the models response")?;
            Ok(arr
                .iter()
                .filter_map(|m| {
                    let raw_name = m["name"].as_str()?;
                    let id = raw_name.strip_prefix("models/").unwrap_or(raw_name).to_string();
                    let name = m["displayName"].as_str().unwrap_or(&id).to_string();
                    Some(ProviderModel {
                        id,
                        name,
                        context_window: m["inputTokenLimit"].as_u64(),
                        max_output_tokens: m["outputTokenLimit"].as_u64(),
                    })
                })
                .collect())
        }
        ListShape::RawArray => {
            let arr = v
                .as_array()
                .ok_or("expected a raw JSON array in the models response")?;
            Ok(arr
                .iter()
                .filter_map(|m| {
                    let id = m["id"].as_str()?.to_string();
                    Some(ProviderModel {
                        name: id.clone(),
                        id,
                        context_window: None,
                        max_output_tokens: None,
                    })
                })
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_provider_id_is_unique_and_lowercase() {
        let mut seen = std::collections::HashSet::new();
        for p in known_providers() {
            assert_eq!(p.id, p.id.to_lowercase(), "provider id must be lowercase: {}", p.id);
            assert!(seen.insert(p.id), "duplicate provider id: {}", p.id);
        }
    }

    #[test]
    fn find_provider_is_case_insensitive() {
        assert!(find_provider("OpenAI").is_some());
        assert!(find_provider("openai").is_some());
        assert!(find_provider("does-not-exist").is_none());
    }

    #[test]
    fn parses_openai_shape() {
        let v = json!({ "data": [{ "id": "gpt-4o" }, { "id": "gpt-4o-mini" }] });
        let out = parse_models(ListShape::OpenAiData, &v).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "gpt-4o");
        assert_eq!(out[0].name, "gpt-4o");
    }

    #[test]
    fn parses_anthropic_shape() {
        let v = json!({ "data": [{ "id": "claude-opus-4-1", "display_name": "Claude Opus 4.1" }] });
        let out = parse_models(ListShape::AnthropicData, &v).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "claude-opus-4-1");
        assert_eq!(out[0].name, "Claude Opus 4.1");
    }

    #[test]
    fn parses_gemini_shape() {
        let v = json!({ "models": [{
            "name": "models/gemini-2.0-flash",
            "displayName": "Gemini 2.0 Flash",
            "inputTokenLimit": 1_000_000,
            "outputTokenLimit": 8192
        }] });
        let out = parse_models(ListShape::GeminiModels, &v).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "gemini-2.0-flash");
        assert_eq!(out[0].context_window, Some(1_000_000));
        assert_eq!(out[0].max_output_tokens, Some(8192));
    }

    #[test]
    fn parses_raw_array_shape() {
        let v = json!([{ "id": "meta-llama/Llama-3-70b-chat-hf" }]);
        let out = parse_models(ListShape::RawArray, &v).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "meta-llama/Llama-3-70b-chat-hf");
    }

    #[test]
    fn rejects_wrong_shape_gracefully() {
        let v = json!({ "unexpected": true });
        assert!(parse_models(ListShape::OpenAiData, &v).is_err());
        assert!(parse_models(ListShape::GeminiModels, &v).is_err());
    }
}
