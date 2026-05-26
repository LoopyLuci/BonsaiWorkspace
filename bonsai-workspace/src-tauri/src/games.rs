//! Chess and Go game session management with Tauri commands, GamePlayer trait,
//! tournament support, and daily puzzles.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;
use crate::AppState;

use bonsai_chess::{
    ChessGameSession, Player as ChessPlayer, PlayerKind as ChessPlayerKind,
    ChessColor, GameResult as ChessGameResult,
    MaterialEvaluator, MctsConfig, search as chess_search, ChessPosition,
};
use bonsai_go::{
    GoGameSession, GoPlayer, GoPlayerKind, GoColor, GoGameResult,
    Stone, GoMctsConfig, go_search, mcts::RandomGoEvaluator,
};

// ── Chat-embeddable game state ─────────────────────────────────────────────────

/// Serialized game state embedded in a chat message for inline board rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGameState {
    pub game_type:    String,           // "chess" | "go"
    pub session_id:   String,
    pub position:     String,           // FEN for chess; JSON array for Go
    pub last_move:    Option<String>,   // UCI / GTP
    pub legal_moves:  Vec<String>,
    pub turn:         String,           // "white" | "black"
    pub orientation:  String,           // board orientation for the viewer
    pub interactive:  bool,
    pub result:       String,
    pub board_size:   Option<u8>,
    pub score_estimate: Option<f32>,
}

// ── Session stores ─────────────────────────────────────────────────────────────

pub struct GameSessionStore {
    pub chess:       RwLock<HashMap<Uuid, ChessGameSession>>,
    pub go:          RwLock<HashMap<Uuid, GoGameSession>>,
    pub tournaments: TournamentManager,
    pub puzzles:     PuzzleStore,
}

impl GameSessionStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            chess:       RwLock::new(HashMap::new()),
            go:          RwLock::new(HashMap::new()),
            tournaments: TournamentManager { tournaments: RwLock::new(HashMap::new()) },
            puzzles:     PuzzleStore::build(),
        })
    }
}

// ── Request / Response DTOs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateChessGameRequest {
    pub human_name: String,
    pub human_color: String, // "white" | "black"
    pub ai_strength: Option<String>, // "interactive" | "strong" | "training"
}

#[derive(Debug, Serialize)]
pub struct ChessGameView {
    pub id: String,
    pub fen: String,
    pub pgn: String,
    pub legal_moves: Vec<String>,
    pub current_player_id: String,
    pub result: String,
    pub white_name: String,
    pub black_name: String,
    pub move_count: usize,
    pub opening_name: Option<String>,
}

impl ChessGameView {
    fn from_session(s: &ChessGameSession) -> Self {
        let result = match &s.result {
            ChessGameResult::WhiteWins(_) => "1-0",
            ChessGameResult::BlackWins(_) => "0-1",
            ChessGameResult::Draw(_)      => "1/2-1/2",
            ChessGameResult::Ongoing      => "*",
        };
        Self {
            id: s.id.to_string(),
            fen: s.current_fen.clone(),
            pgn: s.pgn.clone(),
            legal_moves: s.legal_moves_uci(),
            current_player_id: s.current_player().id.clone(),
            result: result.to_string(),
            white_name: s.white.name.clone(),
            black_name: s.black.name.clone(),
            move_count: s.moves.len(),
            opening_name: s.opening_name.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGoGameRequest {
    pub human_name: String,
    pub human_color: String, // "black" | "white"
    pub board_size: Option<u8>,  // 9 | 13 | 19
    pub komi: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct GoGameView {
    pub id: String,
    pub size: u8,
    pub stones: Vec<StoneView>,
    pub current_player_id: String,
    pub current_color: String,
    pub result: String,
    pub black_name: String,
    pub white_name: String,
    pub black_captures: u32,
    pub white_captures: u32,
    pub komi: f32,
    pub move_count: usize,
    pub score_estimate: f32,
}

#[derive(Debug, Serialize)]
pub struct StoneView {
    pub x: u8,
    pub y: u8,
    pub color: String,
}

impl GoGameView {
    fn from_session(s: &GoGameSession) -> Self {
        let result = match &s.result {
            GoGameResult::BlackWins { margin } => format!("B+{:.1}", margin),
            GoGameResult::WhiteWins { margin } => format!("W+{:.1}", margin),
            GoGameResult::Draw => "Draw".into(),
            GoGameResult::Ongoing => "*".into(),
        };
        let current_color = match s.current_color() {
            GoColor::Black => "black",
            GoColor::White => "white",
        };
        let stones: Vec<StoneView> = s.board.stones.iter()
            .map(|(pt, stone)| StoneView {
                x: pt.x, y: pt.y,
                color: match stone { Stone::Black => "black".into(), Stone::White => "white".into() },
            })
            .collect();

        Self {
            id: s.id.to_string(),
            size: s.size,
            stones,
            current_player_id: s.current_player().id.clone(),
            current_color: current_color.into(),
            result,
            black_name: s.black.name.clone(),
            white_name: s.white.name.clone(),
            black_captures: s.board.black_captures,
            white_captures: s.board.white_captures,
            komi: s.komi,
            move_count: s.moves.len(),
            score_estimate: s.current_score(),
        }
    }
}

// ── Tauri commands: Chess ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_chess_game(
    req: CreateChessGameRequest,
    store: State<'_, AppState>,
) -> Result<ChessGameView, String> {
    let (white, black) = if req.human_color.to_lowercase() == "white" {
        let human = ChessPlayer { id: "user".into(), name: req.human_name.clone(), kind: ChessPlayerKind::Human, color: ChessColor::White, elo: None };
        let ai    = ChessPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: ChessPlayerKind::BonsAI, color: ChessColor::Black, elo: None };
        (human, ai)
    } else {
        let ai    = ChessPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: ChessPlayerKind::BonsAI, color: ChessColor::White, elo: None };
        let human = ChessPlayer { id: "user".into(), name: req.human_name.clone(), kind: ChessPlayerKind::Human, color: ChessColor::Black, elo: None };
        (ai, human)
    };

    let mut session = ChessGameSession::new(white, black);

    // If AI plays white, make its first move immediately
    let view = if session.needs_ai_move() {
        make_chess_ai_move_inner(&mut session, req.ai_strength.as_deref());
        ChessGameView::from_session(&session)
    } else {
        ChessGameView::from_session(&session)
    };

    store.game_sessions.chess.write().await.insert(session.id, session);
    Ok(view)
}

#[tauri::command]
pub async fn make_chess_move(
    game_id: String,
    notation: String,
    store: State<'_, AppState>,
) -> Result<ChessGameView, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id".to_string())?;
    let mut sessions = store.game_sessions.chess.write().await;
    let session = sessions.get_mut(&id).ok_or("game not found")?;

    session.apply_move("user", &notation).map_err(|e| e.to_string())?;

    // AI responds if needed
    if session.needs_ai_move() {
        make_chess_ai_move_inner(session, None);
    }

    Ok(ChessGameView::from_session(session))
}

#[tauri::command]
pub async fn get_chess_game(
    game_id: String,
    store: State<'_, AppState>,
) -> Result<ChessGameView, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id".to_string())?;
    let sessions = store.game_sessions.chess.read().await;
    let session = sessions.get(&id).ok_or("game not found")?;
    Ok(ChessGameView::from_session(session))
}

#[tauri::command]
pub async fn resign_chess_game(
    game_id: String,
    store: State<'_, AppState>,
) -> Result<ChessGameView, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id".to_string())?;
    let mut sessions = store.game_sessions.chess.write().await;
    let session = sessions.get_mut(&id).ok_or("game not found")?;
    session.resign("user");
    Ok(ChessGameView::from_session(session))
}

#[tauri::command]
pub async fn list_chess_games(
    store: State<'_, AppState>,
) -> Result<Vec<ChessGameView>, String> {
    let sessions = store.game_sessions.chess.read().await;
    Ok(sessions.values().map(ChessGameView::from_session).collect())
}

fn make_chess_ai_move_inner(session: &mut ChessGameSession, strength: Option<&str>) {
    let config = match strength {
        Some("strong") => MctsConfig::strong(),
        Some("training") => MctsConfig::training(),
        _ => MctsConfig::interactive(),
    };
    if let Ok(pos) = ChessPosition::from_fen(&session.current_fen) {
        let eval = MaterialEvaluator;
        let result = chess_search(&pos, &eval, &config);
        if !result.best_move.is_empty() {
            let _ = session.apply_move("bonsai", &result.best_move);
        }
    }
}

/// Chess AI move with streaming thinking events emitted via AppHandle.
/// Emits: `agent-thinking-started`, `agent-thinking-complete`.
pub async fn make_chess_ai_move_with_events(
    session: &mut ChessGameSession,
    strength: Option<&str>,
    app: &tauri::AppHandle,
) {
    use tauri::Emitter;
    let config = match strength {
        Some("strong") => MctsConfig::strong(),
        Some("training") => MctsConfig::training(),
        _ => MctsConfig::interactive(),
    };

    let _ = app.emit("agent-thinking-started", serde_json::json!({
        "session_id": session.id.to_string(),
        "agent": "BonsAI",
        "game_type": "chess",
    }));

    if let Ok(pos) = ChessPosition::from_fen(&session.current_fen) {
        let eval = MaterialEvaluator;
        let result = chess_search(&pos, &eval, &config);
        if !result.best_move.is_empty() {
            let pct = (result.value * 100.0) as i32;
            let top_moves: Vec<String> = result.move_probs.iter().take(5)
                .map(|(m, p)| format!("{} ({:.0}%)", m, p * 100.0))
                .collect();
            let _ = app.emit("agent-thinking-complete", serde_json::json!({
                "session_id": session.id.to_string(),
                "agent": "BonsAI",
                "best_move": result.best_move,
                "value_pct": pct,
                "simulations": result.simulations,
                "top_moves": top_moves,
            }));
            let _ = session.apply_move("bonsai", &result.best_move);
        }
    }
}

/// Go AI move with streaming thinking events.
pub async fn make_go_ai_move_with_events(
    session: &mut bonsai_go::GoGameSession,
    app: &tauri::AppHandle,
) {
    use tauri::Emitter;
    let color = session.current_color().to_stone();
    let config = GoMctsConfig::interactive();
    let eval = RandomGoEvaluator;

    let _ = app.emit("agent-thinking-started", serde_json::json!({
        "session_id": session.id.to_string(),
        "agent": "BonsAI",
        "game_type": "go",
    }));

    let result = go_search(&session.board, color, &eval, &config);
    let gtp = if result.best_move.is_empty() { "pass".to_string() } else { result.best_move.clone() };

    let _ = app.emit("agent-thinking-complete", serde_json::json!({
        "session_id": session.id.to_string(),
        "agent": "BonsAI",
        "best_move": gtp,
        "value": result.value,
        "simulations": result.simulations,
    }));

    let _ = session.play("bonsai", &gtp);
}

/// Tauri command: make an AI move for a chess game (with thinking events).
#[tauri::command]
pub async fn chess_ai_move(
    game_id: String,
    app: tauri::AppHandle,
    store: State<'_, AppState>,
) -> Result<ChessGameView, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id")?;
    let mut sessions = store.game_sessions.chess.write().await;
    let session = sessions.get_mut(&id).ok_or("game not found")?;
    make_chess_ai_move_with_events(session, None, &app).await;
    Ok(ChessGameView::from_session(session))
}

/// Tauri command: make an AI move for a Go game (with thinking events).
#[tauri::command]
pub async fn go_ai_move(
    game_id: String,
    app: tauri::AppHandle,
    store: State<'_, AppState>,
) -> Result<GoGameView, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id")?;
    let mut sessions = store.game_sessions.go.write().await;
    let session = sessions.get_mut(&id).ok_or("game not found")?;
    make_go_ai_move_with_events(session, &app).await;
    Ok(GoGameView::from_session(session))
}

/// Tauri command: export a chess game as PGN.
#[tauri::command]
pub async fn export_chess_pgn(
    game_id: String,
    store: State<'_, AppState>,
) -> Result<String, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id")?;
    let sessions = store.game_sessions.chess.read().await;
    let s = sessions.get(&id).ok_or("game not found")?;
    Ok(s.pgn.clone())
}

/// Tauri command: spectate a game — returns current state and subscribes via events.
#[tauri::command]
pub async fn spectate_game(
    game_id: String,
    game_type: String,
    store: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    match game_type.as_str() {
        "chess" => {
            let id: Uuid = game_id.parse().map_err(|_| "invalid game id")?;
            let sessions = store.game_sessions.chess.read().await;
            let s = sessions.get(&id).ok_or("game not found")?;
            let state = chess_to_chat_state(s, false, "white");
            Ok(serde_json::to_value(state).unwrap_or_default())
        }
        "go" => {
            let id: Uuid = game_id.parse().map_err(|_| "invalid game id")?;
            let sessions = store.game_sessions.go.read().await;
            let s = sessions.get(&id).ok_or("game not found")?;
            let state = go_to_chat_state(s, false, "black");
            Ok(serde_json::to_value(state).unwrap_or_default())
        }
        _ => Err("unknown game type".into()),
    }
}

// ── Tauri commands: Go ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_go_game(
    req: CreateGoGameRequest,
    store: State<'_, AppState>,
) -> Result<GoGameView, String> {
    let size = req.board_size.unwrap_or(19);
    let komi = req.komi.unwrap_or(7.5);

    let (black, white) = if req.human_color.to_lowercase() == "black" {
        let human = GoPlayer { id: "user".into(), name: req.human_name.clone(), kind: GoPlayerKind::Human, color: GoColor::Black, rank: None };
        let ai    = GoPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: GoPlayerKind::BonsAI, color: GoColor::White, rank: None };
        (human, ai)
    } else {
        let ai    = GoPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: GoPlayerKind::BonsAI, color: GoColor::Black, rank: None };
        let human = GoPlayer { id: "user".into(), name: req.human_name.clone(), kind: GoPlayerKind::Human, color: GoColor::White, rank: None };
        (ai, human)
    };

    let mut session = GoGameSession::with_options(black, white, size, komi);

    // If AI plays black (first move)
    if session.needs_ai_move() {
        make_go_ai_move_inner(&mut session);
    }

    let view = GoGameView::from_session(&session);
    store.game_sessions.go.write().await.insert(session.id, session);
    Ok(view)
}

#[tauri::command]
pub async fn make_go_move(
    game_id: String,
    gtp_coord: String,
    store: State<'_, AppState>,
) -> Result<GoGameView, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id".to_string())?;
    let mut sessions = store.game_sessions.go.write().await;
    let session = sessions.get_mut(&id).ok_or("game not found")?;

    session.play("user", &gtp_coord).map_err(|e| e.to_string())?;

    if session.needs_ai_move() {
        make_go_ai_move_inner(session);
    }

    Ok(GoGameView::from_session(session))
}

#[tauri::command]
pub async fn get_go_game(
    game_id: String,
    store: State<'_, AppState>,
) -> Result<GoGameView, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id".to_string())?;
    let sessions = store.game_sessions.go.read().await;
    let session = sessions.get(&id).ok_or("game not found")?;
    Ok(GoGameView::from_session(session))
}

#[tauri::command]
pub async fn resign_go_game(
    game_id: String,
    store: State<'_, AppState>,
) -> Result<GoGameView, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id".to_string())?;
    let mut sessions = store.game_sessions.go.write().await;
    let session = sessions.get_mut(&id).ok_or("game not found")?;
    session.resign("user");
    Ok(GoGameView::from_session(session))
}

#[tauri::command]
pub async fn list_go_games(
    store: State<'_, AppState>,
) -> Result<Vec<GoGameView>, String> {
    let sessions = store.game_sessions.go.read().await;
    Ok(sessions.values().map(GoGameView::from_session).collect())
}

#[tauri::command]
pub async fn export_go_sgf(
    game_id: String,
    store: State<'_, AppState>,
) -> Result<String, String> {
    let id: Uuid = game_id.parse().map_err(|_| "invalid game id".to_string())?;
    let sessions = store.game_sessions.go.read().await;
    let session = sessions.get(&id).ok_or("game not found")?;
    Ok(session.to_sgf())
}

fn make_go_ai_move_inner(session: &mut GoGameSession) {
    let color = session.current_color().to_stone();
    let config = GoMctsConfig::interactive();
    let eval = RandomGoEvaluator;
    let result = go_search(&session.board, color, &eval, &config);
    let gtp = if result.best_move.is_empty() { "pass".to_string() } else { result.best_move };
    let _ = session.play("bonsai", &gtp);
}

// ── ChatGameState builders ─────────────────────────────────────────────────────

pub fn chess_to_chat_state(s: &ChessGameSession, interactive: bool, viewer_color: &str) -> ChatGameState {
    let result = match &s.result {
        ChessGameResult::WhiteWins(_) => "1-0",
        ChessGameResult::BlackWins(_) => "0-1",
        ChessGameResult::Draw(_) => "1/2-1/2",
        ChessGameResult::Ongoing => "*",
    };
    let turn = if s.moves.len() % 2 == 0 { "white" } else { "black" };
    let last_move = s.moves.last().map(|m| m.uci.clone());
    ChatGameState {
        game_type:     "chess".into(),
        session_id:    s.id.to_string(),
        position:      s.current_fen.clone(),
        last_move,
        legal_moves:   if interactive { s.legal_moves_uci() } else { vec![] },
        turn:          turn.into(),
        orientation:   viewer_color.into(),
        interactive,
        result:        result.into(),
        board_size:    None,
        score_estimate: None,
    }
}

pub fn go_to_chat_state(s: &GoGameSession, interactive: bool, viewer_color: &str) -> ChatGameState {
    let result = match &s.result {
        GoGameResult::BlackWins { margin } => format!("B+{:.1}", margin),
        GoGameResult::WhiteWins { margin } => format!("W+{:.1}", margin),
        GoGameResult::Draw => "Draw".into(),
        GoGameResult::Ongoing => "*".into(),
    };
    let turn = match s.current_color() {
        GoColor::Black => "black",
        GoColor::White => "white",
    };
    let stones_json = serde_json::to_string(&s.board.stones.iter().map(|(p, st)| {
        serde_json::json!({"x": p.x, "y": p.y, "color": match st { Stone::Black => "black", Stone::White => "white" }})
    }).collect::<Vec<_>>()).unwrap_or_default();
    let last_move = s.moves.last().map(|m| m.gtp.clone());
    ChatGameState {
        game_type:      "go".into(),
        session_id:     s.id.to_string(),
        position:       stones_json,
        last_move,
        legal_moves:    vec![],
        turn:           turn.into(),
        orientation:    viewer_color.into(),
        interactive,
        result,
        board_size:     Some(s.size),
        score_estimate: Some(s.current_score()),
    }
}

// ── Slash command parser ───────────────────────────────────────────────────────

#[derive(Debug)]
pub enum GameSlashCmd {
    ChessNew { human_color: String, ai_strength: String },
    ChessMove { game_id: String, notation: String },
    ChessResign { game_id: String },
    ChessDraw { game_id: String },
    ChessAnalyze { game_id: Option<String> },
    GoNew { size: u8, human_color: String, komi: f32 },
    GoMove { game_id: String, gtp: String },
    GoPass { game_id: String },
    GoResign { game_id: String },
    PuzzleDaily,
    PuzzleGuess { puzzle_id: String, uci_move: String },
    PuzzleHint { puzzle_id: String },
    PuzzleSolution { puzzle_id: String },
    TournamentCreate { name: String, agents: Vec<String> },
    TournamentStandings { tournament_id: String },
    TournamentList,
    Unknown,
}

pub fn parse_slash_command(text: &str) -> Option<GameSlashCmd> {
    let text = text.trim();
    if !text.starts_with('/') { return None; }
    let parts: Vec<&str> = text.split_whitespace().collect();
    let cmd = parts.first().map(|s| s.to_lowercase())?;

    match cmd.as_str() {
        "/chess" => {
            let sub = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
            match sub.as_str() {
                "new" | "" => {
                    let color = extract_flag(&parts, "--color").unwrap_or("white".into());
                    let strength = extract_flag(&parts, "--strength").unwrap_or("interactive".into());
                    Some(GameSlashCmd::ChessNew { human_color: color, ai_strength: strength })
                }
                "move" => {
                    let id = extract_flag(&parts, "--game").unwrap_or_default();
                    let notation = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
                    Some(GameSlashCmd::ChessMove { game_id: id, notation })
                }
                "resign" => {
                    let id = extract_flag(&parts, "--game").unwrap_or_default();
                    Some(GameSlashCmd::ChessResign { game_id: id })
                }
                "analyze" | "analyse" => {
                    let id = extract_flag(&parts, "--game");
                    Some(GameSlashCmd::ChessAnalyze { game_id: id })
                }
                _ => None,
            }
        }
        "/go" => {
            let sub = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
            match sub.as_str() {
                "new" | "" => {
                    let size: u8 = extract_flag(&parts, "--size").and_then(|s| s.parse().ok()).unwrap_or(19);
                    let color = extract_flag(&parts, "--color").unwrap_or("black".into());
                    let komi: f32 = extract_flag(&parts, "--komi").and_then(|s| s.parse().ok()).unwrap_or(7.5);
                    Some(GameSlashCmd::GoNew { size, human_color: color, komi })
                }
                "move" | "play" => {
                    let id = extract_flag(&parts, "--game").unwrap_or_default();
                    let gtp = parts.get(2).map(|s| s.to_string()).unwrap_or("pass".into());
                    Some(GameSlashCmd::GoMove { game_id: id, gtp })
                }
                "pass" => {
                    let id = extract_flag(&parts, "--game").unwrap_or_default();
                    Some(GameSlashCmd::GoPass { game_id: id })
                }
                "resign" => {
                    let id = extract_flag(&parts, "--game").unwrap_or_default();
                    Some(GameSlashCmd::GoResign { game_id: id })
                }
                _ => None,
            }
        }
        "/game" => {
            let sub = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
            match sub.as_str() {
                "chess" => Some(GameSlashCmd::ChessNew { human_color: "white".into(), ai_strength: "interactive".into() }),
                "go" => Some(GameSlashCmd::GoNew { size: 19, human_color: "black".into(), komi: 7.5 }),
                _ => None,
            }
        }
        "/puzzle" => {
            let sub = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
            let puzzle_id = extract_flag(&parts, "--puzzle").unwrap_or_else(|| "daily".into());
            match sub.as_str() {
                "" => Some(GameSlashCmd::PuzzleDaily),
                "daily" => Some(GameSlashCmd::PuzzleDaily),
                "guess" | "check" => {
                    let mv = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
                    Some(GameSlashCmd::PuzzleGuess { puzzle_id, uci_move: mv })
                }
                "hint" => Some(GameSlashCmd::PuzzleHint { puzzle_id }),
                "solution" | "reveal" => Some(GameSlashCmd::PuzzleSolution { puzzle_id }),
                _ => None,
            }
        }
        "/tournament" => {
            let sub = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
            match sub.as_str() {
                "new" | "create" => {
                    let name = extract_flag(&parts, "--name").unwrap_or_else(|| "BonsAI Tournament".into());
                    let agents_str = extract_flag(&parts, "--agents").unwrap_or_default();
                    let agents: Vec<String> = agents_str.split(',').map(|s| s.trim().to_string()).collect();
                    Some(GameSlashCmd::TournamentCreate { name, agents })
                }
                "standings" | "scores" => {
                    let id = extract_flag(&parts, "--id").or_else(|| parts.get(2).map(|s| s.to_string())).unwrap_or_default();
                    Some(GameSlashCmd::TournamentStandings { tournament_id: id })
                }
                "list" | "" => Some(GameSlashCmd::TournamentList),
                _ => None,
            }
        }
        _ => None,
    }
}

fn extract_flag(parts: &[&str], flag: &str) -> Option<String> {
    for (i, p) in parts.iter().enumerate() {
        if *p == flag {
            return parts.get(i + 1).map(|s| s.to_string());
        }
        // --flag=value form
        if let Some(val) = p.strip_prefix(&format!("{}=", flag)) {
            return Some(val.to_string());
        }
    }
    None
}

/// Execute a parsed slash command against the session store.
/// Returns a `(reply_text, Option<ChatGameState>)`.
pub async fn execute_slash_command(
    cmd: GameSlashCmd,
    sessions: &GameSessionStore,
    player_name: &str,
    active_chess_id: Option<Uuid>,
    active_go_id: Option<Uuid>,
) -> (String, Option<ChatGameState>) {
    match cmd {
        GameSlashCmd::ChessNew { human_color, ai_strength } => {
            let human_c = human_color.clone();
            let (white, black) = if human_c == "white" {
                let h = ChessPlayer { id: "user".into(), name: player_name.into(), kind: ChessPlayerKind::Human, color: ChessColor::White, elo: None };
                let a = ChessPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: ChessPlayerKind::BonsAI, color: ChessColor::Black, elo: None };
                (h, a)
            } else {
                let a = ChessPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: ChessPlayerKind::BonsAI, color: ChessColor::White, elo: None };
                let h = ChessPlayer { id: "user".into(), name: player_name.into(), kind: ChessPlayerKind::Human, color: ChessColor::Black, elo: None };
                (a, h)
            };
            let mut session = ChessGameSession::new(white, black);
            if session.needs_ai_move() {
                make_chess_ai_move_inner(&mut session, Some(&ai_strength));
            }
            let state = chess_to_chat_state(&session, true, &human_color);
            let id = session.id;
            sessions.chess.write().await.insert(id, session);
            (format!("Chess game started! You are {}. {}", human_color, if human_color == "white" { "Your move." } else { "BonsAI played first." }), Some(state))
        }
        GameSlashCmd::ChessMove { game_id, notation } => {
            let id = game_id.parse::<Uuid>().or_else(|_| {
                active_chess_id.ok_or_else(|| "no active game".to_string().parse::<Uuid>().unwrap_err())
            });
            match id {
                Ok(id) => {
                    let mut games = sessions.chess.write().await;
                    match games.get_mut(&id) {
                        Some(s) => {
                            match s.apply_move("user", &notation) {
                                Ok(rec) => {
                                    if s.needs_ai_move() { make_chess_ai_move_inner(s, None); }
                                    let last = s.moves.last().map(|m| m.san.clone()).unwrap_or_default();
                                    let state = chess_to_chat_state(s, s.result == ChessGameResult::Ongoing, "white");
                                    (format!("Move {}: {}. BonsAI replied: {}", rec.move_number, rec.san, last), Some(state))
                                }
                                Err(e) => (format!("Illegal move: {}", e), None),
                            }
                        }
                        None => ("Game not found.".into(), None),
                    }
                }
                Err(_) => ("No active chess game. Use `/chess new` to start one.".into(), None),
            }
        }
        GameSlashCmd::ChessResign { game_id } => {
            let id = game_id.parse::<Uuid>().or_else(|_| active_chess_id.ok_or_else(|| "".parse::<Uuid>().unwrap_err()));
            match id {
                Ok(id) => {
                    let mut games = sessions.chess.write().await;
                    if let Some(s) = games.get_mut(&id) {
                        s.resign("user");
                        let state = chess_to_chat_state(s, false, "white");
                        return ("You resigned. Better luck next time!".into(), Some(state));
                    }
                    ("Game not found.".into(), None)
                }
                Err(_) => ("No active chess game.".into(), None),
            }
        }
        GameSlashCmd::ChessAnalyze { game_id } => {
            let id = game_id.as_deref().and_then(|s| s.parse::<Uuid>().ok())
                .or(active_chess_id);
            match id {
                Some(id) => {
                    let games = sessions.chess.read().await;
                    match games.get(&id) {
                        Some(s) => {
                            let pos = match ChessPosition::from_fen(&s.current_fen) {
                                Ok(p) => p,
                                Err(e) => return (format!("FEN error: {}", e), None),
                            };
                            let eval = MaterialEvaluator;
                            let config = MctsConfig { num_simulations: 800, ..Default::default() };
                            let result = chess_search(&pos, &eval, &config);
                            let pct = (result.value * 100.0) as i32;
                            let msg = format!(
                                "**Position analysis** (800 simulations)\n\
                                 Best move: `{}`\n\
                                 Win probability for {} to move: **{}%**\n\
                                 Top moves: {}",
                                result.best_move,
                                if s.moves.len() % 2 == 0 { "White" } else { "Black" },
                                pct,
                                result.move_probs.iter().take(5)
                                    .map(|(m, p)| format!("`{}` {:.0}%", m, p * 100.0))
                                    .collect::<Vec<_>>().join(", "),
                            );
                            let state = chess_to_chat_state(s, false, "white");
                            (msg, Some(state))
                        }
                        None => ("Game not found.".into(), None),
                    }
                }
                None => ("No active chess game to analyze.".into(), None),
            }
        }
        GameSlashCmd::GoNew { size, human_color, komi } => {
            let hc = human_color.clone();
            let (black, white) = if hc == "black" {
                let h = GoPlayer { id: "user".into(), name: player_name.into(), kind: GoPlayerKind::Human, color: GoColor::Black, rank: None };
                let a = GoPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: GoPlayerKind::BonsAI, color: GoColor::White, rank: None };
                (h, a)
            } else {
                let a = GoPlayer { id: "bonsai".into(), name: "BonsAI".into(), kind: GoPlayerKind::BonsAI, color: GoColor::Black, rank: None };
                let h = GoPlayer { id: "user".into(), name: player_name.into(), kind: GoPlayerKind::Human, color: GoColor::White, rank: None };
                (a, h)
            };
            let mut session = bonsai_go::GoGameSession::with_options(black, white, size, komi);
            if session.needs_ai_move() { make_go_ai_move_inner(&mut session); }
            let state = go_to_chat_state(&session, true, &hc);
            let id = session.id;
            sessions.go.write().await.insert(id, session);
            (format!("{}×{} Go game started! You are {}. Komi: {}. {}", size, size, hc, komi,
                if hc == "black" { "Your move." } else { "BonsAI played first." }), Some(state))
        }
        GameSlashCmd::GoMove { game_id, gtp } => {
            let gtp_move = gtp;
            let id = game_id.parse::<Uuid>().or_else(|_| active_go_id.ok_or_else(|| "".parse::<Uuid>().unwrap_err()));
            match id {
                Ok(id) => {
                    let mut games = sessions.go.write().await;
                    match games.get_mut(&id) {
                        Some(s) => {
                            match s.play("user", &gtp_move) {
                                Ok(rec) => {
                                    if s.needs_ai_move() { make_go_ai_move_inner(s); }
                                    let last = s.moves.last().map(|m| m.gtp.clone()).unwrap_or_default();
                                    let state = go_to_chat_state(s, s.result == bonsai_go::GoGameResult::Ongoing, "black");
                                    (format!("Move {}: {}. BonsAI played: {}", rec.move_number, rec.gtp, last), Some(state))
                                }
                                Err(e) => (format!("Invalid move: {}", e), None),
                            }
                        }
                        None => ("Game not found.".into(), None),
                    }
                }
                Err(_) => ("No active Go game. Use `/go new` to start one.".into(), None),
            }
        }
        GameSlashCmd::GoPass { game_id } => {
            let gtp_move = "pass".to_string();
            let id = game_id.parse::<Uuid>().or_else(|_| active_go_id.ok_or_else(|| "".parse::<Uuid>().unwrap_err()));
            match id {
                Ok(id) => {
                    let mut games = sessions.go.write().await;
                    match games.get_mut(&id) {
                        Some(s) => {
                            match s.play("user", &gtp_move) {
                                Ok(rec) => {
                                    if s.needs_ai_move() { make_go_ai_move_inner(s); }
                                    let last = s.moves.last().map(|m| m.gtp.clone()).unwrap_or_default();
                                    let state = go_to_chat_state(s, s.result == bonsai_go::GoGameResult::Ongoing, "black");
                                    (format!("You passed. BonsAI played: {}", last), Some(state))
                                }
                                Err(e) => (format!("Error: {}", e), None),
                            }
                        }
                        None => ("Game not found.".into(), None),
                    }
                }
                Err(_) => ("No active Go game.".into(), None),
            }
        }
        GameSlashCmd::GoResign { game_id } => {
            let id = game_id.parse::<Uuid>().or_else(|_| active_go_id.ok_or_else(|| "".parse::<Uuid>().unwrap_err()));
            match id {
                Ok(id) => {
                    let mut games = sessions.go.write().await;
                    if let Some(s) = games.get_mut(&id) {
                        s.resign("user");
                        let state = go_to_chat_state(s, false, "black");
                        return ("You resigned. Well played!".into(), Some(state));
                    }
                    ("Game not found.".into(), None)
                }
                Err(_) => ("No active Go game.".into(), None),
            }
        }
        GameSlashCmd::ChessDraw { .. } => ("Draw offer sent. BonsAI declines — it wants to keep playing!".into(), None),

        GameSlashCmd::PuzzleDaily => {
            match sessions.puzzles.daily() {
                Some(p) => {
                    let state = ChatGameState {
                        game_type:      "chess".into(),
                        session_id:     format!("puzzle-{}", p.id),
                        position:       p.fen.clone(),
                        last_move:      None,
                        legal_moves:    vec![],
                        turn:           "white".into(),
                        orientation:    "white".into(),
                        interactive:    true,
                        result:         "*".into(),
                        board_size:     None,
                        score_estimate: None,
                    };
                    let msg = format!(
                        "**Daily Chess Puzzle** — {}\nTheme: {} | Difficulty: {} Elo\nHint: {}\n\n*Type `/puzzle guess <move>` to try!*",
                        p.date, p.theme, p.difficulty, p.hint
                    );
                    (msg, Some(state))
                }
                None => ("No daily puzzle available.".into(), None),
            }
        }

        GameSlashCmd::PuzzleGuess { puzzle_id, uci_move } => {
            let reply = match sessions.puzzles.check_move(&puzzle_id, &uci_move) {
                PuzzleCheckResult::Solved { explanation } =>
                    format!("Puzzle solved! {}", explanation),
                PuzzleCheckResult::CorrectContinue { next_hint } =>
                    format!("Correct move! Keep going: {}", next_hint),
                PuzzleCheckResult::Wrong { hint } =>
                    format!("Not quite. Hint: {}", hint),
                PuzzleCheckResult::NotFound =>
                    "Puzzle not found. Try `/puzzle` for today's puzzle.".into(),
            };
            (reply, None)
        }

        GameSlashCmd::PuzzleHint { puzzle_id } => {
            let hint = sessions.puzzles.puzzles.iter()
                .find(|p| p.id == puzzle_id || puzzle_id == "daily")
                .map(|p| p.hint.clone())
                .unwrap_or_else(|| "No hint available.".into());
            (format!("Hint: {}", hint), None)
        }

        GameSlashCmd::PuzzleSolution { puzzle_id } => {
            let solution = sessions.puzzles.puzzles.iter()
                .find(|p| p.id == puzzle_id || puzzle_id == "daily")
                .map(|p| format!("Solution: {}\n{}", p.solution.join(" "), p.explanation))
                .unwrap_or_else(|| "Solution not available.".into());
            (solution, None)
        }

        GameSlashCmd::TournamentCreate { name, agents } => {
            let names = agents.clone();
            let t = sessions.tournaments.create(
                name.clone(),
                "chess".into(),
                agents,
                names,
                TournamentFormat::RoundRobin { games_per_pair: 2 },
            ).await;
            (format!("Tournament **{}** created with {} participants. ID: `{}`", name, t.participants.len(), t.id), None)
        }

        GameSlashCmd::TournamentStandings { tournament_id } => {
            match sessions.tournaments.standings(&tournament_id).await {
                Some(standings) => {
                    let rows: Vec<String> = standings.iter().enumerate()
                        .map(|(i, p)| format!("{}. **{}** — {:.1} pts ({}/{}/{})", i+1, p.name, p.score, p.wins, p.draws, p.losses))
                        .collect();
                    (format!("**Tournament Standings**\n{}", rows.join("\n")), None)
                }
                None => ("Tournament not found.".into(), None),
            }
        }

        GameSlashCmd::TournamentList => {
            let ts = sessions.tournaments.tournaments.read().await;
            if ts.is_empty() {
                ("No tournaments yet. Use `/tournament create --name \"My Cup\" --agents agent1,agent2` to create one.".into(), None)
            } else {
                let lines: Vec<String> = ts.values()
                    .map(|t| format!("• **{}** ({}) — {:?}", t.name, t.id, t.state))
                    .collect();
                (format!("**Tournaments:**\n{}", lines.join("\n")), None)
            }
        }

        GameSlashCmd::Unknown => ("Unknown command.".into(), None),
    }
}

// ── Tournament system ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TournamentFormat { RoundRobin { games_per_pair: u32 }, Swiss { rounds: u32 }, Knockout, Arena { duration_hours: u32 } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TournamentState { Pending, Running, Completed, Aborted }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentParticipant {
    pub agent_id: String,
    pub name:     String,
    pub score:    f64,
    pub wins:     u32,
    pub losses:   u32,
    pub draws:    u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentPairing {
    pub white:      String,
    pub black:      String,
    pub session_id: Option<String>,
    pub result:     Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tournament {
    pub id:           String,
    pub name:         String,
    pub game_type:    String,
    pub format:       TournamentFormat,
    pub participants: Vec<TournamentParticipant>,
    pub pairings:     Vec<TournamentPairing>,
    pub state:        TournamentState,
    pub created_at:   i64,
}

pub struct TournamentManager {
    pub tournaments: RwLock<HashMap<String, Tournament>>,
}

impl TournamentManager {
    pub async fn create(&self, name: String, game_type: String, agent_ids: Vec<String>, agent_names: Vec<String>, format: TournamentFormat) -> Tournament {
        let participants: Vec<TournamentParticipant> = agent_ids.iter().zip(agent_names.iter())
            .map(|(id, name)| TournamentParticipant { agent_id: id.clone(), name: name.clone(), score: 0.0, wins: 0, losses: 0, draws: 0 })
            .collect();

        // Generate round-robin pairings
        let mut pairings = Vec::new();
        let n = participants.len();
        for i in 0..n {
            for j in (i + 1)..n {
                pairings.push(TournamentPairing {
                    white:      participants[i].agent_id.clone(),
                    black:      participants[j].agent_id.clone(),
                    session_id: None,
                    result:     None,
                });
            }
        }

        let t = Tournament {
            id: Uuid::new_v4().to_string(),
            name,
            game_type,
            format,
            participants,
            pairings,
            state: TournamentState::Pending,
            created_at: chrono::Utc::now().timestamp(),
        };
        let id = t.id.clone();
        self.tournaments.write().await.insert(id, t.clone());
        t
    }

    pub async fn standings(&self, id: &str) -> Option<Vec<TournamentParticipant>> {
        let ts = self.tournaments.read().await;
        ts.get(id).map(|t| {
            let mut p = t.participants.clone();
            p.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            p
        })
    }
}

// ── Daily puzzle system ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePuzzle {
    pub id:          String,
    pub date:        String,
    pub fen:         String,
    pub solution:    Vec<String>,   // UCI moves
    pub theme:       String,
    pub difficulty:  u32,
    pub hint:        String,
    pub explanation: String,
}

pub struct PuzzleStore {
    pub puzzles: Vec<GamePuzzle>,
}

impl PuzzleStore {
    pub fn build() -> Self {
        // Seed with a handful of classic puzzles
        let puzzles = vec![
            GamePuzzle {
                id: "001".into(),
                date: "2026-01-01".into(),
                fen: "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4".into(),
                solution: vec!["f3g5".into()],
                theme: "Fork".into(),
                difficulty: 1200,
                hint: "Attack two pieces at once".into(),
                explanation: "Ng5 attacks both the f7 pawn and creates a fork threat".into(),
            },
            GamePuzzle {
                id: "002".into(),
                date: "2026-01-02".into(),
                fen: "r1bk3r/ppp2ppp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 8".into(),
                solution: vec!["c4f7".into()],
                theme: "Discovered Attack".into(),
                difficulty: 1400,
                hint: "A piece sacrifice that wins material".into(),
                explanation: "Bxf7+ forces the king to move and wins material with a discovered attack".into(),
            },
            GamePuzzle {
                id: "003".into(),
                date: "2026-01-03".into(),
                fen: "5rk1/pp3ppp/8/3Rn3/8/1B6/PP3PPP/6K1 w - - 0 1".into(),
                solution: vec!["d5d8".into(), "f8d8".into(), "b3e6".into()],
                theme: "Sacrifice & Fork".into(),
                difficulty: 1600,
                hint: "Sacrifice the rook for a powerful discovered attack".into(),
                explanation: "Rd8+! Rxd8 Be6+ forks king and rook".into(),
            },
        ];
        Self { puzzles }
    }

    pub fn daily(&self) -> Option<&GamePuzzle> {
        use chrono::Datelike;
        let day = chrono::Utc::now().ordinal() as usize;
        self.puzzles.get(day % self.puzzles.len().max(1))
    }

    pub fn check_move(&self, puzzle_id: &str, uci: &str) -> PuzzleCheckResult {
        let puzzle = match self.puzzles.iter().find(|p| p.id == puzzle_id) {
            Some(p) => p,
            None => return PuzzleCheckResult::NotFound,
        };

        if puzzle.solution.first().map(|s| s.as_str()) == Some(uci) {
            if puzzle.solution.len() == 1 {
                PuzzleCheckResult::Solved { explanation: puzzle.explanation.clone() }
            } else {
                PuzzleCheckResult::CorrectContinue { next_hint: puzzle.hint.clone() }
            }
        } else {
            PuzzleCheckResult::Wrong { hint: puzzle.hint.clone() }
        }
    }
}

#[derive(Debug)]
pub enum PuzzleCheckResult {
    Solved { explanation: String },
    CorrectContinue { next_hint: String },
    Wrong { hint: String },
    NotFound,
}

// ── Tauri commands: tournaments ────────────────────────────────────────────────

#[tauri::command]
pub async fn create_tournament(
    name: String,
    game_type: String,
    agent_ids: Vec<String>,
    agent_names: Vec<String>,
    store: State<'_, AppState>,
) -> Result<Tournament, String> {
    let t = store.game_sessions.tournaments.create(
        name, game_type, agent_ids, agent_names,
        TournamentFormat::RoundRobin { games_per_pair: 2 }
    ).await;
    Ok(t)
}

#[tauri::command]
pub async fn get_tournament_standings(
    tournament_id: String,
    store: State<'_, AppState>,
) -> Result<Vec<TournamentParticipant>, String> {
    store.game_sessions.tournaments.standings(&tournament_id).await
        .ok_or_else(|| "tournament not found".into())
}

#[tauri::command]
pub async fn list_tournaments(
    store: State<'_, AppState>,
) -> Result<Vec<Tournament>, String> {
    let ts = store.game_sessions.tournaments.tournaments.read().await;
    Ok(ts.values().cloned().collect())
}

// ── Tauri commands: puzzles ────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_daily_puzzle(
    store: State<'_, AppState>,
) -> Result<Option<GamePuzzle>, String> {
    Ok(store.game_sessions.puzzles.daily().cloned())
}

#[tauri::command]
pub async fn check_puzzle_move(
    puzzle_id: String,
    uci_move: String,
    store: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let result = store.game_sessions.puzzles.check_move(&puzzle_id, &uci_move);
    Ok(match result {
        PuzzleCheckResult::Solved { explanation } => serde_json::json!({"status": "solved", "message": explanation}),
        PuzzleCheckResult::CorrectContinue { next_hint } => serde_json::json!({"status": "correct", "message": next_hint}),
        PuzzleCheckResult::Wrong { hint } => serde_json::json!({"status": "wrong", "hint": hint}),
        PuzzleCheckResult::NotFound => serde_json::json!({"status": "not_found"}),
    })
}
