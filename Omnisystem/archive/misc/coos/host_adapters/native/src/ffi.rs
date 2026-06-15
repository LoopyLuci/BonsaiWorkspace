//! Raw FFI bindings to the llama.cpp C API.
//! Only the subset needed for BonsAI inference + LoRA is exposed here.

use std::ffi::{c_char, c_float, c_int, c_uint};

// ── Opaque handle types ───────────────────────────────────────────────────────

#[repr(C)]
pub struct LlamaModel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct LlamaContext {
    _private: [u8; 0],
}

// ── Parameter structs ─────────────────────────────────────────────────────────

#[repr(C)]
pub struct LlamaModelParams {
    pub n_gpu_layers: c_int,
    pub main_gpu: c_int,
    pub vocab_only: bool,
    pub use_mmap: bool,
    pub use_mlock: bool,
}

impl Default for LlamaModelParams {
    fn default() -> Self {
        // SAFETY: calling llama_model_default_params() would be ideal but we
        // provide a safe compile-time default for offline use.
        Self {
            n_gpu_layers: 0,
            main_gpu: 0,
            vocab_only: false,
            use_mmap: true,
            use_mlock: false,
        }
    }
}

#[repr(C)]
pub struct LlamaContextParams {
    pub n_ctx: c_uint,
    pub n_batch: c_uint,
    pub n_threads: c_int,
    pub n_threads_batch: c_int,
    pub rope_scaling_type: c_int,
    pub rope_freq_base: c_float,
    pub rope_freq_scale: c_float,
}

impl Default for LlamaContextParams {
    fn default() -> Self {
        Self {
            n_ctx: 4096,
            n_batch: 512,
            n_threads: 4,
            n_threads_batch: 4,
            rope_scaling_type: -1,
            rope_freq_base: 0.0,
            rope_freq_scale: 0.0,
        }
    }
}

// ── Token type ────────────────────────────────────────────────────────────────

pub type LlamaToken = c_int;

// ── Extern C block ────────────────────────────────────────────────────────────

extern "C" {
    pub fn llama_backend_init();
    pub fn llama_backend_free();
    pub fn llama_numa_init(numa: c_int);

    pub fn llama_model_default_params() -> LlamaModelParams;
    pub fn llama_load_model_from_file(
        path_model: *const c_char,
        params: LlamaModelParams,
    ) -> *mut LlamaModel;
    pub fn llama_free_model(model: *mut LlamaModel);

    pub fn llama_context_default_params() -> LlamaContextParams;
    pub fn llama_new_context_with_model(
        model: *mut LlamaModel,
        params: LlamaContextParams,
    ) -> *mut LlamaContext;
    pub fn llama_free(ctx: *mut LlamaContext);

    pub fn llama_model_apply_lora_from_file(
        model: *mut LlamaModel,
        path_lora: *const c_char,
        scale: c_float,
        path_base_model: *const c_char,
        n_threads: c_int,
    ) -> c_int;

    pub fn llama_n_ctx(ctx: *const LlamaContext) -> c_uint;
    pub fn llama_n_ctx_train(model: *const LlamaModel) -> c_int;
    pub fn llama_model_n_params(model: *const LlamaModel) -> u64;

    // Tokenisation
    pub fn llama_tokenize(
        model: *const LlamaModel,
        text: *const c_char,
        text_len: c_int,
        tokens: *mut LlamaToken,
        n_tokens_max: c_int,
        add_special: bool,
        parse_special: bool,
    ) -> c_int;

    // Decode / sampling (minimal surface — full pipeline lives in engine.rs)
    pub fn llama_decode(ctx: *mut LlamaContext, batch: LlamaBatch) -> c_int;
    pub fn llama_get_logits_ith(ctx: *mut LlamaContext, i: c_int) -> *mut c_float;

    // Batch
    pub fn llama_batch_init(n_tokens: c_int, embd: c_int, n_seq_max: c_int) -> LlamaBatch;
    pub fn llama_batch_free(batch: LlamaBatch);
}

#[repr(C)]
pub struct LlamaBatch {
    pub n_tokens: c_int,
    pub token: *mut LlamaToken,
    pub embd: *mut c_float,
    pub pos: *mut c_int,
    pub n_seq_id: *mut c_int,
    pub seq_id: *mut *mut c_int,
    pub logits: *mut i8,
    pub all_pos_0: c_int,
    pub all_pos_1: c_int,
    pub all_seq_id: c_int,
}
