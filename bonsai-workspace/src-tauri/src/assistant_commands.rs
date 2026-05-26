use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use std::sync::atomic::Ordering;

use crate::AppState;
use crate::assistant_store::{AssistantProfile, AvatarAsset, AssistantSession, AssistantMessage};

// ── Profile commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_assistant_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<AssistantProfile>, String> {
    state.assistant_store.list_profiles().await
}

#[tauri::command]
pub async fn get_active_assistant_profile(
    state: State<'_, AppState>,
) -> Result<Option<AssistantProfile>, String> {
    state.assistant_store.get_active_profile().await
}

#[tauri::command]
pub async fn upsert_assistant_profile(
    state: State<'_, AppState>,
    profile: AssistantProfile,
) -> Result<AssistantProfile, String> {
    state.assistant_store.upsert_profile(profile).await
}

#[tauri::command]
pub async fn delete_assistant_profile(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.assistant_store.delete_profile(&id).await
}

#[tauri::command]
pub async fn set_active_assistant_profile(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.assistant_store.set_active_profile(&id).await
}

// ── Avatar commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_avatar_assets(
    state: State<'_, AppState>,
) -> Result<Vec<AvatarAsset>, String> {
    state.assistant_store.list_avatars().await
}

#[tauri::command]
pub async fn upsert_avatar_asset(
    state: State<'_, AppState>,
    avatar: AvatarAsset,
) -> Result<AvatarAsset, String> {
    state.assistant_store.upsert_avatar(avatar).await
}

#[tauri::command]
pub async fn delete_avatar_asset(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.assistant_store.delete_avatar(&id).await
}

// ── Session commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_assistant_sessions(
    state: State<'_, AppState>,
    profile_id: Option<String>,
) -> Result<Vec<AssistantSession>, String> {
    state.assistant_store.list_sessions(profile_id.as_deref()).await
}

#[tauri::command]
pub async fn create_assistant_session(
    state: State<'_, AppState>,
    profile_id: Option<String>,
    title: String,
) -> Result<AssistantSession, String> {
    state
        .assistant_store
        .create_session(profile_id.as_deref(), &title)
        .await
}

#[tauri::command]
pub async fn load_assistant_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<AssistantMessage>, String> {
    state.assistant_store.load_messages(&session_id).await
}

#[tauri::command]
pub async fn delete_assistant_session(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.assistant_store.delete_session(&id).await
}

// ── Window control ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn toggle_assistant_window(app: AppHandle) -> Result<(), String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = app;
        return Ok(());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let mut config = crate::config::load_config(&app)?;
        let window = match app.get_webview_window("assistant") {
            Some(w) => w,
            None => {
                // If the user fully closed Bonsai Workspace, recreate it so the toolbar button
                // can always reopen the assistant without requiring an app restart.
                WebviewWindowBuilder::new(
                    &app,
                    "assistant",
                    WebviewUrl::App("assistant.html".into()),
                )
                .title("Bonsai Workspace")
                .inner_size(420.0, 680.0)
                .min_inner_size(340.0, 480.0)
                .resizable(true)
                .visible(false)
                .decorations(true)
                .always_on_top(false)
                .build()
                .map_err(|e| e.to_string())?
            }
        };

        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
            config.assistant_window_open = false;
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
            config.assistant_window_open = true;
        }
        let _ = crate::config::save_config(&app, &config);
    }

    Ok(())
}

#[tauri::command]
pub async fn set_assistant_always_on_top(app: AppHandle, on_top: bool) -> Result<(), String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = (app, on_top);
        return Ok(());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    if let Some(w) = app.get_webview_window("assistant") {
        w.set_always_on_top(on_top).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_android_usb_lab_window(app: AppHandle) -> Result<(), String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = app;
        return Ok(());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let mut config = crate::config::load_config(&app)?;
        let window = match app.get_webview_window("android-usb-lab") {
            Some(w) => w,
            None => {
                WebviewWindowBuilder::new(
                    &app,
                    "android-usb-lab",
                    WebviewUrl::App("android-usb-lab.html".into()),
                )
                .title("Android USB Lab")
                .inner_size(880.0, 640.0)
                .min_inner_size(520.0, 360.0)
                .resizable(true)
                .visible(false)
                .decorations(true)
                .build()
                .map_err(|e| e.to_string())?
            }
        };

        if window.is_visible().unwrap_or(false) {
            window.hide().map_err(|e| e.to_string())?;
            config.usb_lab_window_open = false;
        } else {
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
            config.usb_lab_window_open = true;
        }
        let _ = crate::config::save_config(&app, &config);
    }

    Ok(())
}

// ── Secrets commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn set_smtp_credentials(
    state: State<'_, AppState>,
    host: String,
    username: String,
    password: String,
    from_addr: String,
) -> Result<(), String> {
    use crate::secrets_store::{ACCOUNT_SMTP_HOST, ACCOUNT_SMTP_USERNAME, ACCOUNT_SMTP_PASSWORD, ACCOUNT_SMTP_FROM};
    state.secrets_store.store(ACCOUNT_SMTP_HOST, &host)?;
    state.secrets_store.store(ACCOUNT_SMTP_USERNAME, &username)?;
    state.secrets_store.store(ACCOUNT_SMTP_PASSWORD, &password)?;
    state.secrets_store.store(ACCOUNT_SMTP_FROM, &from_addr)?;
    Ok(())
}

#[tauri::command]
pub async fn has_smtp_credentials(state: State<'_, AppState>) -> Result<bool, String> {
    use crate::secrets_store::ACCOUNT_SMTP_PASSWORD;
    Ok(state.secrets_store.has(ACCOUNT_SMTP_PASSWORD))
}

#[tauri::command]
pub async fn clear_smtp_credentials(state: State<'_, AppState>) -> Result<(), String> {
    use crate::secrets_store::{ACCOUNT_SMTP_HOST, ACCOUNT_SMTP_USERNAME, ACCOUNT_SMTP_PASSWORD, ACCOUNT_SMTP_FROM};
    state.secrets_store.delete(ACCOUNT_SMTP_HOST)?;
    state.secrets_store.delete(ACCOUNT_SMTP_USERNAME)?;
    state.secrets_store.delete(ACCOUNT_SMTP_PASSWORD)?;
    state.secrets_store.delete(ACCOUNT_SMTP_FROM)?;
    Ok(())
}

// ── submit_assistant_chat — routes through ReAct loop ────────────────────────

#[tauri::command]
pub async fn submit_assistant_chat(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    user_message: String,
) -> Result<String, String> {
    // ── Slash-command fast path (chess/go/puzzle) ─────────────────────────────
    if let Some(cmd) = crate::games::parse_slash_command(&user_message) {
        // Persist user message first
        let user_msg = AssistantMessage {
            id: String::new(), session_id: session_id.clone(), role: "user".into(),
            content: user_message.clone(), tool_name: None, tool_result: None,
            tts_synthesized: false, created_at: 0, tool_call_id: None, game_state: None,
        };
        state.assistant_store.append_message(user_msg).await?;

        let profile = state.assistant_store.get_active_profile().await?
            .ok_or("No active assistant profile")?;
        let player_name = profile.name.clone();

        let (reply, game_state) = crate::games::execute_slash_command(
            cmd, &state.game_sessions, &player_name, None, None,
        ).await;

        let asst_msg = AssistantMessage {
            id: String::new(), session_id: session_id.clone(), role: "assistant".into(),
            content: reply.clone(), tool_name: None, tool_result: None,
            tts_synthesized: false, created_at: 0, tool_call_id: None,
            game_state,
        };
        state.assistant_store.append_message(asst_msg).await?;

        // Emit game state event so frontend can update
        if let Err(e) = app.emit("game-state-update", &reply) {
            tracing::warn!("game-state-update emit failed: {e}");
        }
        return Ok(reply);
    }
    // ── Normal ReAct path ────────────────────────────────────────────────────

    // Persist user message
    let user_msg = AssistantMessage {
        id: String::new(),
        session_id: session_id.clone(),
        role: "user".into(),
        content: user_message.clone(),
        tool_name: None,
        tool_result: None,
        tts_synthesized: false,
        created_at: 0,
        tool_call_id: None,
        game_state: None,
    };
    state.assistant_store.append_message(user_msg).await?;

    let profile = state.assistant_store.get_active_profile().await?
        .ok_or("No active assistant profile")?;

    // Build history from persisted messages + new user turn
    const CONTEXT_LIMIT: usize = 30;
    let prior_all = state.assistant_store.load_messages(&session_id).await?;

    // Pin system message, then keep last CONTEXT_LIMIT messages.
    // Insert a context-gap notice when truncation occurs.
    let (prior, truncated) = if prior_all.len() > CONTEXT_LIMIT {
        (&prior_all[prior_all.len() - CONTEXT_LIMIT..], true)
    } else {
        (prior_all.as_slice(), false)
    };

    let mut history: Vec<serde_json::Value> = vec![
        serde_json::json!({ "role": "system", "content": profile.system_prompt }),
    ];
    if truncated {
        history.push(serde_json::json!({
            "role": "user",
            "content": "[Note: earlier messages were trimmed for context. The conversation continues below.]",
        }));
        history.push(serde_json::json!({
            "role": "assistant",
            "content": "Understood.",
        }));
    }
    for (i, m) in prior.iter().enumerate() {
        match m.role.as_str() {
            "user" | "assistant" => {
                history.push(serde_json::json!({ "role": m.role, "content": m.content }));
            }
            "tool" => {
                // Prefer stored tool_call_id; fall back to synthetic stable ID for old rows.
                let call_id = m.tool_call_id.clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("call_{i}"));
                history.push(serde_json::json!({
                    "role": "tool",
                    "content": m.content,
                    "tool_call_id": call_id,
                }));
            }
            _ => {}
        }
    }

    let cancel = state.assistant_cancel.clone();
    cancel.store(false, Ordering::SeqCst);

    let turn = crate::assistant_manager::run_assistant_turn(
        history,
        &profile,
        &state.assistant_store,
        &state.policy_engine,
        &state.confirmation_gate,
        &state.orchestrator,
        &state.secrets_store,
        &state.audit_log,
        &app,
        cancel,
        None,
        &session_id,
    ).await?;

    // Persist final assistant reply
    let asst_msg = AssistantMessage {
        id: String::new(),
        session_id: session_id.clone(),
        role: "assistant".into(),
        content: turn.reply.clone(),
        tool_name: None,
        tool_result: None,
        tts_synthesized: false,
        created_at: 0,
        tool_call_id: None,
        game_state: None,
    };
    state.assistant_store.append_message(asst_msg).await?;

    Ok(turn.reply)
}

// ── stop_assistant_chat ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn stop_assistant_chat(state: State<'_, AppState>) -> Result<(), String> {
    state.assistant_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

// ── Confirm tool action (Phase 2 gate, stub) ──────────────────────────────────

#[tauri::command]
pub async fn confirm_tool_action(
    state: State<'_, AppState>,
    token: String,
) -> Result<String, String> {
    let (tool, _args) = state.confirmation_gate.consume(&token)?;
    state.audit_log.log_decision(&tool, "confirmed", "{}", None, None);
    Ok(format!("Confirmed: {tool}"))
}

#[tauri::command]
pub async fn cancel_tool_action(
    state: State<'_, AppState>,
    token: String,
) -> Result<(), String> {
    state.confirmation_gate.cancel(&token);
    Ok(())
}

// ── TTS commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn speak_text(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> Result<(), String> {
    state.tts_manager.speak(&app, &text).await
}

#[tauri::command]
pub async fn stop_tts(state: State<'_, AppState>) -> Result<(), String> {
    state.tts_manager.stop();
    Ok(())
}

#[tauri::command]
pub async fn set_tts_voice(
    state: State<'_, AppState>,
    voice: String,
) -> Result<(), String> {
    state.tts_manager.set_voice(&voice);
    Ok(())
}

#[tauri::command]
pub async fn set_tts_speed(
    state: State<'_, AppState>,
    speed: f32,
) -> Result<(), String> {
    state.tts_manager.set_speed(speed);
    Ok(())
}

#[tauri::command]
pub async fn is_tts_available(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.tts_manager.is_available())
}

// ── Avatar validation ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn validate_avatar_svg(svg: String) -> Result<crate::avatar_validator::AvatarRigReport, String> {
    let cleaned = crate::avatar_validator::sanitize_svg(&svg)?;
    crate::avatar_validator::validate_rig(&cleaned)
}

// ── Metrics & health ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_assistant_metrics(
    state: State<'_, AppState>,
) -> Result<crate::assistant_metrics::MetricsSnapshot, String> {
    Ok(state.asst_metrics.snapshot())
}

#[tauri::command]
pub async fn get_assistant_health(
    _state: State<'_, AppState>,
) -> Result<crate::assistant_metrics::AssistantHealth, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    // Return a lightweight snapshot; the live health is emitted by the watchdog.
    Ok(crate::assistant_metrics::AssistantHealth {
        sidecars:   vec![],
        db_ok:      true,
        last_error: None,
        checked_at: ts,
    })
}

// ── Audit log tail ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_assistant_audit_log(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let path = state.audit_log.log_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let lines: Vec<String> = content.lines().rev().take(200).map(String::from).collect();
            Ok(lines)
        }
        Err(_) => Ok(vec![]),
    }
}

// ── Backup commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn export_assistant_backup(
    app:              AppHandle,
    state:            State<'_, AppState>,
    include_sessions: bool,
    include_avatars:  bool,
    encrypt:          bool,
    passphrase:       Option<String>,
) -> Result<String, String> {
    crate::assistant_backup::export_backup(
        &app,
        &state.assistant_store,
        include_sessions,
        include_avatars,
        encrypt,
        passphrase.as_deref(),
    ).await
}

#[tauri::command]
pub async fn import_assistant_backup(
    app:        AppHandle,
    state:      State<'_, AppState>,
    zip_path:   String,
    mode:       crate::assistant_backup::ImportMode,
    passphrase: Option<String>,
    dry_run:    bool,
) -> Result<crate::assistant_backup::ImportSummary, String> {
    crate::assistant_backup::import_backup(
        &app,
        &state.assistant_store,
        &zip_path,
        mode,
        passphrase.as_deref(),
        dry_run,
    ).await
}

#[tauri::command]
pub async fn list_assistant_backups(
    state: State<'_, AppState>,
) -> Result<Vec<crate::assistant_backup::BackupEntry>, String> {
    crate::assistant_backup::list_backups(&state.assistant_store).await
}

#[tauri::command]
pub async fn verify_backup_integrity(
    zip_path:   String,
    passphrase: Option<String>,
) -> Result<bool, String> {
    crate::assistant_backup::verify_backup(&zip_path, passphrase.as_deref()).await
}

#[tauri::command]
pub async fn delete_assistant_backup_entry(
    state: State<'_, AppState>,
    id:    String,
) -> Result<(), String> {
    state.assistant_store.delete_backup_entry(&id).await
}

// ── Session auto-title ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn auto_title_session(
    app_handle: tauri::AppHandle,
    state:      State<'_, AppState>,
    session_id: String,
    user_msg:   String,
    reply_msg:  String,
) -> Result<String, String> {
    use tauri::Emitter;
    let profiles = state.assistant_store.list_profiles().await.unwrap_or_default();
    let base_profile = profiles.into_iter().find(|p| p.is_active).unwrap_or_else(|| {
        crate::assistant_store::AssistantProfile {
            id: "auto-title".to_string(), name: "Bonsai Buddy".to_string(),
            persona_id: None, avatar_id: None,
            tts_voice: "en-us".to_string(), tts_speed: 1.0, tts_pitch: 1.0,
            tts_enabled: false, wake_word: None, tool_permissions: "{}".to_string(),
            system_prompt: String::new(), model_id: None, is_active: true,
            created_at: 0, updated_at: 0,
        }
    });

    let title_profile = crate::assistant_store::AssistantProfile {
        system_prompt: "Summarize this exchange as a 4-6 word session title. Return ONLY the title, no punctuation, no quotes.".to_string(),
        ..base_profile
    };

    let history = vec![
        serde_json::json!({ "role": "user",      "content": &user_msg  }),
        serde_json::json!({ "role": "assistant",  "content": &reply_msg }),
        serde_json::json!({ "role": "user",      "content": "Generate the session title now." }),
    ];

    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let turn = crate::assistant_manager::run_assistant_turn(
        history, &title_profile,
        &state.assistant_store, &state.policy_engine, &state.confirmation_gate,
        &state.orchestrator, &state.secrets_store, &state.audit_log,
        &app_handle, cancel, None, &format!("auto-title-{session_id}"),
    ).await?;

    let title = turn.reply.trim()
        .trim_matches(|c: char| !c.is_alphanumeric() && c != ' ')
        .to_string();
    let title = if title.is_empty() { "New conversation".to_string() } else { title };

    state.assistant_store.set_session_title(&session_id, &title).await?;
    let _ = app_handle.emit("assistant-session-titled", serde_json::json!({
        "session_id": session_id,
        "title": title,
    }));
    Ok(title)
}

// ── User-defined skill commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn list_user_skills(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = state.user_skill_store.list_all().await?;
    let out = rows
        .into_iter()
        .map(|r| serde_json::to_value(&r).unwrap_or_default())
        .collect();
    Ok(out)
}

#[tauri::command]
pub async fn upsert_user_skill(
    state: State<'_, AppState>,
    skill: serde_json::Value,
) -> Result<(), String> {
    let row: crate::user_skills::UserSkillRow =
        serde_json::from_value(skill).map_err(|e| format!("invalid skill payload: {e}"))?;
    state.user_skill_store.upsert(&row).await?;
    crate::assistant_manager::reload_user_skills(&state.user_skill_store).await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_user_skill(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.user_skill_store.delete(&id).await?;
    crate::assistant_manager::reload_user_skills(&state.user_skill_store).await?;
    Ok(())
}

#[tauri::command]
pub async fn test_user_skill(
    _state: State<'_, AppState>,
    body: String,
) -> Result<serde_json::Value, String> {
    use std::time::Duration;

    let child_fut = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&body)
        .kill_on_drop(true)
        .output();

    match tokio::time::timeout(Duration::from_secs(10), child_fut).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            Ok(serde_json::json!({ "stdout": stdout, "stderr": stderr, "exit_code": exit_code }))
        }
        Ok(Err(e)) => Err(format!("spawn error: {e}")),
        Err(_) => Err("test timed out after 10 seconds".to_string()),
    }
}

// ── MCP server commands ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_mcp_servers(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let configs = state.assistant_store.list_mcp_servers().await?;
    Ok(configs.iter().map(|c| serde_json::json!({
        "id":        c.id,
        "name":      c.name,
        "command":   c.command,
        "args":      c.args,
        "namespace": c.namespace,
        "enabled":   c.enabled,
    })).collect())
}

#[tauri::command]
pub async fn upsert_mcp_server(
    state: State<'_, AppState>,
    config: serde_json::Value,
) -> Result<(), String> {
    use crate::mcp_bridge::McpServerConfig;
    let cfg = McpServerConfig {
        id:        config["id"].as_str().unwrap_or("").to_string(),
        name:      config["name"].as_str().unwrap_or("").to_string(),
        command:   config["command"].as_str().unwrap_or("").to_string(),
        args:      config["args"].as_array()
                       .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                       .unwrap_or_default(),
        namespace: config["namespace"].as_str().unwrap_or("").to_string(),
        enabled:   config["enabled"].as_bool().unwrap_or(true),
    };
    state.assistant_store.upsert_mcp_server(&cfg).await
}

#[tauri::command]
pub async fn delete_mcp_server(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.assistant_store.delete_mcp_server(&id).await
}

#[tauri::command]
pub async fn reconnect_mcp_servers(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<String>, String> {
    let configs = state.assistant_store.list_mcp_servers().await?;
    let allowed_commands = crate::config::load_config(&app)
        .map(|c| c.mcp_allowed_commands)
        .unwrap_or_default();
    state.mcp_manager.load_configs(configs).await;
    let registry = crate::assistant_manager::assistant_registry();
    let mut reg = registry.write().await;
    let connected = state.mcp_manager.connect_all_into_registry(&mut *reg, &allowed_commands).await;
    Ok(connected)
}
