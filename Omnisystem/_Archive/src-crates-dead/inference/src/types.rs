use anyhow::Result;
use llama_cpp_rs::{LlamaContext, LlamaParams};

pub trait InferenceBackend: Send + Sync {
    fn generate(&self, tokens: &[u32], temperature: f32, top_p: f32, max_tokens: u32) -> Result<GenerationOutput>;
    fn generate_stream(
        &self,
        tokens: &[u32],
        temperature: f32,
        top_p: f32,
        max_tokens: u32,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<()>;
}

pub struct GenerationOutput {
    pub tokens: Vec<u32>,
}

pub struct LlamaBackend {
    ctx: LlamaContext,
}

impl LlamaBackend {
    pub fn new(model_path: &std::path::Path, n_gpu_layers: u32, n_ctx: u32) -> Result<Self> {
        let params = LlamaParams::default()
            .model_path(model_path.to_string_lossy().to_string())
            .n_gpu_layers(n_gpu_layers)
            .n_ctx(n_ctx);
        let ctx = LlamaContext::new(params)?;
        Ok(Self { ctx })
    }
}

impl InferenceBackend for LlamaBackend {
    fn generate(&self, tokens: &[u32], temperature: f32, top_p: f32, max_tokens: u32) -> Result<GenerationOutput> {
        let output = self.ctx.generate(
            tokens,
            llama_cpp_rs::GenerateParams::default()
                .temperature(temperature)
                .top_p(top_p)
                .max_tokens(max_tokens as i32),
        )?;
        Ok(GenerationOutput { tokens: output.tokens })
    }

    fn generate_stream(
        &self,
        tokens: &[u32],
        temperature: f32,
        top_p: f32,
        max_tokens: u32,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<()> {
        self.ctx.generate_stream(
            tokens,
            llama_cpp_rs::GenerateParams::default()
                .temperature(temperature)
                .top_p(top_p)
                .max_tokens(max_tokens as i32),
            |token| {
                callback(token);
            },
        )?;
        Ok(())
    }
}
