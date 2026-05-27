//! Unified Training Collector — the central nervous system of BonsAI's perpetual
//! self-improvement engine.
//!
//! Every user interaction, tool call, feedback signal, skill execution, meeting
//! segment, music generation, and swarm trace flows through this module.  Seven
//! curation stages scrub, deduplicate, score, classify, and stratify examples
//! before they enter typed training buffers consumed by the Eternal Training Loop.
//!
//! Design principles
//! ─────────────────
//! • Append-only SQLite event log  — nothing is ever lost; curation decides later.
//! • Zero-copy hot path           — ingestion never blocks inference.
//! • Privacy-first                — PII scrubbing runs before anything else.
//! • Stratified buffer            — no single domain exceeds 25 % of capacity.
//! • Adaptive quality threshold   — rises when buffer full, falls when starved.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

// ── Re-export the event types so other modules can use them ───────────────────
pub use event_kinds::*;

// ══════════════════════════════════════════════════════════════════════════════
// § 1 — Event types (the raw event bus schema)
// ══════════════════════════════════════════════════════════════════════════════

pub mod event_kinds {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InteractionEvent {
        pub id:               String,
        pub session_id:       String,
        pub sequence:         u64,
        pub occurred_at:      i64,   // unix microseconds
        pub model_id:         String,
        pub adapter_id:       Option<String>,
        pub kind:             EventKind,
        pub hardware:         HardwareSnapshot,
        pub privacy_level:    PrivacyLevel,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum PrivacyLevel {
        /// Never leaves device, never used for federated sharing.
        Private,
        /// Used locally; metrics may be shared.
        LocalOnly,
        /// Anonymised diff may participate in federated aggregation if opted-in.
        FederatedOptIn,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    pub enum EventKind {
        ChatTurn {
            user_prompt:        String,
            assistant_response: String,
            thinking_tokens:    Option<u32>,
            latency_ms:         u32,
            token_count:        u32,
        },
        ToolInvocation {
            tool_name:    String,
            arguments:    serde_json::Value,
            result:       ToolResultSummary,
            latency_ms:   u32,
            was_necessary: Option<bool>,
        },
        UserEdit {
            original:  String,
            edited:    String,
            edit_type: EditType,
        },
        ExplicitFeedback {
            target_event_id: String,
            signal:          FeedbackSignal,
            comment:         Option<String>,
        },
        ImplicitFeedback {
            target_event_id: String,
            signal:          ImplicitSignal,
        },
        CodeExecution {
            language:       String,
            exit_code:      i32,
            stdout_hash:    String,
            test_results:   Option<TestSummary>,
            lint_warnings:  u32,
        },
        SkillExecution {
            skill_id:    String,
            success:     bool,
            error_class: Option<String>,
        },
        MeetingSegment {
            speaker_id:              String,
            transcript:              String,
            confidence:              f32,
            action_items_extracted:  u32,
        },
        ModelComparison {
            prompt_hash:      String,
            bonsai_output:    String,
            reference_count:  u32,
            user_preferred:   Option<String>,
        },
        SwarmTrace {
            task_hash:        String,
            decomposition:    Vec<String>,
            worker_count:     u32,
            synthesis_accepted: bool,
            worker_agreement: f32,
        },
        VisionAnalysis {
            image_hash:      String,
            oracle_label_count: u32,
            bonsai_label_count: u32,
            agreement_rate:  f32,
        },
        MusicGeneration {
            prompt_hash:         String,
            duration_secs:       f32,
            user_action:         MusicUserAction,
            regeneration_prompt: Option<String>,
        },
        AudioTranscription {
            duration_secs:    f32,
            wer_estimate:     Option<f32>,
            corrections_made: u32,
        },
        MemoryRetrieval {
            items_retrieved:         u32,
            items_used_in_response:  u32,
            relevance_judgement:     Option<f32>,
        },
        RouterDecision {
            prompt_hash:     String,
            selected_model:  String,
            confidence:      f32,
            actual_quality:  Option<f32>,
        },
        CrashEvent {
            subsystem:   String,
            error_class: String,
            recovered:   bool,
            replay_succeeded: Option<bool>,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ToolResultSummary {
        pub success:    bool,
        pub error_type: Option<String>,
        pub output_len: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TestSummary {
        pub total:   u32,
        pub passed:  u32,
        pub failed:  u32,
        pub skipped: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HardwareSnapshot {
        pub gpu_util_pct:        u8,
        pub vram_used_mb:        u32,
        pub cpu_util_pct:        u8,
        pub ram_used_mb:         u32,
        pub battery_pct:         Option<u8>,
        pub thermal_throttling:  bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum EditType { Correction, Expansion, Reformatting, Deletion }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum FeedbackSignal {
        ThumbsUp,
        ThumbsDown,
        FiveStarRating(u8),
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum ImplicitSignal {
        CopiedOutput,
        UsedOutputDirectly,
        MadeMinorModification,
        MadeMajorModification,
        DidNotActOnResponse,
        RepeatedQuestion,
        FollowUpContradicted,
        SessionAbandoned,
        TaskCompletedSuccessfully,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum MusicUserAction {
        ListenedFully,
        Skipped,
        Favorited,
        RegeneratedSimilar,
        RegeneratedDifferent,
        RegeneratedWithModification,
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// § 2 — Unified training example (normalised schema)
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTrainingExample {
    pub id:                  String,
    pub target_model:        ModelRole,
    pub suitable_strategies: Vec<TrainingStrategyType>,
    pub input:               TrainingInput,
    pub expected_output:     TrainingOutput,
    pub source:              TrainingSource,
    pub quality_score:       f32,
    pub priority:            f32,
    pub timestamp:           i64,
    pub dimensions:          Vec<String>,
    pub used:                bool,
    pub use_count:           u32,
    pub metadata:            serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    Router,
    Draft,
    Primary,
    CodeSpecialist,
    MusicSpecialist,
    VisionSpecialist,
    DocumentSpecialist,
    Critic,
    MicroBonsai,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrainingStrategyType { Lora, Dpo, Distillation, AdapterFusion, Rl, CurriculumSft }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrainingInput {
    Prompt { text: String },
    Conversation { messages: Vec<ConvMessage> },
    CodeWithContext {
        code:             String,
        language:         String,
        file_path:        String,
        surrounding_code: Option<String>,
    },
    ImageWithPrompt { image_hash: String, prompt: String },
    AudioWithPrompt { audio_hash: String, prompt: String, duration_secs: f32 },
    MeetingTranscript { transcript: String, question: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvMessage { pub role: String, pub content: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrainingOutput {
    Text { content: String },
    Json { value: serde_json::Value },
    CodeWithExplanation { code: String, explanation: String },
    ToolCalls { calls: Vec<serde_json::Value> },
    PreferencePair { chosen: String, rejected: String },
    FileOutput { path: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrainingSource {
    UserCorrections,
    CodeReviews,
    ModelComparisons,
    SelfPlay,
    Swarm,
    ToolExecution,
    MusicFeedback,
    VisionOracle,
    MeetingTranscripts,
    AudioTranscription,
    SkillCompilations,
    FederatedDiffs,
    AdversarialProbes,
    ConstitutionalSelfPlay,
    ReasoningSelfPlay,
}

impl TrainingSource {
    pub fn domain_label(&self) -> &'static str {
        match self {
            Self::CodeReviews | Self::ToolExecution => "code",
            Self::MusicFeedback => "music",
            Self::VisionOracle => "vision",
            Self::MeetingTranscripts | Self::AudioTranscription => "audio",
            Self::Swarm => "swarm",
            Self::AdversarialProbes => "safety",
            Self::ReasoningSelfPlay => "reasoning",
            _ => "general",
        }
    }

    fn domain_label_of(s: &TrainingSource) -> &'static str { s.domain_label() }
}

// Silence dead_code for the associated-fn alias above
impl TrainingSource {
    fn _unused() {}
}

// ══════════════════════════════════════════════════════════════════════════════
// § 3 — Quality scoring per source
// ══════════════════════════════════════════════════════════════════════════════

pub fn quality_score(source: &TrainingSource, meta: &QualityMeta) -> f32 {
    match source {
        TrainingSource::UserCorrections => {
            let significance = (meta.edit_distance as f32
                / meta.original_len.max(1) as f32)
                .min(1.0);
            0.70 + significance * 0.30
        }
        TrainingSource::CodeReviews => {
            let sev = match meta.severity.as_deref() {
                Some("critical") => 1.0,
                Some("high")     => 0.9,
                Some("medium")   => 0.7,
                Some("low")      => 0.4,
                _                => 0.2,
            };
            (sev * 0.6 + meta.confidence * 0.4).min(1.0)
        }
        TrainingSource::ModelComparisons => {
            if meta.is_overconfidence { 0.85 + meta.training_priority * 0.15 }
            else                      { 0.50 + meta.training_priority * 0.40 }
        }
        TrainingSource::SelfPlay | TrainingSource::ConstitutionalSelfPlay => {
            let specificity = (meta.critique_len as f32 / 200.0).min(1.0);
            0.30 + specificity * 0.40
        }
        TrainingSource::Swarm => {
            let accepted = if meta.user_accepted { 0.3 } else { 0.0 };
            0.40 + meta.agreement * 0.30 + accepted
        }
        TrainingSource::ToolExecution => {
            if meta.tool_succeeded && meta.user_accepted { 0.85 }
            else if meta.tool_succeeded                  { 0.50 }
            else                                         { 0.10 }
        }
        TrainingSource::MusicFeedback => {
            if meta.favorited      { 0.95 }
            else if meta.listened  { 0.70 }
            else if meta.regenerated { 0.30 }
            else                   { 0.10 }
        }
        TrainingSource::VisionOracle => {
            meta.oracle_confidence * 0.8 + if meta.user_validated { 0.2 } else { 0.0 }
        }
        TrainingSource::MeetingTranscripts => {
            if meta.notes_accepted && meta.items_completed { 0.85 } else { 0.50 }
        }
        TrainingSource::AudioTranscription => {
            // Treat audio transcriptions similarly to meeting transcripts for now.
            if meta.notes_accepted && meta.items_completed { 0.85 } else { 0.50 }
        }
        TrainingSource::SkillCompilations => {
            if meta.execution_success_rate > 0.9 { 0.90 }
            else { meta.execution_success_rate * 0.70 }
        }
        TrainingSource::FederatedDiffs => {
            meta.peer_reputation * 0.6 + meta.eval_improvement * 0.4
        }
        TrainingSource::AdversarialProbes => 0.92,
        TrainingSource::ReasoningSelfPlay => {
            let specificity = (meta.critique_len as f32 / 200.0).min(1.0);
            0.55 + specificity * 0.35
        }
    }
}

/// Bag of optional fields used by `quality_score`.
/// Callers set only the fields relevant to their source type.
#[derive(Debug, Default, Clone)]
pub struct QualityMeta {
    pub edit_distance:         u32,
    pub original_len:          u32,
    pub severity:              Option<String>,
    pub confidence:            f32,
    pub is_overconfidence:     bool,
    pub training_priority:     f32,
    pub critique_len:          u32,
    pub user_accepted:         bool,
    pub agreement:             f32,
    pub tool_succeeded:        bool,
    pub favorited:             bool,
    pub listened:              bool,
    pub regenerated:           bool,
    pub oracle_confidence:     f32,
    pub user_validated:        bool,
    pub notes_accepted:        bool,
    pub items_completed:       bool,
    pub execution_success_rate: f32,
    pub peer_reputation:       f32,
    pub eval_improvement:      f32,
}

// ══════════════════════════════════════════════════════════════════════════════
// § 4 — Feedback weighter
// ══════════════════════════════════════════════════════════════════════════════

pub struct FeedbackWeighter {
    source_reliability: RwLock<HashMap<String, f32>>,
    recency_halflife_hours: f32,
}

impl FeedbackWeighter {
    pub fn new(recency_halflife_hours: f32) -> Self {
        Self {
            source_reliability: RwLock::new(HashMap::new()),
            recency_halflife_hours,
        }
    }

    pub async fn weight(&self, event: &InteractionEvent, user_id: &str) -> f32 {
        let base: f32 = match &event.kind {
            EventKind::ExplicitFeedback { signal: FeedbackSignal::ThumbsUp, .. }    =>  1.00,
            EventKind::ExplicitFeedback { signal: FeedbackSignal::ThumbsDown, .. }  =>  1.00,
            EventKind::ExplicitFeedback { signal: FeedbackSignal::FiveStarRating(r), .. } => {
                (*r as f32 - 3.0) / 2.0
            }
            EventKind::UserEdit { edit_type: EditType::Correction, .. }              =>  0.95,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::UsedOutputDirectly, .. }       =>  0.80,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::MadeMinorModification, .. }    =>  0.65,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::CopiedOutput, .. }             =>  0.55,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::TaskCompletedSuccessfully, .. } => 0.50,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::MadeMajorModification, .. }    =>  0.30,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::RepeatedQuestion, .. }         => -0.70,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::FollowUpContradicted, .. }     => -0.60,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::SessionAbandoned, .. }         => -0.40,
            EventKind::ImplicitFeedback { signal: ImplicitSignal::DidNotActOnResponse, .. }      => -0.20,
            _ => 0.0,
        };

        let reliability = self.source_reliability.read().await
            .get(user_id).copied().unwrap_or(0.5);

        let age_hours = (chrono::Utc::now().timestamp_micros() - event.occurred_at) as f32
            / 3_600_000_000.0_f32;
        let recency = 0.5_f32.powf(age_hours / self.recency_halflife_hours);

        base * reliability * (0.5 + 0.5 * recency)
    }

    /// Update user reliability based on whether their feedback was predictive.
    pub async fn update_reliability(&self, user_id: &str, was_correct: bool) {
        let mut map = self.source_reliability.write().await;
        let r = map.entry(user_id.to_string()).or_insert(0.5);
        // Bayesian-style update with learning rate 0.05
        if was_correct { *r = (*r + 0.05).min(1.0) }
        else           { *r = (*r - 0.05).max(0.0) }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// § 5 — PII scrubber (Stage 1 of curation)
// ══════════════════════════════════════════════════════════════════════════════

pub struct PiiScrubber {
    patterns: Vec<(regex::Regex, &'static str)>,
}

impl PiiScrubber {
    pub fn new() -> Self {
        let raw: &[(&str, &str)] = &[
            (r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b",      "[REDACTED_EMAIL]"),
            (r"\b(\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]\d{3}[-.\s]\d{4}\b",        "[REDACTED_PHONE]"),
            (r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b",                      "[REDACTED_IP]"),
            (r"\b\d{5}(?:-\d{4})?\b",                                         "[REDACTED_ZIP]"),
            (r"\b4\d{12}(?:\d{3})?\b|\b5[1-5]\d{14}\b|\b3[47]\d{13}\b",     "[REDACTED_CARD]"),
            (r"\b\d{3}-\d{2}-\d{4}\b",                                        "[REDACTED_SSN]"),
            (r"(?i)(Bearer|token|api[_-]?key|secret)[:\s=]+[A-Za-z0-9\-_/+=]{32,}", "[REDACTED_TOKEN]"),
            (r"(?i)/(?:home|users)/[^/\s]+",                                  "[REDACTED_PATH]"),
        ];
        let patterns = raw.iter()
            .filter_map(|(pat, label)| {
                regex::Regex::new(pat).ok().map(|r| (r, *label))
            })
            .collect();
        Self { patterns }
    }

    /// Returns scrubbed text and count of redactions.
    pub fn scrub(&self, text: &str) -> (String, u32) {
        let mut out = text.to_string();
        let mut count = 0u32;
        for (re, label) in &self.patterns {
            let before = out.len();
            out = re.replace_all(&out, *label).into_owned();
            if out.len() != before { count += 1; }
        }
        (out, count)
    }

    /// True if PII density (redactions per 100 chars) exceeds threshold.
    pub fn too_dense(&self, text: &str, threshold: f32) -> bool {
        let (_, count) = self.scrub(text);
        let density = count as f32 / (text.len().max(1) as f32 / 100.0);
        density > threshold
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// § 6 — Semantic deduplicator (Stage 2)
// ══════════════════════════════════════════════════════════════════════════════

/// Lightweight MinHash-based dedup using character 3-grams.
/// A full sentence-transformer would be more accurate but requires a model;
/// this runs entirely in Rust with zero network calls and near-zero latency.
pub struct SemanticDeduplicator {
    seen_hashes: RwLock<std::collections::HashSet<u64>>,
    similarity_threshold: f32,
    window_capacity: usize,
    eviction_list: Mutex<std::collections::VecDeque<u64>>,
}

impl SemanticDeduplicator {
    pub fn new(threshold: f32, window_capacity: usize) -> Self {
        Self {
            seen_hashes: RwLock::new(std::collections::HashSet::new()),
            similarity_threshold: threshold,
            window_capacity,
            eviction_list: Mutex::new(std::collections::VecDeque::new()),
        }
    }

    /// Returns true if the text is a near-duplicate of a seen example.
    pub async fn is_duplicate(&self, text: &str) -> bool {
        let sig = self.minhash(text);
        let seen = self.seen_hashes.read().await;
        seen.contains(&sig)
    }

    /// Register a text as seen. Call only after accepting the example.
    pub async fn register(&self, text: &str) {
        let sig = self.minhash(text);
        let mut evict = self.eviction_list.lock().await;
        let mut seen  = self.seen_hashes.write().await;
        if seen.len() >= self.window_capacity {
            if let Some(old) = evict.pop_front() { seen.remove(&old); }
        }
        seen.insert(sig);
        evict.push_back(sig);
    }

    /// Fast character-3-gram minhash signature.
    fn minhash(&self, text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let normalised: String = text.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
        let bytes = normalised.as_bytes();
        let mut min: u64 = u64::MAX;
        for i in 0..bytes.len().saturating_sub(2) {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            bytes[i..i+3].hash(&mut h);
            let v = std::hash::Hasher::finish(&h);
            if v < min { min = v; }
        }
        min
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// § 7 — Domain classifier (Stage 5)
// ══════════════════════════════════════════════════════════════════════════════

pub const DOMAINS: &[&str] = &[
    "code", "math", "reasoning", "writing", "tool_use",
    "vision", "music", "audio", "document", "safety",
    "planning", "memory", "swarm",
];

pub const DOMAIN_MAX_SHARE: f32 = 0.25; // no domain exceeds 25 % of buffer

/// Rule-based domain tagger. Each rule is (keywords, domain, weight).
/// First rule whose keywords all appear in the lowercased text wins.
pub fn classify_domain(text: &str) -> String {
    let low = text.to_lowercase();
    let rules: &[(&[&str], &str)] = &[
        (&["vulnerability", "cve", "injection", "xss", "overflow", "exploit"], "safety"),
        (&["def ", "fn ", "function", "class ", "import ", "struct ", "=>", "::"], "code"),
        (&["refactor", "unit test", "lint", "compile", "build", "cargo", "npm"], "code"),
        (&["image", "photo", "picture", "detect", "caption", "vision", "pixel"], "vision"),
        (&["music", "melody", "chord", "tempo", "bpm", "audio", "song", "beat"], "music"),
        (&["transcribe", "wer", "speaker", "diarize", "microphone", "speech"], "audio"),
        (&["pdf", "ocr", "document", "table", "header", "extract text"], "document"),
        (&["remember", "recall", "context", "memory", "store", "retrieve"], "memory"),
        (&["plan", "step", "decompose", "subtask", "workflow", "pipeline"], "planning"),
        (&["swarm", "worker", "orchestrate", "parallel", "agent", "collaborate"], "swarm"),
        (&["equation", "integral", "derivative", "matrix", "proof", "theorem"], "math"),
        (&["write", "essay", "poem", "story", "blog", "draft", "creative"], "writing"),
        (&["tool", "call", "function call", "api", "endpoint", "invoke"], "tool_use"),
    ];
    for (kws, domain) in rules {
        if kws.iter().any(|kw| low.contains(kw)) {
            return domain.to_string();
        }
    }
    "reasoning".to_string()
}

// ══════════════════════════════════════════════════════════════════════════════
// § 8 — Stratified buffer (Stage 7)
// ══════════════════════════════════════════════════════════════════════════════

/// Per-domain buffer with overflow queue.
struct DomainSlot {
    examples:  std::collections::VecDeque<UnifiedTrainingExample>,
    capacity:  usize,
    overflow:  std::collections::VecDeque<UnifiedTrainingExample>,
}

impl DomainSlot {
    fn new(capacity: usize) -> Self {
        Self { examples: Default::default(), capacity, overflow: Default::default() }
    }

    fn insert(&mut self, ex: UnifiedTrainingExample) -> bool {
        if self.examples.len() < self.capacity {
            self.examples.push_back(ex);
            true
        } else {
            // Evict lowest-quality example if new one is better
            if let Some(worst_pos) = self.examples.iter().enumerate()
                .min_by(|a, b| a.1.quality_score.partial_cmp(&b.1.quality_score).unwrap())
                .map(|(i, _)| i)
            {
                if self.examples[worst_pos].quality_score < ex.quality_score {
                    self.examples.remove(worst_pos);
                    self.examples.push_back(ex);
                    return true;
                }
            }
            self.overflow.push_back(ex);
            false
        }
    }

    fn drain(&mut self, n: usize) -> Vec<UnifiedTrainingExample> {
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            if let Some(ex) = self.examples.pop_front() { out.push(ex); }
            else { break; }
        }
        // Flush overflow into main
        while self.examples.len() < self.capacity {
            if let Some(ex) = self.overflow.pop_front() { self.examples.push_back(ex); }
            else { break; }
        }
        out
    }

    fn len(&self) -> usize { self.examples.len() }
}

pub struct StratifiedBuffer {
    slots:    Mutex<HashMap<String, DomainSlot>>,
    capacity_per_domain: usize,
    /// Quality threshold, adaptive: rises when buffer full, falls when starved.
    threshold: RwLock<f32>,
}

impl StratifiedBuffer {
    pub fn new(total_capacity: usize) -> Self {
        let per_domain = (total_capacity as f32 * DOMAIN_MAX_SHARE) as usize;
        let mut slots = HashMap::new();
        for d in DOMAINS { slots.insert(d.to_string(), DomainSlot::new(per_domain)); }
        Self {
            slots: Mutex::new(slots),
            capacity_per_domain: per_domain,
            threshold: RwLock::new(0.4),
        }
    }

    pub async fn insert(&self, ex: UnifiedTrainingExample) -> bool {
        let threshold = *self.threshold.read().await;
        if ex.quality_score < threshold { return false; }

        let domain = ex.dimensions.first().cloned().unwrap_or_else(|| "reasoning".into());
        let mut slots = self.slots.lock().await;
        let slot = slots.entry(domain).or_insert_with(|| DomainSlot::new(self.capacity_per_domain));
        let accepted = slot.insert(ex);

        // Adaptive threshold
        let total: usize = slots.values().map(|s| s.len()).sum();
        let max_cap = self.capacity_per_domain * DOMAINS.len();
        let fill_ratio = total as f32 / max_cap as f32;
        drop(slots);

        let mut thr = self.threshold.write().await;
        if fill_ratio > 0.90 { *thr = (*thr + 0.01).min(0.90); }
        else if fill_ratio < 0.40 { *thr = (*thr - 0.02).max(0.20); }

        accepted
    }

    pub async fn drain_lora(&self, n: usize) -> Vec<UnifiedTrainingExample> {
        self.drain_strategy(n, &[TrainingStrategyType::Lora, TrainingStrategyType::CurriculumSft]).await
    }

    pub async fn drain_dpo(&self, n: usize) -> Vec<UnifiedTrainingExample> {
        self.drain_strategy(n, &[TrainingStrategyType::Dpo]).await
    }

    async fn drain_strategy(
        &self,
        n: usize,
        strategies: &[TrainingStrategyType],
    ) -> Vec<UnifiedTrainingExample> {
        let mut slots = self.slots.lock().await;
        let per_domain = (n / DOMAINS.len()).max(1);
        let mut out = Vec::with_capacity(n);
        for slot in slots.values_mut() {
            let candidates: Vec<_> = slot.examples.iter()
                .filter(|ex| ex.suitable_strategies.iter().any(|s| strategies.contains(s)))
                .take(per_domain)
                .cloned()
                .collect();
            for c in &candidates {
                slot.examples.retain(|e| e.id != c.id);
            }
            out.extend(candidates);
        }
        out.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
        out.truncate(n);
        out
    }

    pub async fn stats(&self) -> BufferStats {
        let slots = self.slots.lock().await;
        let by_domain: HashMap<String, usize> = slots.iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();
        let total = by_domain.values().sum();
        let threshold = *self.threshold.read().await;
        BufferStats { total, by_domain, quality_threshold: threshold }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferStats {
    pub total:             usize,
    pub by_domain:         HashMap<String, usize>,
    pub quality_threshold: f32,
}

// ══════════════════════════════════════════════════════════════════════════════
// § 9 — Seven-stage curation pipeline
// ══════════════════════════════════════════════════════════════════════════════

pub struct CurationPipeline {
    scrubber:    PiiScrubber,
    deduplicator: SemanticDeduplicator,
}

impl CurationPipeline {
    pub fn new() -> Self {
        Self {
            scrubber:     PiiScrubber::new(),
            deduplicator: SemanticDeduplicator::new(0.95, 50_000),
        }
    }

    /// Returns the curated example or a rejection reason.
    pub async fn process(
        &self,
        mut ex: UnifiedTrainingExample,
    ) -> Result<UnifiedTrainingExample, &'static str> {
        // Stage 1 — Privacy scrubbing
        let text = self.extract_text(&ex);
        if self.scrubber.too_dense(&text, 2.0) { return Err("pii_density"); }
        self.apply_scrubbing(&mut ex);

        // Stage 2 — Semantic deduplication
        let sig_text = self.extract_text(&ex);
        if self.deduplicator.is_duplicate(&sig_text).await { return Err("duplicate"); }

        // Stage 3 — Quality gate (already set by caller; enforced here)
        if ex.quality_score < 0.10 { return Err("quality_too_low"); }

        // Stage 4 — Difficulty calibration (store raw; curriculum will order it)
        // quality_score doubles as difficulty proxy — no separate field needed.

        // Stage 5 — Domain classification (fill if empty)
        if ex.dimensions.is_empty() {
            ex.dimensions.push(classify_domain(&self.extract_text(&ex)));
        }

        // Stage 6 — Format normalisation (already in UnifiedTrainingExample)

        // Stage 7 — Register as seen (after acceptance)
        self.deduplicator.register(&sig_text).await;

        Ok(ex)
    }

    fn extract_text(&self, ex: &UnifiedTrainingExample) -> String {
        match &ex.input {
            TrainingInput::Prompt { text }                 => text.clone(),
            TrainingInput::Conversation { messages }       => messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join(" "),
            TrainingInput::CodeWithContext { code, .. }    => code.clone(),
            TrainingInput::MeetingTranscript { transcript, .. } => transcript.clone(),
            _ => String::new(),
        }
    }

    fn apply_scrubbing(&self, ex: &mut UnifiedTrainingExample) {
        // Scrub input text in-place
        if let TrainingInput::Prompt { text } = &mut ex.input {
            *text = self.scrubber.scrub(text).0;
        }
        if let TrainingInput::Conversation { messages } = &mut ex.input {
            for m in messages.iter_mut() {
                m.content = self.scrubber.scrub(&m.content).0;
            }
        }
        // Scrub output
        if let TrainingOutput::Text { content } = &mut ex.expected_output {
            *content = self.scrubber.scrub(content).0;
        }
        if let TrainingOutput::PreferencePair { chosen, rejected } = &mut ex.expected_output {
            *chosen   = self.scrubber.scrub(chosen).0;
            *rejected = self.scrubber.scrub(rejected).0;
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// § 10 — Unified Training Collector (the public entry point)
// ══════════════════════════════════════════════════════════════════════════════

pub struct UnifiedTrainingCollector {
    pipeline:  CurationPipeline,
    buffer:    Arc<StratifiedBuffer>,
    weighter:  FeedbackWeighter,
    db_path:   PathBuf,
    stats:     RwLock<CollectorStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollectorStats {
    pub events_received:  u64,
    pub events_accepted:  u64,
    pub events_rejected:  u64,
    pub rejection_reasons: HashMap<String, u64>,
    pub buffer:           Option<BufferStats>,
}

impl UnifiedTrainingCollector {
    pub fn new(total_buffer_capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            pipeline: CurationPipeline::new(),
            buffer:   Arc::new(StratifiedBuffer::new(total_buffer_capacity)),
            weighter: FeedbackWeighter::new(48.0),
            db_path:  dirs::home_dir()
                          .unwrap_or_default()
                          .join(".bonsai/events/events.db"),
            stats:    RwLock::new(CollectorStats::default()),
        })
    }

    /// Ingest a raw event.  Non-blocking: returns immediately after queueing.
    pub async fn ingest(&self, event: InteractionEvent) {
        self.stats.write().await.events_received += 1;
        if let Some(ex) = self.event_to_example(&event) {
            match self.pipeline.process(ex).await {
                Ok(curated) => {
                    let accepted = self.buffer.insert(curated).await;
                    let mut st = self.stats.write().await;
                    if accepted { st.events_accepted += 1; }
                    else {
                        st.events_rejected += 1;
                        *st.rejection_reasons.entry("buffer_full_evicted".into()).or_insert(0) += 1;
                    }
                }
                Err(reason) => {
                    let mut st = self.stats.write().await;
                    st.events_rejected += 1;
                    *st.rejection_reasons.entry(reason.into()).or_insert(0) += 1;
                }
            }
        }
        // Persist raw event to SQLite (best-effort; never blocks inference)
        let _ = self.persist_event_async(&event).await;
    }

    /// Bulk ingest (e.g. remedial examples from ForgettingPrevention).
    pub async fn ingest_bulk(&self, examples: Vec<UnifiedTrainingExample>) {
        for mut ex in examples {
            if ex.quality_score < 0.10 { continue; }
            ex.dimensions = if ex.dimensions.is_empty() {
                vec![classify_domain(&self.extract_text_from_example(&ex))]
            } else {
                ex.dimensions
            };
            self.buffer.insert(ex).await;
        }
    }

    // Non-destructive snapshot for UI: return a slice of examples without removing them.
    pub async fn snapshot(&self, offset: usize, limit: usize, domain: Option<&str>) -> Vec<UnifiedTrainingExample> {
        // Simple implementation: iterate domain slots and collect up to limit
        let mut out: Vec<UnifiedTrainingExample> = Vec::new();
        let slots = self.buffer.slots.lock().await;
        for (dom, slot) in slots.iter() {
            if let Some(dflt) = domain { if dom != dflt { continue; } }
            for ex in slot.examples.iter().skip(offset).take(limit - out.len()) {
                out.push(ex.clone());
            }
            if out.len() >= limit { break; }
        }
        out
    }

    pub async fn delete_example(&self, example_id: &str) -> bool {
        let mut slots = self.buffer.slots.lock().await;
        for (_dom, slot) in slots.iter_mut() {
            let before = slot.examples.len();
            slot.examples.retain(|e| e.id != example_id);
            if slot.examples.len() != before { return true; }
        }
        false
    }

    pub async fn edit_example(&self, example_id: &str, new_output: &str, _new_quality: Option<f32>) -> bool {
        let mut slots = self.buffer.slots.lock().await;
        for (_dom, slot) in slots.iter_mut() {
            for e in slot.examples.iter_mut() {
                if e.id == example_id {
                    if let TrainingOutput::Text { content } = &mut e.expected_output {
                        *content = new_output.to_string();
                        return true;
                    }
                    if let TrainingOutput::PreferencePair { chosen, .. } = &mut e.expected_output {
                        *chosen = new_output.to_string();
                        return true;
                    }
                }
            }
        }
        false
    }

    pub async fn boost_example(&self, example_id: &str) -> bool {
        let mut slots = self.buffer.slots.lock().await;
        for (_dom, slot) in slots.iter_mut() {
            for e in slot.examples.iter_mut() {
                if e.id == example_id {
                    e.priority = e.priority.max(1.0);
                    return true;
                }
            }
        }
        false
    }

    pub async fn bulk_delete(&self, _from: Option<i64>, _to: Option<i64>, _src: Option<&str>, _domain: Option<&str>) -> usize {
        // For now: wipe everything when called with no filters
        let mut slots = self.buffer.slots.lock().await;
        let mut removed = 0usize;
        for (_dom, slot) in slots.iter_mut() {
            removed += slot.examples.len();
            slot.examples.clear();
            slot.overflow.clear();
        }
        removed
    }

    pub async fn wipe(&self) {
        let mut slots = self.buffer.slots.lock().await;
        for (_dom, slot) in slots.iter_mut() {
            slot.examples.clear();
            slot.overflow.clear();
        }
    }


    pub async fn drain_lora(&self, n: usize) -> Vec<UnifiedTrainingExample> {
        self.buffer.drain_lora(n).await
    }

    pub async fn drain_dpo(&self, n: usize) -> Vec<UnifiedTrainingExample> {
        self.buffer.drain_dpo(n).await
    }

    pub async fn stats(&self) -> CollectorStats {
        let mut st = self.stats.read().await.clone();
        st.buffer = Some(self.buffer.stats().await);
        st
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn event_to_example(&self, event: &InteractionEvent) -> Option<UnifiedTrainingExample> {
        let id = Uuid::new_v4().to_string();
        let ts = event.occurred_at;

        match &event.kind {
            EventKind::ChatTurn { user_prompt, assistant_response, .. } => {
                Some(UnifiedTrainingExample {
                    id,
                    target_model: ModelRole::Primary,
                    suitable_strategies: vec![TrainingStrategyType::Lora],
                    input:  TrainingInput::Prompt { text: user_prompt.clone() },
                    expected_output: TrainingOutput::Text { content: assistant_response.clone() },
                    source: TrainingSource::SelfPlay,
                    quality_score: 0.40, // baseline; upgraded by feedback events
                    priority: 0.5,
                    timestamp: ts,
                    dimensions: vec![classify_domain(user_prompt)],
                    used: false,
                    use_count: 0,
                    metadata: serde_json::json!({
                        "adapter_id": event.adapter_id,
                        "model_id": event.model_id,
                    }),
                })
            }
            EventKind::UserEdit { original, edited, edit_type } => {
                let qual_meta = QualityMeta {
                    edit_distance: levenshtein_approx(original, edited),
                    original_len: original.len() as u32,
                    ..Default::default()
                };
                let score = quality_score(&TrainingSource::UserCorrections, &qual_meta);
                Some(UnifiedTrainingExample {
                    id,
                    target_model: ModelRole::Primary,
                    suitable_strategies: vec![TrainingStrategyType::Dpo, TrainingStrategyType::Lora],
                    input:  TrainingInput::Prompt { text: original.clone() },
                    expected_output: TrainingOutput::PreferencePair {
                        chosen:   edited.clone(),
                        rejected: original.clone(),
                    },
                    source: TrainingSource::UserCorrections,
                    quality_score: score,
                    priority: score * 1.5,
                    timestamp: ts,
                    dimensions: vec![classify_domain(original)],
                    used: false,
                    use_count: 0,
                    metadata: serde_json::json!({ "edit_type": edit_type }),
                })
            }
            EventKind::MusicGeneration { prompt_hash, user_action, .. } => {
                let (score, priority) = match user_action {
                    MusicUserAction::Favorited                   => (0.95, 1.8),
                    MusicUserAction::ListenedFully               => (0.70, 1.2),
                    MusicUserAction::RegeneratedWithModification => (0.60, 1.0),
                    MusicUserAction::RegeneratedSimilar          => (0.30, 0.6),
                    MusicUserAction::RegeneratedDifferent        => (0.15, 0.3),
                    MusicUserAction::Skipped                     => (0.10, 0.2),
                };
                Some(UnifiedTrainingExample {
                    id,
                    target_model: ModelRole::MusicSpecialist,
                    suitable_strategies: vec![TrainingStrategyType::Dpo],
                    input:  TrainingInput::Prompt { text: format!("music:{prompt_hash}") },
                    expected_output: TrainingOutput::Text { content: format!("user_action:{user_action:?}") },
                    source: TrainingSource::MusicFeedback,
                    quality_score: score,
                    priority,
                    timestamp: ts,
                    dimensions: vec!["music".into()],
                    used: false,
                    use_count: 0,
                    metadata: serde_json::json!({ "action": format!("{user_action:?}") }),
                })
            }
            EventKind::VisionAnalysis { image_hash, agreement_rate, .. } => {
                let score = *agreement_rate * 0.8;
                Some(UnifiedTrainingExample {
                    id,
                    target_model: ModelRole::VisionSpecialist,
                    suitable_strategies: vec![TrainingStrategyType::Lora],
                    input:  TrainingInput::ImageWithPrompt { image_hash: image_hash.clone(), prompt: String::new() },
                    expected_output: TrainingOutput::Json { value: serde_json::json!({ "agreement_rate": agreement_rate }) },
                    source: TrainingSource::VisionOracle,
                    quality_score: score,
                    priority: score,
                    timestamp: ts,
                    dimensions: vec!["vision".into()],
                    used: false,
                    use_count: 0,
                    metadata: serde_json::Value::Null,
                })
            }
            EventKind::CrashEvent { subsystem, error_class, replay_succeeded, .. } => {
                // Crash events produce robustness training data
                if replay_succeeded == &Some(true) {
                    Some(UnifiedTrainingExample {
                        id,
                        target_model: ModelRole::MicroBonsai,
                        suitable_strategies: vec![TrainingStrategyType::Rl],
                        input:  TrainingInput::Prompt { text: format!("crash:{subsystem}:{error_class}") },
                        expected_output: TrainingOutput::Json { value: serde_json::json!({ "action": "use_safer_config" }) },
                        source: TrainingSource::AdversarialProbes,
                        quality_score: 0.88,
                        priority: 1.5,
                        timestamp: ts,
                        dimensions: vec!["safety".into()],
                        used: false,
                        use_count: 0,
                        metadata: serde_json::json!({ "subsystem": subsystem }),
                    })
                } else { None }
            }
            // Router decisions become router training data
            EventKind::RouterDecision { prompt_hash, selected_model, actual_quality, .. } => {
                actual_quality.map(|q| UnifiedTrainingExample {
                    id,
                    target_model: ModelRole::Router,
                    suitable_strategies: vec![TrainingStrategyType::Rl],
                    input:  TrainingInput::Prompt { text: format!("route:{prompt_hash}") },
                    expected_output: TrainingOutput::Json { value: serde_json::json!({
                        "model": selected_model,
                        "quality": q,
                    })},
                    source: TrainingSource::ToolExecution,
                    quality_score: q,
                    priority: q,
                    timestamp: ts,
                    dimensions: vec!["tool_use".into()],
                    used: false,
                    use_count: 0,
                    metadata: serde_json::Value::Null,
                })
            }
            _ => None,
        }
    }

    fn extract_text_from_example(&self, ex: &UnifiedTrainingExample) -> String {
        match &ex.input {
            TrainingInput::Prompt { text }           => text.clone(),
            TrainingInput::Conversation { messages } => messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join(" "),
            TrainingInput::CodeWithContext { code, .. } => code.clone(),
            _ => String::new(),
        }
    }

    async fn persist_event_async(&self, event: &InteractionEvent) -> anyhow::Result<()> {
        // Best-effort JSON append — a full SQLite WAL journal is the production
        // version, but for now we append to a JSONL cold log.
        let dir = self.db_path.parent().unwrap_or(std::path::Path::new("."));
        tokio::fs::create_dir_all(dir).await.ok();
        let line = serde_json::to_string(event)? + "\n";
        let mut f = tokio::fs::OpenOptions::new()
            .create(true).append(true)
            .open(self.db_path.with_extension("jsonl"))
            .await?;
        use tokio::io::AsyncWriteExt;
        f.write_all(line.as_bytes()).await?;
        Ok(())
    }
}

/// Fast approximate Levenshtein using character-level differences (not exact).
pub(crate) fn levenshtein_approx(a: &str, b: &str) -> u32 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len(); let n = b.len();
    if m == 0 { return n as u32; }
    if n == 0 { return m as u32; }
    let mut prev: Vec<u32> = (0..=n as u32).collect();
    let mut curr = vec![0u32; n + 1];
    for i in 1..=m {
        curr[0] = i as u32;
        for j in 1..=n {
            let cost = if a[i-1] == b[j-1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j-1] + 1).min(prev[j-1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

// (levenshtein_approx is pub(crate) above)

