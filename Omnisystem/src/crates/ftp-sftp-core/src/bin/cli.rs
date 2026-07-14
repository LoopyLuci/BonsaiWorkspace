//! CLI demo: create an SFTP session and look it up through the session manager.

use ftp_sftp_core::{DefaultSessionManager, Protocol, SessionManager, UserId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sessions = DefaultSessionManager::new();
    let user = UserId("alice".to_string());

    let session_id = sessions
        .create_session(&user, Protocol::Sftp, "10.0.0.5:2222".to_string())
        .await?;
    println!("Created session: {}", session_id.0);

    let session = sessions.get_session(&session_id).await?;
    println!("Session status: {:?}", session.status);
    println!("Active sessions: {}", sessions.session_count());

    Ok(())
}
