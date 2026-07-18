//! Demo CLI: builds a ProviderRequest/ProviderResponse pair and prints them,
//! exercising the real shared provider types.

use freellmapi_providers_base::{Choice, Message, ProviderRequest, ProviderResponse, Usage};

fn main() {
    let req = ProviderRequest {
        provider: "demo".to_string(),
        model: "demo-model".to_string(),
        messages: vec![Message { role: "user".to_string(), content: "hello".to_string() }],
        temperature: Some(0.7),
        max_tokens: Some(128),
        top_p: None,
        stream: Some(false),
    };
    println!("Request: {}", serde_json::to_string_pretty(&req).unwrap());

    let resp = ProviderResponse {
        provider: "demo".to_string(),
        model: "demo-model".to_string(),
        choices: vec![Choice {
            index: 0,
            message: Message { role: "assistant".to_string(), content: "hi there".to_string() },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage { prompt_tokens: 3, completion_tokens: 2, total_tokens: 5 },
    };
    println!("Response: {}", serde_json::to_string_pretty(&resp).unwrap());
}
