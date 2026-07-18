//! remote-access-support CLI: creates a remote session, grants a security
//! policy, opens a control channel, runs a command, then tears the
//! session down.

use remote_access_support::{
    ChannelManager, ChannelType, CommandExecutor, SecurityManager, SecurityPolicy, SessionManager,
};

fn main() {
    let sessions = SessionManager::new();
    let session_id = sessions.create_session("alice".to_string(), "laptop-01".to_string());
    println!("Created session {} ({} active)", session_id, sessions.session_count());

    let security = SecurityManager::new();
    security.set_policy(
        "alice".to_string(),
        SecurityPolicy {
            user_id: "alice".to_string(),
            can_read_files: true,
            can_execute_commands: true,
            can_access_system: false,
        },
    );
    println!(
        "alice can execute commands: {}",
        security.check_permission("alice", "execute_commands")
    );

    let channels = ChannelManager::new();
    let channel_id = channels.create_channel(session_id.clone(), ChannelType::Control, 1_000_000);
    println!("Opened channel {} ({} active)", channel_id, channels.channel_count());

    let commands = CommandExecutor::new();
    let command_id = commands.execute_command(
        session_id.clone(),
        "file_list".to_string(),
        vec!["/home/alice".to_string()],
    );
    println!("Ran command {} ({} total)", command_id, commands.command_count());

    channels.close_channel(&channel_id);
    sessions.disconnect_session(&session_id);
    println!("Session {} disconnected", session_id);
}
