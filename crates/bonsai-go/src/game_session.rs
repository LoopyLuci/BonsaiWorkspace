//! Go game session — players, move history, SGF export.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::board::{GoBoard, Stone, Point, BoardSize, DEFAULT_SIZE};
use crate::error::GoError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoPlayerKind {
    Human,
    BonsAI,
    Agent { agent_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoColor { Black, White }

impl GoColor {
    pub fn to_stone(self) -> Stone {
        match self { GoColor::Black => Stone::Black, GoColor::White => Stone::White }
    }
    pub fn opponent(self) -> Self {
        match self { GoColor::Black => GoColor::White, GoColor::White => GoColor::Black }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoPlayer {
    pub id: String,
    pub name: String,
    pub kind: GoPlayerKind,
    pub color: GoColor,
    pub rank: Option<String>, // e.g. "9d", "1k"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoMoveRecord {
    pub move_number: u32,
    pub color: GoColor,
    pub player_id: String,
    /// GTP coordinate (e.g., "D4") or "pass".
    pub gtp: String,
    /// Raw point if not a pass.
    pub point: Option<Point>,
    pub captures: Vec<Point>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoGameResult {
    BlackWins { margin: f32 },
    WhiteWins { margin: f32 },
    Draw,
    Ongoing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoGameSession {
    pub id: Uuid,
    pub black: GoPlayer,
    pub white: GoPlayer,
    pub board: GoBoard,
    pub moves: Vec<GoMoveRecord>,
    pub result: GoGameResult,
    pub komi: f32,
    pub created_at: i64,
    pub last_move_at: i64,
    pub size: BoardSize,
}

impl GoGameSession {
    pub fn new(black: GoPlayer, white: GoPlayer) -> Self {
        Self::with_options(black, white, DEFAULT_SIZE, 7.5)
    }

    pub fn with_options(black: GoPlayer, white: GoPlayer, size: BoardSize, komi: f32) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4(),
            black,
            white,
            board: GoBoard::new(size),
            moves: Vec::new(),
            result: GoGameResult::Ongoing,
            komi,
            created_at: now,
            last_move_at: now,
            size,
        }
    }

    pub fn current_color(&self) -> GoColor {
        if self.moves.len() % 2 == 0 { GoColor::Black } else { GoColor::White }
    }

    pub fn current_player(&self) -> &GoPlayer {
        if self.moves.len() % 2 == 0 { &self.black } else { &self.white }
    }

    /// Play a move. `gtp_coord` is a GTP string like "D4" or "pass".
    pub fn play(&mut self, player_id: &str, gtp_coord: &str) -> Result<GoMoveRecord, GoError> {
        if self.result != GoGameResult::Ongoing {
            return Err(GoError::GameOver);
        }
        if self.current_player().id != player_id {
            return Err(GoError::WrongTurn);
        }

        let color = self.current_color();
        let stone = color.to_stone();
        let move_number = self.moves.len() as u32 + 1;
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        let (point, captures, gtp) = if gtp_coord.eq_ignore_ascii_case("pass") {
            self.board.pass();
            (None, vec![], "pass".to_string())
        } else {
            let p = Point::from_gtp(gtp_coord, self.size)
                .ok_or_else(|| GoError::InvalidPosition(0, 0))?;
            let caps = self.board.place_stone(p, stone)?;
            (Some(p), caps, p.to_gtp(self.size))
        };

        self.last_move_at = chrono::Utc::now().timestamp();

        let rec = GoMoveRecord {
            move_number,
            color,
            player_id: player_id.to_string(),
            gtp,
            point,
            captures,
            timestamp_ms: now_ms,
        };
        self.moves.push(rec.clone());

        // Check terminal
        if self.board.is_terminal() {
            let score = self.board.final_score(self.komi);
            self.result = if score > 0.0 {
                GoGameResult::BlackWins { margin: score }
            } else if score < 0.0 {
                GoGameResult::WhiteWins { margin: -score }
            } else {
                GoGameResult::Draw
            };
        }

        Ok(rec)
    }

    /// Resign.
    pub fn resign(&mut self, player_id: &str) {
        if self.black.id == player_id {
            self.result = GoGameResult::WhiteWins { margin: 0.0 };
        } else {
            self.result = GoGameResult::BlackWins { margin: 0.0 };
        }
    }

    /// Whether the AI should move next.
    pub fn needs_ai_move(&self) -> bool {
        self.result == GoGameResult::Ongoing
            && self.current_player().kind == GoPlayerKind::BonsAI
    }

    /// Export game as SGF string.
    pub fn to_sgf(&self) -> String {
        let move_seq: Vec<(Stone, Option<Point>)> = self.moves.iter()
            .map(|m| (m.color.to_stone(), m.point))
            .collect();
        GoBoard::to_sgf(&move_seq, self.size, self.komi)
    }

    /// Score if game is ongoing (for display/analysis).
    pub fn current_score(&self) -> f32 {
        self.board.final_score(self.komi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> GoGameSession {
        let b = GoPlayer { id: "user".into(), name: "User".into(), kind: GoPlayerKind::Human, color: GoColor::Black, rank: None };
        let w = GoPlayer { id: "ai".into(), name: "BonsAI".into(), kind: GoPlayerKind::BonsAI, color: GoColor::White, rank: None };
        GoGameSession::new(b, w)
    }

    #[test]
    fn play_move() {
        let mut s = make_session();
        let rec = s.play("user", "D4").unwrap();
        assert_eq!(rec.color, GoColor::Black);
        assert_eq!(s.current_color(), GoColor::White);
    }

    #[test]
    fn wrong_turn() {
        let mut s = make_session();
        assert!(s.play("ai", "D4").is_err());
    }

    #[test]
    fn double_pass_ends_game() {
        let mut s = make_session();
        s.play("user", "pass").unwrap();
        s.play("ai", "pass").unwrap();
        assert_ne!(s.result, GoGameResult::Ongoing);
    }
}
