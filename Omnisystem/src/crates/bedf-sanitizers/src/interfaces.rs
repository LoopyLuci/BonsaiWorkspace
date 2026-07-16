//! Public interfaces for bonsai-bedf-sanitizers

pub trait Component {
    fn init(&mut self) -> impl std::future::Future<Output = Result<(), anyhow::Error>> + Send;
    fn name(&self) -> &str;
}
