use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::RwLock;

// ── State ─────────────────────────────────────────────────────────────────────

pub struct CollaborationState {
    pub active_sessions: Arc<RwLock<HashMap<String, SessionData>>>,
    pub chat_messages: Arc<RwLock<HashMap<String, Vec<ChatMessage>>>>,
}

impl CollaborationState {
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            chat_messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub invitation_code: String,
    pub host_name: String,
    pub participants: Vec<ParticipantSummary>,
    pub is_active: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantSummary {
    pub peer_id: String,
    pub display_name: String,
    pub is_online: bool,
    pub is_speaking: bool,
    pub has_video: bool,
    pub current_file: Option<String>,
    pub cursor_position: Option<CursorPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub text: String,
    pub timestamp: u64,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSession {
    pub session_id: String,
    pub invitation_code: String,
    pub participants: Vec<ParticipantSummary>,
    pub is_active: bool,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn generate_invitation_code() -> String {
    const WORDS: &[&str] = &[
        "bonsai", "tree", "leaf", "root", "branch", "seed", "grow", "green", "fox", "owl", "bear",
        "wolf", "hawk", "deer", "fish", "bird", "star", "moon", "sun", "wind", "rain", "snow",
        "fire", "stone",
    ];
    let a = WORDS[rand::random::<usize>() % WORDS.len()];
    let b = WORDS[rand::random::<usize>() % WORDS.len()];
    let n = rand::random::<u16>() % 10000;
    format!("{a}-{b}-{n:04}")
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_collaboration_session(
    state: State<'_, CollaborationState>,
    app_handle: tauri::AppHandle,
    host_name: String,
    allow_edit: bool,
    allow_voice: bool,
    allow_video: bool,
) -> Result<CollaborationSession, String> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let invitation_code = generate_invitation_code();

    let host = ParticipantSummary {
        peer_id: format!("host-{}", &session_id[..8]),
        display_name: host_name.clone(),
        is_online: true,
        is_speaking: false,
        has_video: allow_video,
        current_file: None,
        cursor_position: None,
    };

    let data = SessionData {
        session_id: session_id.clone(),
        invitation_code: invitation_code.clone(),
        host_name,
        participants: vec![host.clone()],
        is_active: true,
        created_at: now_ms(),
    };

    state
        .active_sessions
        .write()
        .await
        .insert(session_id.clone(), data);

    let _ = app_handle.emit(
        "collab-session-created",
        serde_json::json!({
            "session_id": &session_id,
            "invitation_code": &invitation_code,
        }),
    );

    tracing::info!("Collaboration session created: {session_id} code={invitation_code}");

    Ok(CollaborationSession {
        session_id,
        invitation_code,
        participants: vec![host],
        is_active: true,
    })
}

#[tauri::command]
pub async fn join_collaboration_session(
    state: State<'_, CollaborationState>,
    app_handle: tauri::AppHandle,
    invitation_code: String,
    display_name: String,
) -> Result<CollaborationSession, String> {
    let mut sessions = state.active_sessions.write().await;
    let session = sessions
        .values_mut()
        .find(|s| s.invitation_code == invitation_code && s.is_active)
        .ok_or_else(|| "Invalid or expired invitation code".to_string())?;

    let participant = ParticipantSummary {
        peer_id: uuid::Uuid::new_v4().to_string(),
        display_name: display_name.clone(),
        is_online: true,
        is_speaking: false,
        has_video: false,
        current_file: None,
        cursor_position: None,
    };

    session.participants.push(participant.clone());

    let _ = app_handle.emit("collab-participant-joined", serde_json::json!(&participant));

    Ok(CollaborationSession {
        session_id: session.session_id.clone(),
        invitation_code: session.invitation_code.clone(),
        participants: session.participants.clone(),
        is_active: true,
    })
}

#[tauri::command]
pub async fn leave_collaboration_session(
    state: State<'_, CollaborationState>,
    app_handle: tauri::AppHandle,
    session_id: String,
    peer_id: String,
) -> Result<(), String> {
    let mut sessions = state.active_sessions.write().await;
    if let Some(session) = sessions.get_mut(&session_id) {
        session.participants.retain(|p| p.peer_id != peer_id);
        let _ = app_handle.emit(
            "collab-participant-left",
            serde_json::json!({ "peer_id": &peer_id }),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn get_collaboration_session(
    state: State<'_, CollaborationState>,
    session_id: String,
) -> Result<Option<CollaborationSession>, String> {
    let sessions = state.active_sessions.read().await;
    Ok(sessions.get(&session_id).map(|s| CollaborationSession {
        session_id: s.session_id.clone(),
        invitation_code: s.invitation_code.clone(),
        participants: s.participants.clone(),
        is_active: s.is_active,
    }))
}

#[tauri::command]
pub async fn list_collaboration_sessions(
    state: State<'_, CollaborationState>,
) -> Result<Vec<CollaborationSession>, String> {
    let sessions = state.active_sessions.read().await;
    Ok(sessions
        .values()
        .map(|s| CollaborationSession {
            session_id: s.session_id.clone(),
            invitation_code: s.invitation_code.clone(),
            participants: s.participants.clone(),
            is_active: s.is_active,
        })
        .collect())
}

#[tauri::command]
pub async fn send_chat_message(
    state: State<'_, CollaborationState>,
    app_handle: tauri::AppHandle,
    session_id: String,
    text: String,
    sender_id: String,
    sender_name: String,
    parent_id: Option<String>,
) -> Result<String, String> {
    let msg = ChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        sender_id,
        sender_name,
        text,
        timestamp: now_ms(),
        parent_id,
    };

    state
        .chat_messages
        .write()
        .await
        .entry(session_id.clone())
        .or_default()
        .push(msg.clone());

    let _ = app_handle.emit("collab-chat-message", serde_json::json!(&msg));

    Ok(msg.id)
}

#[tauri::command]
pub async fn get_chat_history(
    state: State<'_, CollaborationState>,
    session_id: String,
    limit: Option<usize>,
) -> Result<Vec<ChatMessage>, String> {
    let messages = state.chat_messages.read().await;
    let all = messages.get(&session_id).cloned().unwrap_or_default();
    let limit = limit.unwrap_or(100);
    Ok(all.into_iter().rev().take(limit).rev().collect())
}

#[tauri::command]
pub async fn send_cursor_position(
    app_handle: tauri::AppHandle,
    session_id: String,
    peer_id: String,
    file: String,
    line: u32,
    column: u32,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-cursor-update",
        serde_json::json!({
            "session_id": session_id,
            "peer_id": peer_id,
            "file": file,
            "line": line,
            "column": column,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn update_participant_file(
    app_handle: tauri::AppHandle,
    state: State<'_, CollaborationState>,
    session_id: String,
    peer_id: String,
    file: Option<String>,
) -> Result<(), String> {
    let mut sessions = state.active_sessions.write().await;
    if let Some(session) = sessions.get_mut(&session_id) {
        if let Some(p) = session
            .participants
            .iter_mut()
            .find(|p| p.peer_id == peer_id)
        {
            p.current_file = file.clone();
        }
    }
    let _ = app_handle.emit(
        "collab-participant-file",
        serde_json::json!({
            "peer_id": peer_id,
            "file": file,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn start_voice_call(
    app_handle: tauri::AppHandle,
    session_id: String,
    initiator_id: String,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-call-started",
        serde_json::json!({
            "session_id": session_id,
            "initiator_id": initiator_id,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn end_voice_call(
    app_handle: tauri::AppHandle,
    session_id: String,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-call-ended",
        serde_json::json!({
            "session_id": session_id,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn send_webrtc_signal(
    app_handle: tauri::AppHandle,
    session_id: String,
    from_peer_id: String,
    to_peer_id: String,
    signal_type: String,
    payload: String,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-webrtc-signal",
        serde_json::json!({
            "session_id": session_id,
            "from": from_peer_id,
            "to": to_peer_id,
            "type": signal_type,
            "payload": payload,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn close_collaboration_session(
    state: State<'_, CollaborationState>,
    app_handle: tauri::AppHandle,
    session_id: String,
) -> Result<(), String> {
    let mut sessions = state.active_sessions.write().await;
    if let Some(session) = sessions.get_mut(&session_id) {
        session.is_active = false;
    }
    let _ = app_handle.emit(
        "collab-session-closed",
        serde_json::json!({ "session_id": session_id }),
    );
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrdtOperation {
    pub file: String,
    pub op_type: String,
    pub position: usize,
    pub text: Option<String>,
    pub length: Option<usize>,
    pub peer_id: String,
    pub timestamp: u64,
}

#[tauri::command]
pub async fn send_crdt_operation(
    app_handle: tauri::AppHandle,
    session_id: String,
    operation: CrdtOperation,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-crdt-operation",
        serde_json::json!({
            "session_id": session_id,
            "operation": operation,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn add_chat_reaction(
    state: State<'_, CollaborationState>,
    app_handle: tauri::AppHandle,
    session_id: String,
    message_id: String,
    emoji: String,
    peer_id: String,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-reaction-added",
        serde_json::json!({
            "session_id": session_id,
            "message_id": message_id,
            "emoji": emoji,
            "peer_id": peer_id,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn set_media_mute(
    app_handle: tauri::AppHandle,
    session_id: String,
    track: String,
    muted: bool,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-media-mute",
        serde_json::json!({
            "session_id": session_id,
            "track": track,
            "muted": muted,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn toggle_screen_share(
    app_handle: tauri::AppHandle,
    session_id: String,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-screen-share-toggled",
        serde_json::json!({
            "session_id": session_id,
        }),
    );
    Ok(())
}

#[tauri::command]
pub async fn execute_shared_command(
    app_handle: tauri::AppHandle,
    session_id: String,
    command: String,
) -> Result<(), String> {
    let _ = app_handle.emit(
        "collab-terminal-output",
        serde_json::json!({
            "text": format!("[Command received: {}]", command),
        }),
    );
    Ok(())
}
