use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct AgentConnectSession {
    pub id: String,
    pub goal: Option<String>,
    pub workspace_path: Option<String>,
    pub status: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub last_event_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentConnectEvent {
    pub seq: u64,
    pub session_id: String,
    pub event_type: String,
    pub summary: String,
    pub details: Value,
    pub ts_ms: u64,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    session: AgentConnectSession,
    events: Vec<AgentConnectEvent>,
}

#[derive(Default)]
pub struct AgentConnectHub {
    sessions: HashMap<String, SessionRecord>,
    active_session_id: Option<String>,
    next_seq: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn new_session_id() -> String {
    let rand_tail: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    format!("acs-{}-{}", now_ms(), rand_tail.to_lowercase())
}

impl AgentConnectHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_session(
        &mut self,
        goal: Option<String>,
        workspace_path: Option<String>,
    ) -> AgentConnectSession {
        let ts = now_ms();
        let session = AgentConnectSession {
            id: new_session_id(),
            goal,
            workspace_path,
            status: "active".to_string(),
            created_at_ms: ts,
            updated_at_ms: ts,
            last_event_summary: None,
        };

        let id = session.id.clone();
        self.sessions.insert(
            id.clone(),
            SessionRecord {
                session: session.clone(),
                events: Vec::new(),
            },
        );
        self.active_session_id = Some(id);
        session
    }

    pub fn get_active_session(&self) -> Option<AgentConnectSession> {
        let id = self.active_session_id.as_ref()?;
        self.sessions.get(id).map(|s| s.session.clone())
    }

    pub fn set_active_session(&mut self, session_id: &str) -> Result<AgentConnectSession, String> {
        if !self.sessions.contains_key(session_id) {
            return Err(format!("Session not found: {}", session_id));
        }
        self.active_session_id = Some(session_id.to_string());
        self.sessions
            .get(session_id)
            .map(|s| s.session.clone())
            .ok_or_else(|| "Session not found".to_string())
    }

    pub fn list_sessions(&self) -> Vec<AgentConnectSession> {
        let mut out: Vec<AgentConnectSession> =
            self.sessions.values().map(|r| r.session.clone()).collect();
        out.sort_by_key(|s| std::cmp::Reverse(s.updated_at_ms));
        out
    }

    pub fn append_to_active(
        &mut self,
        event_type: &str,
        summary: &str,
        details: Value,
    ) -> Option<AgentConnectEvent> {
        let active_id = self.active_session_id.clone()?;
        self.append_to_session(&active_id, event_type, summary, details)
            .ok()
    }

    pub fn append_to_session(
        &mut self,
        session_id: &str,
        event_type: &str,
        summary: &str,
        details: Value,
    ) -> Result<AgentConnectEvent, String> {
        let record = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        self.next_seq = self.next_seq.saturating_add(1);
        let event = AgentConnectEvent {
            seq: self.next_seq,
            session_id: session_id.to_string(),
            event_type: event_type.to_string(),
            summary: summary.to_string(),
            details,
            ts_ms: now_ms(),
        };

        record.events.push(event.clone());
        record.session.updated_at_ms = event.ts_ms;
        record.session.last_event_summary = Some(event.summary.clone());

        Ok(event)
    }

    pub fn get_timeline(
        &self,
        session_id: Option<&str>,
        after_seq: Option<u64>,
        limit: Option<usize>,
    ) -> Vec<AgentConnectEvent> {
        let default_limit = 200usize;
        let max_limit = 1000usize;
        let take_n = limit.unwrap_or(default_limit).clamp(1, max_limit);

        let sid = session_id
            .map(|s| s.to_string())
            .or_else(|| self.active_session_id.clone());

        let Some(sid) = sid else {
            return Vec::new();
        };

        let Some(record) = self.sessions.get(&sid) else {
            return Vec::new();
        };

        let after = after_seq.unwrap_or(0);
        let mut events: Vec<AgentConnectEvent> = record
            .events
            .iter()
            .filter(|e| e.seq > after)
            .cloned()
            .collect();

        if events.len() > take_n {
            events = events[events.len() - take_n..].to_vec();
        }

        events
    }

    pub fn end_session(
        &mut self,
        session_id: Option<&str>,
        status: Option<String>,
    ) -> Result<AgentConnectSession, String> {
        let sid = session_id
            .map(|s| s.to_string())
            .or_else(|| self.active_session_id.clone())
            .ok_or_else(|| "No active session".to_string())?;

        let record = self
            .sessions
            .get_mut(&sid)
            .ok_or_else(|| format!("Session not found: {}", sid))?;

        record.session.status = status.unwrap_or_else(|| "completed".to_string());
        record.session.updated_at_ms = now_ms();

        if self.active_session_id.as_deref() == Some(sid.as_str()) {
            self.active_session_id = None;
        }

        Ok(record.session.clone())
    }
}
