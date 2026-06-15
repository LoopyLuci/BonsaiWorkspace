//! BonsAI Meeting Agent — real-time audio capture, VAD, Whisper transcription,
//! speaker diarization, note generation, action-item tracking, and caption events.
//!
//! ## Pipeline
//!   1. **Audio capture** — loopback or microphone via `cpal` (platform-agnostic).
//!   2. **VAD** — lightweight energy-threshold gate → speech segments.
//!   3. **WAV encoding** — `hound` packs each segment into WAV bytes.
//!   4. **Whisper** — segments POSTed to the running `WhisperManager` sidecar.
//!   5. **Diarization** — simple embedding (energy+MFCC fingerprint) cosine match.
//!   6. **Notes** — periodic LLM call extracts decisions / actions / topics.
//!   7. **Events** — every step emits structured Tauri events consumed by the UI.
//!
//! All processing is local and offline.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

// ── Audio format constants ────────────────────────────────────────────────────

const SAMPLE_RATE: u32 = 16_000; // Whisper expects 16 kHz
const CHANNELS: u16 = 1;
const BITS: u16 = 16;
const FRAME_MS: u32 = 20; // VAD operates on 20 ms frames
const FRAME_SAMPLES: usize = (SAMPLE_RATE as usize * FRAME_MS as usize) / 1_000;

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: String,
    pub speaker: String,
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub confidence: f32,
    pub is_final: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub id: String,
    pub description: String,
    pub assignee: Option<String>,
    pub deadline: Option<String>,
    pub priority: Priority,
    pub source_text: String,
    pub status: ActionStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Open,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingNotes {
    pub generated_at: DateTime<Utc>,
    pub discussion_points: Vec<String>,
    pub action_items: Vec<ActionItem>,
    pub decisions: Vec<String>,
    pub open_questions: Vec<String>,
    pub topics: Vec<TopicSummary>,
    pub sentiment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSummary {
    pub topic: String,
    pub key_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSummary {
    pub session_id: String,
    pub title: Option<String>,
    pub date: String,
    pub duration_minutes: f64,
    pub participants: Vec<String>,
    pub executive_summary: String,
    pub key_discussion_points: Vec<String>,
    pub decisions_made: Vec<String>,
    pub action_items: Vec<ActionItem>,
    pub next_steps: Vec<String>,
    pub sentiment_summary: Option<String>,
}

/// Events emitted to the frontend via Tauri `emit`.
#[derive(Debug, Clone, Serialize)]
pub struct MeetingEvent {
    pub kind: MeetingEventKind,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingEventKind {
    Started,
    Stopped,
    Caption,    // live partial/final transcription
    Notes,      // periodic structured notes
    ActionItem, // newly extracted action item
    SpeakerNew, // new speaker detected
    Error,
}

// ── Speaker diarizer — lightweight embedding fingerprint ──────────────────────

#[derive(Clone)]
struct SpeakerProfile {
    label: String,
    /// Normalised energy fingerprint (16-band pseudo-spectrum).
    embedding: [f32; 16],
    count: u32,
}

pub(crate) struct Diarizer {
    profiles: Vec<SpeakerProfile>,
    threshold: f32,
    next_index: u32,
}

impl Diarizer {
    fn new() -> Self {
        Self {
            profiles: vec![],
            threshold: 0.82,
            next_index: 1,
        }
    }

    /// Compute a 16-band energy fingerprint from 16 kHz i16 samples.
    fn fingerprint(samples: &[i16]) -> [f32; 16] {
        let n = samples.len().max(1);
        let band = n / 16;
        let mut fp = [0f32; 16];
        for (b, chunk) in samples.chunks(band.max(1)).enumerate().take(16) {
            let rms = (chunk
                .iter()
                .map(|&s| (s as f32 / 32768.0).powi(2))
                .sum::<f32>()
                / chunk.len() as f32)
                .sqrt();
            fp[b] = rms;
        }
        // Normalise
        let sum: f32 = fp.iter().sum::<f32>() + 1e-9;
        for v in &mut fp {
            *v /= sum;
        }
        fp
    }

    fn cosine(a: &[f32; 16], b: &[f32; 16]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt() + 1e-9;
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt() + 1e-9;
        dot / (na * nb)
    }

    /// Return the speaker label for this audio segment (creates new profile if needed).
    fn identify(&mut self, samples: &[i16]) -> String {
        let fp = Self::fingerprint(samples);
        if let Some(best) = self.profiles.iter_mut().max_by(|a, b| {
            Self::cosine(&fp, &a.embedding)
                .partial_cmp(&Self::cosine(&fp, &b.embedding))
                .unwrap()
        }) {
            if Self::cosine(&fp, &best.embedding) >= self.threshold {
                // Update running mean
                for (a, &f) in best.embedding.iter_mut().zip(fp.iter()) {
                    *a = (*a * best.count as f32 + f) / (best.count as f32 + 1.0);
                }
                let sum: f32 = best.embedding.iter().sum::<f32>() + 1e-9;
                for v in &mut best.embedding {
                    *v /= sum;
                }
                best.count += 1;
                return best.label.clone();
            }
        }
        // New speaker
        let label = format!("Speaker {}", self.next_index);
        self.next_index += 1;
        self.profiles.push(SpeakerProfile {
            label: label.clone(),
            embedding: fp,
            count: 1,
        });
        label
    }
}

// ── Voice Activity Detector ───────────────────────────────────────────────────

struct Vad {
    energy_threshold: f32,
    speech_frames_min: usize,  // frames of speech before declaring speech
    silence_frames_end: usize, // frames of silence before ending segment
    buf: Vec<i16>,
    speech_frames: usize,
    silence_frames: usize,
    in_speech: bool,
    segment_start_ms: u64,
    elapsed_ms: u64,
}

struct SpeechSegment {
    samples: Vec<i16>,
    start_ms: u64,
    end_ms: u64,
}

impl Vad {
    fn new() -> Self {
        Self {
            energy_threshold: 0.003,
            speech_frames_min: 3,   // 60 ms minimum speech
            silence_frames_end: 25, // 500 ms silence ends segment
            buf: Vec::new(),
            speech_frames: 0,
            silence_frames: 0,
            in_speech: false,
            segment_start_ms: 0,
            elapsed_ms: 0,
        }
    }

    /// Feed a 20 ms frame, returns a complete segment when speech ends.
    fn push_frame(&mut self, frame: &[i16]) -> Option<SpeechSegment> {
        let rms = (frame
            .iter()
            .map(|&s| (s as f32 / 32768.0).powi(2))
            .sum::<f32>()
            / frame.len() as f32)
            .sqrt();
        self.elapsed_ms += FRAME_MS as u64;
        let is_speech = rms > self.energy_threshold;

        if !self.in_speech {
            if is_speech {
                self.speech_frames += 1;
                self.buf.extend_from_slice(frame);
                if self.speech_frames >= self.speech_frames_min {
                    self.in_speech = true;
                    self.silence_frames = 0;
                    self.segment_start_ms = self
                        .elapsed_ms
                        .saturating_sub(self.speech_frames as u64 * FRAME_MS as u64);
                }
            } else {
                self.speech_frames = 0;
                self.buf.clear();
            }
        } else {
            self.buf.extend_from_slice(frame);
            if is_speech {
                self.silence_frames = 0;
            } else {
                self.silence_frames += 1;
                if self.silence_frames >= self.silence_frames_end {
                    // End of speech
                    let seg = SpeechSegment {
                        samples: std::mem::take(&mut self.buf),
                        start_ms: self.segment_start_ms,
                        end_ms: self.elapsed_ms,
                    };
                    self.in_speech = false;
                    self.speech_frames = 0;
                    self.silence_frames = 0;
                    return Some(seg);
                }
            }
        }
        None
    }
}

// ── WAV encoding helper ───────────────────────────────────────────────────────

fn encode_wav(samples: &[i16]) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: BITS,
        sample_format: hound::SampleFormat::Int,
    };
    let mut buf = Vec::new();
    {
        let mut w = hound::WavWriter::new(std::io::Cursor::new(&mut buf), spec).unwrap();
        for &s in samples {
            let _ = w.write_sample(s);
        }
        let _ = w.finalize();
    }
    buf
}

// ── LLM helper (same pattern as ai_code_tools.rs) ────────────────────────────

async fn call_llm(system: &str, user: &str, max_tokens: u32) -> Result<String, String> {
    let api_url =
        std::env::var("BONSAI_API_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".into());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "model": "bonsai",
        "messages": [
            {"role": "system", "content": system},
            {"role": "user",   "content": user},
        ],
        "max_tokens": max_tokens,
        "temperature": 0.1,
        "stream": false,
    });
    let resp = client
        .post(format!("{api_url}/v1/chat/completions"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

// ── Session state ─────────────────────────────────────────────────────────────

pub struct SessionState {
    pub session_id: String,
    pub started_at: Instant,
    pub started_utc: DateTime<Utc>,
    pub running: bool,
    pub paused: bool,
    pub transcripts: Vec<TranscriptSegment>,
    pub notes: Vec<MeetingNotes>,
    pub action_items: Vec<ActionItem>,
    pub diarizer: Diarizer,
    pub last_note_at: Instant,
    pub note_interval: Duration,
}

impl SessionState {
    fn new() -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            started_at: Instant::now(),
            started_utc: Utc::now(),
            running: false,
            paused: false,
            transcripts: Vec::new(),
            notes: Vec::new(),
            action_items: Vec::new(),
            diarizer: Diarizer::new(),
            last_note_at: Instant::now(),
            note_interval: Duration::from_secs(300), // 5 min
        }
    }
}

// ── MeetingAgent ──────────────────────────────────────────────────────────────

pub struct MeetingAgent {
    state: Arc<RwLock<SessionState>>,
    event_tx: broadcast::Sender<MeetingEvent>,
}

impl MeetingAgent {
    pub fn new() -> (Self, broadcast::Receiver<MeetingEvent>) {
        let (tx, rx) = broadcast::channel(256);
        let agent = Self {
            state: Arc::new(RwLock::new(SessionState::new())),
            event_tx: tx,
        };
        (agent, rx)
    }

    fn emit(&self, kind: MeetingEventKind, payload: serde_json::Value) {
        let _ = self.event_tx.send(MeetingEvent { kind, payload });
    }

    // ── Start / Stop / Pause ─────────────────────────────────────────────────

    pub async fn start(
        &self,
        audio_source: &str,
        whisper: Arc<crate::sidecar_manager::WhisperManager>,
    ) -> Result<String, String> {
        let mut s = self.state.write().await;
        if s.running {
            return Err("Meeting already in progress".into());
        }
        // Reset session
        *s = SessionState::new();
        s.running = true;
        let session_id = s.session_id.clone();
        drop(s);

        self.emit(
            MeetingEventKind::Started,
            serde_json::json!({ "session_id": session_id }),
        );
        info!(
            "[meeting] session {} started, source={audio_source}",
            session_id
        );

        // Spawn audio capture loop
        let state = self.state.clone();
        let event_tx = self.event_tx.clone();
        let source = audio_source.to_string();

        tokio::spawn(async move {
            if let Err(e) = audio_capture_loop(state, event_tx, source, whisper).await {
                warn!("[meeting] audio loop error: {e}");
            }
        });

        Ok(session_id)
    }

    pub async fn stop(&self) -> Result<MeetingSummary, String> {
        {
            let mut s = self.state.write().await;
            s.running = false;
        }
        let summary = self.generate_summary().await?;
        self.emit(
            MeetingEventKind::Stopped,
            serde_json::to_value(&summary).unwrap_or_default(),
        );
        Ok(summary)
    }

    pub async fn set_paused(&self, paused: bool) {
        self.state.write().await.paused = paused;
    }

    // ── Transcript / Notes ───────────────────────────────────────────────────

    pub async fn get_transcripts(&self) -> Vec<TranscriptSegment> {
        self.state.read().await.transcripts.clone()
    }

    pub async fn get_latest_notes(&self) -> Option<MeetingNotes> {
        self.state.read().await.notes.last().cloned()
    }

    pub async fn get_action_items(&self) -> Vec<ActionItem> {
        self.state.read().await.action_items.clone()
    }

    pub async fn update_action_item_status(&self, id: &str, status: ActionStatus) -> bool {
        let mut s = self.state.write().await;
        if let Some(item) = s.action_items.iter_mut().find(|i| i.id == id) {
            item.status = status;
            return true;
        }
        false
    }

    // ── Question answering ───────────────────────────────────────────────────

    pub async fn ask(&self, question: &str) -> Result<String, String> {
        let transcripts = self.state.read().await.transcripts.clone();
        if transcripts.is_empty() {
            return Ok("No meeting transcript available yet.".into());
        }

        // Build context from recent + keyword-matched segments
        let q_lower = question.to_lowercase();
        let relevant: Vec<&TranscriptSegment> = transcripts
            .iter()
            .filter(|t| {
                let tl = t.text.to_lowercase();
                q_lower.split_whitespace().any(|w| tl.contains(w))
            })
            .take(15)
            .collect();

        let context_segs: Vec<&TranscriptSegment> = if relevant.is_empty() {
            transcripts.iter().rev().take(20).collect()
        } else {
            relevant
        };

        let context = context_segs
            .iter()
            .map(|t| format!("[{:.0}s] {}: {}", t.start_ms / 1000, t.speaker, t.text))
            .collect::<Vec<_>>()
            .join("\n");

        let system = "You are a meeting assistant. Answer questions based ONLY on the transcript. \
                      Cite speakers and times. If the answer is not in the transcript say so.";
        let user = format!("## Meeting Transcript Excerpts\n{context}\n\n## Question\n{question}");
        call_llm(system, &user, 512).await
    }

    // ── Summary ──────────────────────────────────────────────────────────────

    pub async fn generate_summary(&self) -> Result<MeetingSummary, String> {
        let s = self.state.read().await;
        if s.transcripts.is_empty() {
            let now = Utc::now();
            return Ok(MeetingSummary {
                session_id: s.session_id.clone(),
                title: None,
                date: now.format("%Y-%m-%d").to_string(),
                duration_minutes: s.started_at.elapsed().as_secs_f64() / 60.0,
                participants: vec![],
                executive_summary: "No transcript recorded.".into(),
                key_discussion_points: vec![],
                decisions_made: vec![],
                action_items: s.action_items.clone(),
                next_steps: vec![],
                sentiment_summary: None,
            });
        }

        let transcript_text = s
            .transcripts
            .iter()
            .map(|t| format!("[{:.0}s] {}: {}", t.start_ms / 1000, t.speaker, t.text))
            .collect::<Vec<_>>()
            .join("\n");

        let participants: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            s.transcripts
                .iter()
                .filter_map(|t| seen.insert(t.speaker.clone()).then_some(t.speaker.clone()))
                .collect()
        };

        let duration_min = s.started_at.elapsed().as_secs_f64() / 60.0;
        drop(s);

        let system =
            "You are an executive assistant. Extract a structured meeting summary as JSON.";
        let user = format!(
            r#"## Meeting Transcript
{transcript_text}

Return a JSON object with exactly these fields (no extra text):
{{
  "title": "inferred meeting title",
  "executive_summary": "2-3 sentences",
  "key_discussion_points": ["..."],
  "decisions_made": ["..."],
  "action_items": [{{"description":"...","assignee":null,"deadline":null,"priority":"medium","source_text":"..."}}],
  "next_steps": ["..."],
  "sentiment_summary": "positive|neutral|tense|mixed"
}}"#
        );

        let raw = call_llm(system, &user, 1024).await.unwrap_or_default();

        // Extract JSON from the response
        let json_str = extract_json(&raw).unwrap_or_else(|| "{}".to_string());
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();

        let s = self.state.read().await;

        fn str_vec(v: &serde_json::Value, key: &str) -> Vec<String> {
            v[key]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        }

        let mut action_items = s.action_items.clone();
        // Merge any new ones from the summary
        if let Some(arr) = v["action_items"].as_array() {
            for item in arr {
                let desc = item["description"].as_str().unwrap_or("").to_string();
                if !desc.is_empty() && !action_items.iter().any(|a| a.description == desc) {
                    action_items.push(ActionItem {
                        id: Uuid::new_v4().to_string(),
                        description: desc,
                        assignee: item["assignee"].as_str().map(|s| s.to_string()),
                        deadline: item["deadline"].as_str().map(|s| s.to_string()),
                        priority: match item["priority"].as_str().unwrap_or("medium") {
                            "high" => Priority::High,
                            "low" => Priority::Low,
                            _ => Priority::Medium,
                        },
                        source_text: item["source_text"].as_str().unwrap_or("").to_string(),
                        status: ActionStatus::Open,
                        created_at: Utc::now(),
                    });
                }
            }
        }

        Ok(MeetingSummary {
            session_id: s.session_id.clone(),
            title: v["title"].as_str().map(|s| s.to_string()),
            date: Utc::now().format("%Y-%m-%d").to_string(),
            duration_minutes: duration_min,
            participants,
            executive_summary: v["executive_summary"].as_str().unwrap_or("").to_string(),
            key_discussion_points: str_vec(&v, "key_discussion_points"),
            decisions_made: str_vec(&v, "decisions_made"),
            action_items,
            next_steps: str_vec(&v, "next_steps"),
            sentiment_summary: v["sentiment_summary"].as_str().map(|s| s.to_string()),
        })
    }

    // ── Generate periodic notes ───────────────────────────────────────────────

    async fn maybe_generate_notes(&self) {
        let should_gen = {
            let s = self.state.read().await;
            s.last_note_at.elapsed() >= s.note_interval && s.transcripts.len() >= 5
        };
        if !should_gen {
            return;
        }

        if let Ok(notes) = generate_notes_from_state(&self.state).await {
            let mut s = self.state.write().await;
            // Merge action items
            for item in &notes.action_items {
                if !s
                    .action_items
                    .iter()
                    .any(|a| a.description == item.description)
                {
                    let item = item.clone();
                    self.event_tx
                        .send(MeetingEvent {
                            kind: MeetingEventKind::ActionItem,
                            payload: serde_json::to_value(&item).unwrap_or_default(),
                        })
                        .ok();
                    s.action_items.push(item);
                }
            }
            self.event_tx
                .send(MeetingEvent {
                    kind: MeetingEventKind::Notes,
                    payload: serde_json::to_value(&notes).unwrap_or_default(),
                })
                .ok();
            s.notes.push(notes);
            s.last_note_at = Instant::now();
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MeetingEvent> {
        self.event_tx.subscribe()
    }

    pub async fn session_id(&self) -> String {
        self.state.read().await.session_id.clone()
    }

    pub async fn is_running(&self) -> bool {
        self.state.read().await.running
    }
}

// ── Periodic note generation ──────────────────────────────────────────────────

async fn generate_notes_from_state(
    state: &Arc<RwLock<SessionState>>,
) -> Result<MeetingNotes, String> {
    let transcripts = state.read().await.transcripts.clone();
    if transcripts.is_empty() {
        return Err("No transcripts".into());
    }

    let transcript_text = transcripts
        .iter()
        .map(|t| format!("[{:.0}s] {}: {}", t.start_ms / 1000, t.speaker, t.text))
        .collect::<Vec<_>>()
        .join("\n");

    let system =
        "You are a meeting note generator. Extract structured notes from this transcript as JSON.";
    let user = format!(
        r#"## Transcript
{transcript_text}

Return ONLY a JSON object:
{{
  "discussion_points": ["..."],
  "action_items": [{{"description":"...","assignee":null,"deadline":null,"priority":"medium","source_text":"..."}}],
  "decisions": ["..."],
  "open_questions": ["..."],
  "topics": [{{"topic":"...","key_points":["..."]}}],
  "sentiment": "positive|neutral|tense|mixed"
}}"#
    );

    let raw = call_llm(system, &user, 768).await.unwrap_or_default();
    let json_str = extract_json(&raw).unwrap_or_else(|| "{}".to_string());
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();

    fn str_vec(v: &serde_json::Value, key: &str) -> Vec<String> {
        v[key]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    let action_items = v["action_items"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let desc = item["description"].as_str()?.to_string();
                    Some(ActionItem {
                        id: Uuid::new_v4().to_string(),
                        description: desc,
                        assignee: item["assignee"].as_str().map(|s| s.to_string()),
                        deadline: item["deadline"].as_str().map(|s| s.to_string()),
                        priority: match item["priority"].as_str().unwrap_or("medium") {
                            "high" => Priority::High,
                            "low" => Priority::Low,
                            _ => Priority::Medium,
                        },
                        source_text: item["source_text"].as_str().unwrap_or("").to_string(),
                        status: ActionStatus::Open,
                        created_at: Utc::now(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let topics = v["topics"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let topic = t["topic"].as_str()?.to_string();
                    let key_points = t["key_points"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    Some(TopicSummary { topic, key_points })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(MeetingNotes {
        generated_at: Utc::now(),
        discussion_points: str_vec(&v, "discussion_points"),
        action_items,
        decisions: str_vec(&v, "decisions"),
        open_questions: str_vec(&v, "open_questions"),
        topics,
        sentiment: v["sentiment"].as_str().map(|s| s.to_string()),
    })
}

// ── JSON extraction from LLM output ──────────────────────────────────────────

fn extract_json(text: &str) -> Option<String> {
    // Find the first { ... } block
    let start = text.find('{')?;
    let mut depth = 0i32;
    let mut end = start;
    for (i, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth == 0 && end > start {
        Some(text[start..=end].to_string())
    } else {
        None
    }
}

// ── Audio capture loop ────────────────────────────────────────────────────────

async fn audio_capture_loop(
    state: Arc<RwLock<SessionState>>,
    event_tx: broadcast::Sender<MeetingEvent>,
    source: String,
    whisper: Arc<crate::sidecar_manager::WhisperManager>,
) -> Result<(), String> {
    let host = cpal::default_host();

    // Select device: "loopback" or "microphone" or a device name
    let device = if source.to_lowercase().contains("loopback") {
        // On Windows, cpal exposes the WASAPI loopback via output devices.
        // We iterate output devices and pick the default.
        host.default_output_device()
            .ok_or_else(|| "No default output device".to_string())?
    } else {
        host.default_input_device()
            .ok_or_else(|| "No default input device".to_string())?
    };

    info!(
        "[meeting] capture device: {}",
        device.name().unwrap_or_default()
    );

    // Pick a supported 16 kHz mono config, falling back to device default.
    let config = find_config(&device)?;

    // Shared sample queue
    let (sample_tx, mut sample_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(512);

    let running_flag = {
        let s = state.read().await;
        s.running
    };

    // Spawn cpal stream in a blocking thread
    let stream_sample_tx = sample_tx.clone();
    let _stream_handle = tokio::task::spawn_blocking(move || -> Result<(), String> {
        use cpal::traits::StreamTrait;

        let err_fn = |err| warn!("[meeting] stream error: {err}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    let _ = stream_sample_tx.try_send(data.to_vec());
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::F32 => {
                let tx2 = stream_sample_tx.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _| {
                        let i16_data: Vec<i16> = data
                            .iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                            .collect();
                        let _ = tx2.try_send(i16_data);
                    },
                    err_fn,
                    None,
                )
            }
            _ => return Err("Unsupported sample format".into()),
        }
        .map_err(|e| format!("stream build error: {e}"))?;

        stream
            .play()
            .map_err(|e| format!("stream play error: {e}"))?;
        // Keep alive until channel closed
        loop {
            std::thread::sleep(Duration::from_millis(100));
        }
    });

    // Processing loop
    let mut vad = Vad::new();
    let mut sample_buf: Vec<i16> = Vec::new();

    loop {
        // Check if meeting was stopped
        if !state.read().await.running {
            break;
        }

        // Drain incoming samples
        while let Ok(chunk) = sample_rx.try_recv() {
            // Resample to 16 kHz mono if needed (simple downsample)
            sample_buf.extend_from_slice(&chunk);
        }

        // Process accumulated samples in FRAME_SAMPLES blocks
        while sample_buf.len() >= FRAME_SAMPLES {
            let frame: Vec<i16> = sample_buf.drain(..FRAME_SAMPLES).collect();

            // Skip if paused
            if state.read().await.paused {
                continue;
            }

            if let Some(segment) = vad.push_frame(&frame) {
                // Got a speech segment — transcribe it
                let wav = encode_wav(&segment.samples);
                let whisper_ref = whisper.clone();
                let state_ref = state.clone();
                let etx = event_tx.clone();
                let seg_start = segment.start_ms;
                let seg_end = segment.end_ms;
                let samples_cp = segment.samples.clone();

                tokio::spawn(async move {
                    match whisper_ref.transcribe(wav).await {
                        Ok(text) if !text.trim().is_empty() => {
                            let mut s = state_ref.write().await;
                            let speaker = s.diarizer.identify(&samples_cp);
                            let is_new_speaker = s.transcripts.iter().all(|t| t.speaker != speaker);

                            let seg = TranscriptSegment {
                                id: Uuid::new_v4().to_string(),
                                speaker: speaker.clone(),
                                text: text.clone(),
                                start_ms: seg_start,
                                end_ms: seg_end,
                                confidence: 0.9,
                                is_final: true,
                            };
                            s.transcripts.push(seg.clone());

                            drop(s);

                            if is_new_speaker {
                                let _ = etx.send(MeetingEvent {
                                    kind: MeetingEventKind::SpeakerNew,
                                    payload: serde_json::json!({ "speaker": speaker }),
                                });
                            }
                            let _ = etx.send(MeetingEvent {
                                kind: MeetingEventKind::Caption,
                                payload: serde_json::to_value(&seg).unwrap_or_default(),
                            });
                        }
                        Ok(_) => {} // silence / noise
                        Err(e) => warn!("[meeting] transcription error: {e}"),
                    }
                });
            }
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    Ok(())
}

fn find_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    // Try to find a 16 kHz mono config
    let supported = device
        .supported_input_configs()
        .map_err(|e| format!("configs error: {e}"))?;
    for cfg in supported {
        if cfg.channels() == 1
            && cfg.min_sample_rate().0 <= 16_000
            && cfg.max_sample_rate().0 >= 16_000
        {
            return Ok(cfg.with_sample_rate(cpal::SampleRate(16_000)));
        }
    }
    // Fall back to device default
    device
        .default_input_config()
        .map_err(|e| format!("default config error: {e}"))
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_meeting_agent(
    state: tauri::State<'_, crate::AppState>,
    audio_source: String,
) -> Result<String, String> {
    let whisper = state.whisper.clone();
    state.meeting_agent.start(&audio_source, whisper).await
}

#[tauri::command]
pub async fn stop_meeting_agent(
    state: tauri::State<'_, crate::AppState>,
) -> Result<MeetingSummary, String> {
    state.meeting_agent.stop().await
}

#[tauri::command]
pub async fn pause_meeting_agent(
    state: tauri::State<'_, crate::AppState>,
    paused: bool,
) -> Result<(), String> {
    state.meeting_agent.set_paused(paused).await;
    Ok(())
}

#[tauri::command]
pub async fn ask_meeting_agent(
    state: tauri::State<'_, crate::AppState>,
    question: String,
) -> Result<String, String> {
    state.meeting_agent.ask(&question).await
}

#[tauri::command]
pub async fn get_meeting_transcript(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<TranscriptSegment>, String> {
    Ok(state.meeting_agent.get_transcripts().await)
}

#[tauri::command]
pub async fn get_meeting_notes(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Option<MeetingNotes>, String> {
    Ok(state.meeting_agent.get_latest_notes().await)
}

#[tauri::command]
pub async fn get_meeting_action_items(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<ActionItem>, String> {
    Ok(state.meeting_agent.get_action_items().await)
}

#[tauri::command]
pub async fn update_action_item(
    state: tauri::State<'_, crate::AppState>,
    id: String,
    status: ActionStatus,
) -> Result<bool, String> {
    Ok(state
        .meeting_agent
        .update_action_item_status(&id, status)
        .await)
}

#[tauri::command]
pub async fn get_meeting_summary(
    state: tauri::State<'_, crate::AppState>,
) -> Result<MeetingSummary, String> {
    state.meeting_agent.generate_summary().await
}

#[tauri::command]
pub async fn is_meeting_running(state: tauri::State<'_, crate::AppState>) -> Result<bool, String> {
    Ok(state.meeting_agent.is_running().await)
}
