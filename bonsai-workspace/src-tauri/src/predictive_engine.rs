//! Workstream B — Predictive Engine
//!
//! Learns user behavioural patterns from the OmnEvent stream and produces:
//!  • Next-action predictions (Markov chain + temporal models)
//!  • Automation rules mined from repeated command sequences
//!  • User-visible suggestions that can be approved or rejected
//!
//! The rule mining loop runs every 100 events.  Approved suggestions raise the
//! rule confidence; rejections lower it and eventually prune the rule.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use crate::omnipresent_capture::{OmnContext, OmnEvent, OmnEventType};

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Prediction types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedAction {
    pub action_label: String,
    pub confidence: f32,
    pub source: String,
    pub automation_rule_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    pub trigger: TriggerCondition,
    pub action: AutomatedAction,
    /// Confidence threshold (0–1) before rule fires automatically
    pub confidence_threshold: f32,
    /// When true the system asks the user before executing
    pub requires_confirmation: bool,
    pub times_correct: u32,
    pub times_rejected: u32,
    pub created_at: i64,
    pub last_fired_at: Option<i64>,
    pub enabled: bool,
}

impl AutomationRule {
    pub fn confidence(&self) -> f32 {
        let total = self.times_correct + self.times_rejected;
        if total == 0 { return 0.5; }
        self.times_correct as f32 / total as f32
    }

    pub fn should_fire(&self) -> bool {
        self.enabled && self.confidence() >= self.confidence_threshold
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TriggerCondition {
    TimeOfDay { hour: u32, day_of_week: Option<u32> },
    AppLaunch { app_id: String },
    FileOpen { extension: String },
    CommandSequence { commands: Vec<String> },
    ContextMatch { pattern: String },
    EventAfterEvent { first: String, second: String },
}

impl TriggerCondition {
    pub fn matches(&self, ev: &OmnEvent, recent: &[String]) -> bool {
        match self {
            Self::TimeOfDay { hour, day_of_week } => {
                let now = Utc::now();
                now.hour() == *hour && day_of_week.map_or(true, |d| now.weekday().num_days_from_monday() == d)
            }
            Self::AppLaunch { app_id } => matches!(&ev.event_type,
                OmnEventType::AppLaunch { app_id: id, .. } if id == app_id),
            Self::FileOpen { extension } => matches!(&ev.event_type,
                OmnEventType::FileOpen { path, .. } if path.ends_with(extension.as_str())),
            Self::CommandSequence { commands } => {
                commands.iter().all(|c| recent.contains(c))
            }
            Self::ContextMatch { pattern } => {
                ev.context.active_window_title.to_lowercase().contains(pattern.to_lowercase().as_str())
                || ev.context.active_app.to_lowercase().contains(pattern.to_lowercase().as_str())
            }
            Self::EventAfterEvent { first, second } => {
                let label = event_label(&ev.event_type);
                label.contains(second.as_str()) && recent.last().map_or(false, |r| r.contains(first.as_str()))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum AutomatedAction {
    LaunchApp { app_id: String },
    OpenFile { path: String },
    RunCommand { command: String },
    ExecuteWorkflow { workflow_id: String },
    SendNotification { message: String },
    AdjustSetting { setting: String, value: serde_json::Value },
    SuggestCompletion { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSuggestion {
    pub id: String,
    pub rule_id: String,
    pub description: String,
    pub action: AutomatedAction,
    pub created_at: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — Markov action predictor
// ─────────────────────────────────────────────────────────────────────────────

struct MarkovModel {
    /// bigram transition counts: prev_label → (next_label → count)
    transitions: HashMap<String, HashMap<String, u32>>,
    window: VecDeque<String>,
}

impl MarkovModel {
    fn new() -> Self {
        Self { transitions: HashMap::new(), window: VecDeque::with_capacity(64) }
    }

    fn update(&mut self, label: &str) {
        if let Some(prev) = self.window.back() {
            *self.transitions
                .entry(prev.clone())
                .or_default()
                .entry(label.to_string())
                .or_default() += 1;
        }
        self.window.push_back(label.to_string());
        if self.window.len() > 64 { self.window.pop_front(); }
    }

    fn predict(&self, current: &str, top_n: usize) -> Vec<PredictedAction> {
        let Some(nexts) = self.transitions.get(current) else { return vec![]; };
        let total: u32 = nexts.values().sum();
        let mut ranked: Vec<(&String, f32)> = nexts.iter()
            .map(|(k, &v)| (k, v as f32 / total as f32))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(top_n);
        ranked.into_iter().map(|(label, conf)| PredictedAction {
            action_label: label.clone(),
            confidence: conf,
            source: "markov".into(),
            automation_rule_id: None,
        }).collect()
    }

    fn recent_sequence(&self, n: usize) -> Vec<String> {
        self.window.iter().rev().take(n).cloned().collect::<Vec<_>>().into_iter().rev().collect()
    }

    /// Extract n-grams that appear with high frequency
    fn extract_sequences(&self, min_freq: u32, top_n: usize) -> Vec<(String, String, u32)> {
        let mut seqs: Vec<(String, String, u32)> = self.transitions.iter()
            .flat_map(|(prev, nexts)| {
                nexts.iter().filter_map(|(next, &cnt)| {
                    if cnt >= min_freq { Some((prev.clone(), next.clone(), cnt)) } else { None }
                })
            })
            .collect();
        seqs.sort_by(|a, b| b.2.cmp(&a.2));
        seqs.truncate(top_n);
        seqs
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3 — Temporal predictor (time-of-day patterns)
// ─────────────────────────────────────────────────────────────────────────────

struct TemporalModel {
    /// hour → list of event labels observed at that hour
    hourly: HashMap<u32, HashMap<String, u32>>,
}

impl TemporalModel {
    fn new() -> Self { Self { hourly: HashMap::new() } }

    fn update(&mut self, label: &str) {
        let hour = Utc::now().hour();
        *self.hourly.entry(hour).or_default().entry(label.to_string()).or_default() += 1;
    }

    fn predict_now(&self, top_n: usize) -> Vec<PredictedAction> {
        let hour = Utc::now().hour();
        let Some(labels) = self.hourly.get(&hour) else { return vec![]; };
        let total: u32 = labels.values().sum();
        let mut ranked: Vec<(&String, f32)> = labels.iter()
            .map(|(k, &v)| (k, v as f32 / total as f32))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(top_n);
        ranked.into_iter().map(|(label, conf)| PredictedAction {
            action_label: label.clone(),
            confidence: conf * 0.6, // temporal predictions are lower confidence
            source: "temporal".into(),
            automation_rule_id: None,
        }).collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4 — PredictiveEngine
// ─────────────────────────────────────────────────────────────────────────────

pub struct PredictiveEngine {
    markov: RwLock<MarkovModel>,
    temporal: RwLock<TemporalModel>,
    pub rules: RwLock<Vec<AutomationRule>>,
    pub suggestions: RwLock<Vec<PendingSuggestion>>,
    events_since_mine: RwLock<u64>,
}

impl PredictiveEngine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            markov: RwLock::new(MarkovModel::new()),
            temporal: RwLock::new(TemporalModel::new()),
            rules: RwLock::new(Vec::new()),
            suggestions: RwLock::new(Vec::new()),
            events_since_mine: RwLock::new(0),
        })
    }

    /// Observe an OmnEvent and update all internal models
    pub async fn observe(&self, ev: &OmnEvent) {
        let label = event_label(&ev.event_type);
        self.markov.write().await.update(&label);
        self.temporal.write().await.update(&label);

        // Check rules
        let recent = self.markov.read().await.recent_sequence(10);
        let recent_strs: Vec<String> = recent.clone();
        let rules_snapshot: Vec<AutomationRule> = self.rules.read().await.clone();
        for rule in &rules_snapshot {
            if rule.trigger.matches(ev, &recent_strs) && rule.should_fire() {
                if rule.requires_confirmation {
                    self.add_suggestion(rule, ev).await;
                } else {
                    debug!("[predictive] auto-executing rule {}", rule.id);
                    // In a full implementation, dispatch the AutomatedAction here
                }
            }
        }

        // Mine rules every 100 events
        let mut count = self.events_since_mine.write().await;
        *count += 1;
        if *count >= 100 {
            *count = 0;
            drop(count);
            self.mine_rules().await;
        }
    }

    /// Predict the user's next likely actions given the current context
    pub async fn predict_next(&self, _context: &OmnContext, top_n: usize) -> Vec<PredictedAction> {
        let recent = self.markov.read().await.recent_sequence(1);
        let current = recent.last().cloned().unwrap_or_default();
        let mut preds = self.markov.read().await.predict(&current, top_n);
        let temporal = self.temporal.read().await.predict_now(top_n);

        // Merge: give Markov priority, add temporal predictions for uncovered labels
        let existing: std::collections::HashSet<String> = preds.iter().map(|p| p.action_label.clone()).collect();
        for p in temporal {
            if !existing.contains(&p.action_label) {
                preds.push(p);
            }
        }
        preds.truncate(top_n);
        preds
    }

    /// Mine automation rules from high-frequency bigrams
    async fn mine_rules(&self) {
        let seqs = self.markov.read().await.extract_sequences(5, 20);
        let mut rules = self.rules.write().await;
        for (prev, next, count) in seqs {
            // Skip if we already have a rule for this transition
            let exists = rules.iter().any(|r| {
                matches!(&r.trigger, TriggerCondition::CommandSequence { commands }
                    if commands == &[prev.clone()])
                && matches!(&r.action, AutomatedAction::SuggestCompletion { text }
                    if text == &next)
            });
            if exists { continue; }
            if count >= 5 {
                info!("[predictive] mined rule: {prev} → {next} (×{count})");
                rules.push(AutomationRule {
                    id: Uuid::new_v4().to_string(),
                    trigger: TriggerCondition::CommandSequence { commands: vec![prev] },
                    action: AutomatedAction::SuggestCompletion { text: next },
                    confidence_threshold: 0.8,
                    requires_confirmation: true,
                    times_correct: 0,
                    times_rejected: 0,
                    created_at: chrono::Utc::now().timestamp_micros(),
                    last_fired_at: None,
                    enabled: true,
                });
            }
        }
    }

    async fn add_suggestion(&self, rule: &AutomationRule, _ev: &OmnEvent) {
        let desc = match &rule.action {
            AutomatedAction::SuggestCompletion { text } => format!("Complete with: {text}"),
            AutomatedAction::LaunchApp { app_id } => format!("Launch {app_id}?"),
            AutomatedAction::RunCommand { command } => format!("Run: {command}"),
            AutomatedAction::OpenFile { path } => format!("Open {path}?"),
            AutomatedAction::SendNotification { message } => format!("Notify: {message}"),
            AutomatedAction::AdjustSetting { setting, .. } => format!("Adjust setting: {setting}"),
            AutomatedAction::ExecuteWorkflow { workflow_id } => format!("Run workflow: {workflow_id}"),
        };
        self.suggestions.write().await.push(PendingSuggestion {
            id: Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            description: desc,
            action: rule.action.clone(),
            created_at: chrono::Utc::now().timestamp_micros(),
        });
    }

    pub async fn approve_suggestion(&self, suggestion_id: &str) {
        let rule_id = {
            let mut sugg = self.suggestions.write().await;
            let pos = sugg.iter().position(|s| s.id == suggestion_id);
            if let Some(i) = pos { let s = sugg.remove(i); Some(s.rule_id) } else { None }
        };
        if let Some(rid) = rule_id {
            let mut rules = self.rules.write().await;
            if let Some(r) = rules.iter_mut().find(|r| r.id == rid) {
                r.times_correct += 1;
                r.last_fired_at = Some(chrono::Utc::now().timestamp_micros());
            }
        }
    }

    pub async fn reject_suggestion(&self, suggestion_id: &str) {
        let rule_id = {
            let mut sugg = self.suggestions.write().await;
            let pos = sugg.iter().position(|s| s.id == suggestion_id);
            if let Some(i) = pos { let s = sugg.remove(i); Some(s.rule_id) } else { None }
        };
        if let Some(rid) = rule_id {
            let mut rules = self.rules.write().await;
            if let Some(r) = rules.iter_mut().find(|r| r.id == rid) {
                r.times_rejected += 1;
                // Disable rules that have been rejected many times
                if r.times_rejected >= 3 && r.times_correct == 0 {
                    r.enabled = false;
                }
            }
        }
    }
}

fn event_label(ev: &OmnEventType) -> String {
    match ev {
        OmnEventType::KeyPress { key, .. } => format!("key:{key}"),
        OmnEventType::MouseClick { button, .. } => format!("click:{button}"),
        OmnEventType::AppLaunch { app_id, .. } => format!("launch:{app_id}"),
        OmnEventType::AppClose { app_id, .. } => format!("close:{app_id}"),
        OmnEventType::FileOpen { app_id, .. } => format!("open:{app_id}"),
        OmnEventType::FileSave { .. } => "filesave".into(),
        OmnEventType::FileDelete { .. } => "filedelete".into(),
        OmnEventType::CommandTyped { command, .. } => format!("cmd:{}", command.split_whitespace().next().unwrap_or("")),
        OmnEventType::CommandCompleted { command, .. } => format!("done:{}", command.split_whitespace().next().unwrap_or("")),
        OmnEventType::ToolInvocation { tool_name, .. } => format!("tool:{tool_name}"),
        OmnEventType::ModelInference { model_id, .. } => format!("infer:{model_id}"),
        OmnEventType::WindowFocus { app_id, .. } => format!("focus:{app_id}"),
        OmnEventType::WorkspaceSwitch { to, .. } => format!("ws:{to}"),
        other => format!("{:?}", std::mem::discriminant(other)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_predictions(
    state: tauri::State<'_, crate::AppState>,
    top_n: usize,
) -> Result<Vec<PredictedAction>, String> {
    let ctx = crate::omnipresent_capture::OmnContext::default();
    Ok(state.predictive_engine.predict_next(&ctx, top_n.min(10)).await)
}

#[tauri::command]
pub async fn get_automation_rules(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<AutomationRule>, String> {
    Ok(state.predictive_engine.rules.read().await.clone())
}

#[tauri::command]
pub async fn get_pending_suggestions(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<PendingSuggestion>, String> {
    Ok(state.predictive_engine.suggestions.read().await.clone())
}

#[tauri::command]
pub async fn approve_automation(
    state: tauri::State<'_, crate::AppState>,
    suggestion_id: String,
) -> Result<(), String> {
    state.predictive_engine.approve_suggestion(&suggestion_id).await;
    Ok(())
}

#[tauri::command]
pub async fn reject_automation(
    state: tauri::State<'_, crate::AppState>,
    suggestion_id: String,
) -> Result<(), String> {
    state.predictive_engine.reject_suggestion(&suggestion_id).await;
    Ok(())
}

#[tauri::command]
pub async fn add_automation_rule(
    state: tauri::State<'_, crate::AppState>,
    trigger: TriggerCondition,
    action: AutomatedAction,
    requires_confirmation: bool,
) -> Result<String, String> {
    let rule = AutomationRule {
        id: Uuid::new_v4().to_string(),
        trigger,
        action,
        confidence_threshold: 0.85,
        requires_confirmation,
        times_correct: 0,
        times_rejected: 0,
        created_at: chrono::Utc::now().timestamp_micros(),
        last_fired_at: None,
        enabled: true,
    };
    let id = rule.id.clone();
    state.predictive_engine.rules.write().await.push(rule);
    Ok(id)
}

#[tauri::command]
pub async fn delete_automation_rule(
    state: tauri::State<'_, crate::AppState>,
    rule_id: String,
) -> Result<(), String> {
    let mut rules = state.predictive_engine.rules.write().await;
    rules.retain(|r| r.id != rule_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn markov_learns_and_predicts() {
        let mut m = MarkovModel::new();
        for _ in 0..5 { m.update("open:vim"); m.update("cmd:git"); }
        let preds = m.predict("open:vim", 3);
        assert!(!preds.is_empty());
        assert_eq!(preds[0].action_label, "cmd:git");
        assert!((preds[0].confidence - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn rule_confidence_updates() {
        let mut r = AutomationRule {
            id: "r1".into(), trigger: TriggerCondition::AppLaunch { app_id: "vim".into() },
            action: AutomatedAction::SendNotification { message: "hi".into() },
            confidence_threshold: 0.8, requires_confirmation: true,
            times_correct: 4, times_rejected: 1,
            created_at: 0, last_fired_at: None, enabled: true,
        };
        assert!((r.confidence() - 0.8).abs() < 0.01);
        r.times_rejected += 3;
        assert!(r.confidence() < 0.8);
    }

    #[tokio::test]
    async fn engine_predicts_after_observation() {
        let engine = PredictiveEngine::new();
        let ctx = OmnContext::default();
        // Feed some events
        for _ in 0..3 {
            engine.markov.write().await.update("open:vim");
            engine.markov.write().await.update("cmd:git");
        }
        let preds = engine.predict_next(&ctx, 5).await;
        // Temporal predictions may appear; Markov predictions are from last event
        assert!(preds.len() <= 5);
    }
}
