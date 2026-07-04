//! Chess game session — manages players, game state, move history, PGN.

use crate::error::ChessError;
use crate::position::ChessPosition;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use uuid::Uuid;

// ── Player ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlayerKind {
    Human,
    BonsAI,
    Agent { agent_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub kind: PlayerKind,
    pub color: ChessColor,
    pub elo: Option<i32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChessColor {
    White,
    Black,
}

impl ChessColor {
    pub fn opponent(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

// ── Move record ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRecord {
    pub move_number: u32,
    pub color: ChessColor,
    pub player_id: String,
    pub uci: String,
    pub san: String,
    pub fen_after: String,
    pub timestamp_ms: u64,
}

// ── Game result ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameResult {
    WhiteWins(GameEndReason),
    BlackWins(GameEndReason),
    Draw(DrawReason),
    Ongoing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameEndReason {
    Checkmate,
    Resignation,
    Timeout,
    Forfeit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DrawReason {
    Stalemate,
    FiftyMoveRule,
    InsufficientMaterial,
    Repetition,
    Agreement,
}

// ── Game session ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChessGameSession {
    pub id: Uuid,
    pub white: Player,
    pub black: Player,
    pub moves: Vec<MoveRecord>,
    pub result: GameResult,
    pub current_fen: String,
    pub created_at: i64,
    pub last_move_at: i64,
    pub pgn: String,
    /// Opening name detected (e.g., "Sicilian Defense").
    pub opening_name: Option<String>,
}

impl ChessGameSession {
    pub fn new(white: Player, black: Player) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4(),
            white,
            black,
            moves: Vec::new(),
            result: GameResult::Ongoing,
            current_fen: ChessPosition::initial().to_fen(),
            created_at: now,
            last_move_at: now,
            pgn: String::new(),
            opening_name: None,
        }
    }

    /// The player whose turn it is.
    pub fn current_player(&self) -> &Player {
        if self.moves.len().is_multiple_of(2) {
            &self.white
        } else {
            &self.black
        }
    }

    pub fn current_color(&self) -> ChessColor {
        if self.moves.len().is_multiple_of(2) {
            ChessColor::White
        } else {
            ChessColor::Black
        }
    }

    /// Apply a move (UCI or SAN). Returns the move record.
    pub fn apply_move(
        &mut self,
        player_id: &str,
        notation: &str,
    ) -> Result<MoveRecord, ChessError> {
        if self.result != GameResult::Ongoing {
            return Err(ChessError::GameOver);
        }

        let current = self.current_player();
        if current.id != player_id {
            return Err(ChessError::WrongTurn);
        }

        let mut pos = ChessPosition::from_fen(&self.current_fen)?;
        let color = self.current_color();
        let move_number = (self.moves.len() / 2 + 1) as u32;
        let now_ms = (chrono::Utc::now().timestamp_millis()) as u64;

        // Try UCI first, then SAN
        let (uci, san) = if notation.len() == 4 || notation.len() == 5 {
            let san = pos.make_move_uci(notation)?;
            (notation.to_string(), san)
        } else {
            let uci = pos.make_move_san(notation)?;
            (uci, notation.to_string())
        };

        let fen_after = pos.to_fen();
        self.current_fen = fen_after.clone();
        self.last_move_at = chrono::Utc::now().timestamp();

        let rec = MoveRecord {
            move_number,
            color,
            player_id: player_id.to_string(),
            uci: uci.clone(),
            san: san.clone(),
            fen_after,
            timestamp_ms: now_ms,
        };
        self.moves.push(rec.clone());

        // Check result
        if pos.is_terminal() {
            self.result = match pos.result() {
                Some(v) if v > 0.5 => GameResult::WhiteWins(GameEndReason::Checkmate),
                Some(v) if v < 0.5 => GameResult::BlackWins(GameEndReason::Checkmate),
                _ => GameResult::Draw(DrawReason::Stalemate),
            };
        }

        // Update PGN
        self.rebuild_pgn();

        Ok(rec)
    }

    /// Resign a game.
    pub fn resign(&mut self, player_id: &str) {
        if self.white.id == player_id {
            self.result = GameResult::BlackWins(GameEndReason::Resignation);
        } else {
            self.result = GameResult::WhiteWins(GameEndReason::Resignation);
        }
        self.rebuild_pgn();
    }

    /// Offer/accept draw.
    pub fn accept_draw(&mut self) {
        self.result = GameResult::Draw(DrawReason::Agreement);
        self.rebuild_pgn();
    }

    fn rebuild_pgn(&mut self) {
        let mut pgn = String::new();
        // Headers
        writeln!(pgn, "[Event \"BonsAI Chess\"]").ok();
        writeln!(pgn, "[White \"{}\"]", self.white.name).ok();
        writeln!(pgn, "[Black \"{}\"]", self.black.name).ok();
        writeln!(pgn, "[Result \"{}\"]", self.pgn_result_str()).ok();
        writeln!(pgn).ok();
        // Moves
        for (i, m) in self.moves.iter().enumerate() {
            if i % 2 == 0 {
                write!(pgn, "{}. ", m.move_number).ok();
            }
            write!(pgn, "{} ", m.san).ok();
        }
        write!(pgn, "{}", self.pgn_result_str()).ok();
        self.pgn = pgn;
    }

    fn pgn_result_str(&self) -> &'static str {
        match &self.result {
            GameResult::WhiteWins(_) => "1-0",
            GameResult::BlackWins(_) => "0-1",
            GameResult::Draw(_) => "1/2-1/2",
            GameResult::Ongoing => "*",
        }
    }

    /// Whether the AI (BonsAI) should move next.
    pub fn needs_ai_move(&self) -> bool {
        self.result == GameResult::Ongoing && self.current_player().kind == PlayerKind::BonsAI
    }

    /// Legal moves in current position.
    pub fn legal_moves_uci(&self) -> Vec<String> {
        ChessPosition::from_fen(&self.current_fen)
            .map(|p| p.legal_moves_uci())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> ChessGameSession {
        let white = Player {
            id: "user".into(),
            name: "User".into(),
            kind: PlayerKind::Human,
            color: ChessColor::White,
            elo: None,
        };
        let black = Player {
            id: "ai".into(),
            name: "BonsAI".into(),
            kind: PlayerKind::BonsAI,
            color: ChessColor::Black,
            elo: None,
        };
        ChessGameSession::new(white, black)
    }

    #[test]
    fn apply_first_move() {
        let mut s = make_session();
        let rec = s.apply_move("user", "e2e4").unwrap();
        assert_eq!(rec.san, "e4");
        assert_eq!(s.current_player().id, "ai");
    }

    #[test]
    fn wrong_turn_rejected() {
        let mut s = make_session();
        assert!(s.apply_move("ai", "e7e5").is_err());
    }

    #[test]
    fn pgn_generated() {
        let mut s = make_session();
        s.apply_move("user", "e2e4").unwrap();
        s.apply_move("ai", "e7e5").unwrap();
        assert!(s.pgn.contains("1. e4 e5"));
    }
}
