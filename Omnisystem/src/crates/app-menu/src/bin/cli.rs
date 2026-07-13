/// Launcher CLI Binary
///
/// Usage:
///   launcher-cli list              - List all apps
///   launcher-cli launch <app_id>   - Launch app
///   launcher-cli search <query>    - Search apps
///   launcher-cli -i                - Interactive mode
///   launcher-cli --help            - Show help

use app_menu::cli::UI;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Run CLI
    UI::render().await?;
    Ok(())
}
