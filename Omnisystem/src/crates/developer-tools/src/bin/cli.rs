//! CLI demo: generate an SDK and look it up.

use developer_tools::{Sdk, SdkGenerator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SdkGenerator::new();

    generator
        .generate(&Sdk {
            sdk_name: "python-sdk".to_string(),
            language: "python".to_string(),
            version: "1.0.0".to_string(),
        })
        .await?;

    let sdk = generator.get_sdk("python-sdk").await?;
    println!("Generated SDK: {} v{} ({})", sdk.sdk_name, sdk.version, sdk.language);
    println!("Total SDKs: {}", generator.sdk_count());

    Ok(())
}
