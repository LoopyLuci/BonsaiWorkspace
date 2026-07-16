//! Public interfaces for bonsai-bedf-fuzzing

/// Async init/name interface for wiring a fuzz target into a larger host
/// system. `async fn` in a public trait intentionally: this trait is meant
/// to be implemented and called directly (not through `dyn Component`), so
/// the usual auto-trait-bound caveat doesn't apply here.
#[allow(async_fn_in_trait)]
pub trait Component {
    async fn init(&mut self) -> Result<(), anyhow::Error>;
    fn name(&self) -> &str;
}
