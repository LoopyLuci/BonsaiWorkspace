//! Public interfaces for bonsai-bedf-mcp

pub trait Component {
    async fn init(&mut self) -> Result<(), anyhow::Error>;
    fn name(&self) -> &str;
}
