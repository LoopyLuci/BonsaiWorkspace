//! CLI — create a version, add an example, export it to JSONL, and print
//! summary counts, exercising the real `TrainingDataLibrary` API.

use tdl::{ExportFormat, Metadata, TrainingDataLibrary};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = std::env::temp_dir().join("tdl_cli_demo");
    std::fs::create_dir_all(&tmp_dir)?;
    let db_path = tmp_dir.join("tdl_demo.sqlite3");
    // Start fresh each run.
    let _ = std::fs::remove_file(&db_path);

    let library = TrainingDataLibrary::new(&db_path).await?;

    let version_id = library
        .create_version(
            "0.1.0",
            "tdl_cli",
            "Initial demo version",
            vec!["demo".to_string()],
        )
        .await?;
    println!("Created version {version_id}");

    let metadata = Metadata::new()
        .with_source("cli-demo")
        .with_domain("ml")
        .with_tag("demo");

    let example_id = library
        .add_example(
            version_id,
            "The quick brown fox jumps over the lazy dog.".to_string(),
            metadata,
            0.92,
        )
        .await?;
    println!("Added example {example_id}");

    let history = library.get_version_history().await?;
    println!("Version history: {} version(s)", history.len());

    let jsonl_path = tmp_dir.join("export.jsonl");
    let exported = library
        .export_dataset(version_id, ExportFormat::Jsonl, &jsonl_path)
        .await?;
    println!("Exported JSONL to {}", exported.display());

    let examples = library.search_by_quality(0.5, 10).await?;
    println!("Examples with quality >= 0.5: {}", examples.len());

    Ok(())
}
