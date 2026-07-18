//! extension-converter CLI.
//!
//! Usage:
//!   extension_converter_cli <path-to-vsix> [output-dir]
//!
//! Imports a VSCode `.vsix` extension into the Extension IR, prints a summary,
//! then exports it as a standalone MCP server into `output-dir` (or a fresh
//! temp directory if omitted).

use std::path::PathBuf;

use extension_converter::export::mcp::export_as_mcp;
use extension_converter::import::vscode::import_vsix;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);

    let Some(vsix_path) = args.next().map(PathBuf::from) else {
        eprintln!("usage: extension_converter_cli <path-to-vsix> [output-dir]");
        std::process::exit(2);
    };

    let ir = import_vsix(&vsix_path).await?;
    let summary = ir.capability_summary();

    println!("Extension: {} v{}", ir.metadata.id, ir.metadata.version);
    println!(
        "Capabilities: {} commands, {} language, {} views, {} tools, {} themes, {} snippets, {} keybindings, {} custom",
        summary.commands,
        summary.language_support,
        summary.views,
        summary.tools,
        summary.themes,
        summary.snippets,
        summary.keybindings,
        summary.custom,
    );
    for w in ir.warnings() {
        println!("warning [{}]: {}", w.capability_id, w.message);
    }

    let out_dir = match args.next() {
        Some(p) => PathBuf::from(p),
        None => tempfile::tempdir()?.keep(),
    };

    let result = export_as_mcp(&ir, &out_dir).await?;
    println!(
        "Exported MCP server ({} tools) to {}",
        result.tool_count,
        result.output_dir.display()
    );

    Ok(())
}
