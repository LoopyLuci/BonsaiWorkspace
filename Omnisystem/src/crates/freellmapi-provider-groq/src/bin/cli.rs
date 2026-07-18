//! Demo CLI: translates an OpenAI-shaped chat request into Groq's wire format
//! using the real adapter. Does not make a network call (no API key required)
//! - set GROQ_API_KEY and use send_request in your own integration for that.

use freellmapi_core::{ChatMessage, OpenAIChatRequest};
use freellmapi_providers_base::ProviderAdapter;
use freellmapi_provider_groq::GroqAdapter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("GROQ_API_KEY").unwrap_or_else(|_| "unset".to_string());
    let adapter = GroqAdapter::new(api_key);

    let request = OpenAIChatRequest {
        model: "llama3-8b-8192".to_string(),
        messages: vec![ChatMessage { role: "user".to_string(), content: "Say hi".to_string() }],
        stream: false,
        temperature: 0.7,
        max_tokens: Some(64),
        tools: None,
        tool_choice: None,
    };

    let translated = adapter.translate_request(&request).await?;
    println!("Translated provider request: {}", serde_json::to_string_pretty(&translated)?);

    let models = adapter.get_supported_models().await?;
    println!("Supported models: {models:?}");

    Ok(())
}
