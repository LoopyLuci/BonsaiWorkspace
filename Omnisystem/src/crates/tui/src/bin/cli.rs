//! tui_cli - non-interactive smoke test for the Bonsai TUI library.
//!
//! Builds the same panel set the real interactive `tui` binary uses,
//! prints their metadata, and attempts (with a short timeout) to reach a
//! daemon, demonstrating the client's offline fallback when none is
//! running. This does not draw to a terminal, so it is safe to run in
//! CI / headless environments.

use tui::client::DaemonClient;
use tui::panel::Panel;
use tui::panels::{
    chat_panel::ChatPanel, collaboration_panel::CollaborationPanel, compute_panel::ComputePanel,
    credits_panel::CreditsPanel, estimate_panel::EstimatePanel, files_panel::FilesPanel,
    health_panel::HealthPanel, logs_panel::LogsPanel, marketplace_panel::MarketplacePanel,
    settings_panel::SettingsPanel, terminal_panel::TerminalPanel, trainer_panel::TrainerPanel,
};
use tui::Theme;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let panels: Vec<Box<dyn Panel>> = vec![
        Box::new(ChatPanel::new()),
        Box::new(FilesPanel::new()),
        Box::new(TrainerPanel::new()),
        Box::new(TerminalPanel::new()),
        Box::new(HealthPanel::new()),
        Box::new(CollaborationPanel::new()),
        Box::new(MarketplacePanel::new()),
        Box::new(CreditsPanel::new()),
        Box::new(EstimatePanel::new()),
        Box::new(ComputePanel::new()),
        Box::new(SettingsPanel::new()),
        Box::new(LogsPanel::new()),
    ];

    println!("Bonsai TUI ({} panels registered):", panels.len());
    for panel in &panels {
        println!("  [{}] {}", panel.icon(), panel.name());
    }

    let theme = Theme::dark();
    println!("Default theme accent color: {:?}", theme.accent);

    // Attempt a short-lived connection to the default daemon address; the
    // client gracefully falls back to offline mode if nothing is listening.
    let connect = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        DaemonClient::connect("127.0.0.1".to_string(), 11370, String::new()),
    )
    .await;

    match connect {
        Ok(Ok((client, _events))) => {
            println!("Daemon connected: {}", client.is_connected());
        }
        Ok(Err(e)) => println!("Daemon client error: {e}"),
        Err(_) => println!("Daemon connect attempt timed out (running offline)"),
    }

    Ok(())
}
