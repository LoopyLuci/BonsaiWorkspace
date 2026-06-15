//! Model format converters

pub mod gguf_to_bkp;
pub mod bkp_to_gguf;
pub mod gguf_to_safetensors;
pub mod safetensors_to_gguf;
pub mod bkp_to_safetensors;
pub mod safetensors_to_bkp;
pub mod huggingface_to_bkp;
pub mod bkp_to_huggingface;
pub mod batch;

pub use gguf_to_bkp::convert_gguf_to_bkp;
pub use bkp_to_gguf::convert_bkp_to_gguf;
pub use gguf_to_safetensors::convert_gguf_to_safetensors;
pub use safetensors_to_gguf::convert_safetensors_to_gguf;
pub use bkp_to_safetensors::convert_bkp_to_safetensors;
pub use safetensors_to_bkp::convert_safetensors_to_bkp;
pub use huggingface_to_bkp::convert_huggingface_to_bkp;
pub use bkp_to_huggingface::convert_bkp_to_huggingface;
pub use batch::convert_batch;

use crate::error::{ConverterError, ConverterResult};
use crate::format::ModelFormat;
use crate::ConversionConfig;
use std::path::Path;

/// Universal converter dispatcher — routes to appropriate converter based on format pair
pub async fn convert(
    from_format: ModelFormat,
    to_format: ModelFormat,
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    config: ConversionConfig,
) -> ConverterResult<()> {
    let (from, to) = (from_format, to_format);

    match (from, to) {
        (ModelFormat::Gguf, ModelFormat::Bkp) => {
            convert_gguf_to_bkp(input_path, output_path, config).await
        }
        (ModelFormat::Bkp, ModelFormat::Gguf) => {
            convert_bkp_to_gguf(input_path, output_path, config).await
        }
        (ModelFormat::Gguf, ModelFormat::Safetensors) => {
            convert_gguf_to_safetensors(input_path, output_path, config).await
        }
        (ModelFormat::Safetensors, ModelFormat::Gguf) => {
            convert_safetensors_to_gguf(input_path, output_path, config).await
        }
        (ModelFormat::Bkp, ModelFormat::Safetensors) => {
            convert_bkp_to_safetensors(input_path, output_path, config).await
        }
        (ModelFormat::Safetensors, ModelFormat::Bkp) => {
            convert_safetensors_to_bkp(input_path, output_path, config).await
        }
        (ModelFormat::HuggingFace, ModelFormat::Bkp) => {
            let model_id = input_path.as_ref().to_string_lossy().to_string();
            convert_huggingface_to_bkp(&model_id, output_path, config).await
        }
        (ModelFormat::Bkp, ModelFormat::HuggingFace) => {
            let repo_id = output_path.as_ref().to_string_lossy().to_string();
            convert_bkp_to_huggingface(input_path, &repo_id, None, config).await
        }
        _ => Err(ConverterError::ConversionNotSupported {
            from: from.to_string(),
            to: to.to_string(),
        }),
    }
}
