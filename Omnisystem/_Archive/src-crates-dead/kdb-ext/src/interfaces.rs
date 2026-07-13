//! Public interfaces for bonsai-kdb-ext

pub trait Component {
    async fn init(&mut self) -> Result<(), anyhow::Error>;
    fn name(&self) -> &str;
}
