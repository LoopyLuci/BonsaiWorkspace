use msg_core::{SmtpConfig, ImapConfig, Message};
use msg_smtp::SmtpServer;
use msg_imap::ImapServer;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Shared message store
    let messages: Arc<Mutex<Vec<Message>>> = Arc::new(Mutex::new(Vec::new()));

    // SMTP server
    let smtp = SmtpServer::new(SmtpConfig::default());
    let smtp_handle = tokio::spawn(async move { smtp.serve().await });

    // IMAP server
    let imap = ImapServer::new(ImapConfig::default(), messages);
    let imap_handle = tokio::spawn(async move { imap.serve().await });

    tracing::info!("Bonsai Messaging Fabric is running");
    tokio::select! {
        r = smtp_handle => r??,
        r = imap_handle => r??,
    }

    Ok(())
}
