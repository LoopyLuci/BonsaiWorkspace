use async_trait::async_trait;
use anyhow::Result;
use freellmapi_core::{OpenAIChatRequest, OpenAIChatResponse};
use crate::types::{ProviderRequest, ProviderResponse};

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn provider_name(&self) -> &str;

    async fn authenticate(&self, api_key: &str) -> Result<()>;

    async fn translate_request(&self, req: &OpenAIChatRequest) -> Result<ProviderRequest>;

    async fn send_request(&self, provider_req: &ProviderRequest) -> Result<ProviderResponse>;

    async fn translate_response(&self, resp: &ProviderResponse) -> Result<OpenAIChatResponse>;

    async fn get_supported_models(&self) -> Result<Vec<String>>;

    async fn health_check(&self) -> Result<bool>;
}
