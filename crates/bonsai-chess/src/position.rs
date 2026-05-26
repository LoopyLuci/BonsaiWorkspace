//! Chess position representation, FEN parsing, legal move generation.
//!
//! Wraps `shakmaty` for all game logic.

use serde::{Deserialize, Serialize};
use shakmaty::{
    Chess, CastlingMode, EnPassantMode, Position, Move,
    fen::Fen,
    uci::UciMove,
    san::San,
    Color, Outcome,
};
use crate::error::ChessError;

// ── Move representation ───────────────────────────────────────────────────────

/// A chess move represented as a UCI string (e.g., "e2e4", "e7e8q").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChessMove(pub String);

impl ChessMove {
    pub fn from_uci(s: &str) -> Result<Self, ChessError> {
        // Validate it looks like a UCI move
        if s.len() < 4 {
            return Err(ChessError::InvalidMove(s.to_string()));
        }
        Ok(Self(s.to_lowercase()))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for ChessMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Position ──────────────────────────────────────────────────────────────────

/// A chess position with full game history for repetition detection.
#[derive(Debug, Clone)]
pub struct ChessPosition {
    inner: Chess,
    move_history: Vec<Move>,
    halfmove_clock: u32,
}

impl ChessPosition {
    /// Standard starting position.
    pub fn initial() -> Self {
        Self {
            inner: Chess::default(),
            move_history: Vec::new(),
            halfmove_clock: 0,
        }
    }

    /// Parse from FEN string.
    pub fn from_fen(fen: &str) -> Result<Self, ChessError> {
        let fen: Fen = fen.parse()
            .map_err(|_| ChessError::InvalidFen(fen.to_string()))?;
        let pos: Chess = fen.into_position(CastlingMode::Standard)
            .map_err(|e| ChessError::InvalidFen(e.to_string()))?;
        Ok(Self {
            inner: pos,
            move_history: Vec::new(),
            halfmove_clock: 0,
        })
    }

    /// Return current FEN string.
    pub fn to_fen(&self) -> String {
        Fen::from_position(self.inner.clone(), EnPassantMode::Legal).to_string()
    }

    /// Side to move: "white" or "black".
    pub fn side_to_move(&self) -> &'static str {
        match self.inner.turn() {
            Color::White => "white",
            Color::Black => "black",
        }
    }

    /// List all legal moves in UCI notation.
    pub fn legal_moves_uci(&self) -> Vec<String> {
        use shakmaty::MoveList;
        let moves: MoveList = self.inner.legal_moves();
        moves.iter()
            .map(|m| UciMove::from_chess960(m).to_string())
            .collect()
    }

    /// Parse and apply a move from UCI notation. Returns SAN for display.
    pub fn make_move_uci(&mut self, uci: &str) -> Result<String, ChessError> {
        let uci_move: UciMove = uci.parse()
            .map_err(|_| ChessError::InvalidMove(uci.to_string()))?;
        let m = uci_move.to_move(&self.inner)
            .map_err(|_| ChessError::IllegalMove(uci.to_string()))?;

        // Compute SAN before applying
        let san = San::from_move(&self.inner, &m).to_string();

        self.inner.play_unchecked(&m);
        self.move_history.push(m);
        self.halfmove_clock += 1;

        Ok(san)
    }

    /// Parse and apply a move from SAN notation. Returns UCI string.
    pub fn make_move_san(&mut self, san_str: &str) -> Result<String, ChessError> {
        let san: San = san_str.parse()
            .map_err(|_| ChessError::InvalidMove(san_str.to_string()))?;
        let m = san.to_move(&self.inner)
            .map_err(|_| ChessError::IllegalMove(san_str.to_string()))?;
        let uci = UciMove::from_chess960(&m).to_string();
        self.inner.play_unchecked(&m);
        self.move_history.push(m);
        self.halfmove_clock += 1;
        Ok(uci)
    }

    /// Whether the game is over.
    pub fn is_terminal(&self) -> bool {
        self.inner.outcome().is_some() || self.is_draw_by_rule()
    }

    /// Game result: Some(1.0) = white wins, Some(0.0) = black wins, Some(0.5) = draw.
    pub fn result(&self) -> Option<f32> {
        if self.is_draw_by_rule() {
            return Some(0.5);
        }
        match self.inner.outcome()? {
            Outcome::Decisive { winner: Color::White } => Some(1.0),
            Outcome::Decisive { winner: Color::Black } => Some(0.0),
            Outcome::Draw => Some(0.5),
        }
    }

    /// 50-move rule or insufficient material draw.
    fn is_draw_by_rule(&self) -> bool {
        // Simplified: draw if halfmove clock >= 100
        // Full implementation would check insufficient material and 3-fold repetition
        self.halfmove_clock >= 100
    }

    /// Whether the current player is in check.
    pub fn is_in_check(&self) -> bool {
        self.inner.is_check()
    }

    /// Number of half-moves played.
    pub fn halfmove_count(&self) -> u32 { self.halfmove_clock }

    /// Whether position is legal (used for validation).
    pub fn is_valid_move_uci(&self, uci: &str) -> bool {
        if let Ok(uci_move) = uci.parse::<UciMove>() {
            uci_move.to_move(&self.inner).is_ok()
        } else {
            false
        }
    }

    /// Encode the board as a flat float tensor for neural network input.
    /// Returns 119 planes of 8×8 = 7616 f32 values.
    pub fn to_nn_input(&self) -> Vec<f32> {
        let mut planes = vec![0.0f32; 119 * 64];
        use shakmaty::Role;

        // Planes 0-11: piece positions (6 roles × 2 colors)
        let board = self.inner.board();
        let roles = [Role::Pawn, Role::Knight, Role::Bishop, Role::Rook, Role::Queen, Role::King];
        for (ri, role) in roles.iter().enumerate() {
            for sq in board.by_role(*role) & board.white() {
                planes[ri * 64 + sq as usize] = 1.0;
            }
            for sq in board.by_role(*role) & board.black() {
                planes[(ri + 6) * 64 + sq as usize] = 1.0;
            }
        }

        // Plane 12: side to move
        if self.inner.turn() == Color::White {
            for i in 0..64 { planes[12 * 64 + i] = 1.0; }
        }

        // Remaining planes: castling rights, en passant (simplified)
        // Full encoding would include 100 history planes
        planes
    }
}

// ── Position evaluation (placeholder until neural net is wired) ───────────────

/// Simple material-count heuristic returning a value in [0,1].
/// 1.0 = white is winning, 0.0 = black is winning, 0.5 = equal.
pub fn material_evaluation(pos: &ChessPosition) -> f32 {
    use shakmaty::{Role};
    let board = pos.inner.board();
    let piece_values = [(Role::Pawn, 1.0), (Role::Knight, 3.0), (Role::Bishop, 3.0),
                        (Role::Rook, 5.0), (Role::Queen, 9.0)];
    let mut white_mat = 0.0f32;
    let mut black_mat = 0.0f32;
    for (role, val) in &piece_values {
        white_mat += (board.by_role(*role) & board.white()).count() as f32 * val;
        black_mat += (board.by_role(*role) & board.black()).count() as f32 * val;
    }
    let total = white_mat + black_mat;
    if total == 0.0 { 0.5 } else { (white_mat / total).clamp(0.0, 1.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_position_has_20_moves() {
        let pos = ChessPosition::initial();
        assert_eq!(pos.legal_moves_uci().len(), 20);
    }

    #[test]
    fn fen_round_trip() {
        let pos = ChessPosition::initial();
        let fen = pos.to_fen();
        let pos2 = ChessPosition::from_fen(&fen).unwrap();
        assert_eq!(pos2.to_fen(), fen);
    }

    #[test]
    fn make_e2e4() {
        let mut pos = ChessPosition::initial();
        let san = pos.make_move_uci("e2e4").unwrap();
        assert_eq!(san, "e4");
        assert_eq!(pos.side_to_move(), "black");
    }

    #[test]
    fn scholars_mate() {
        let mut pos = ChessPosition::initial();
        pos.make_move_uci("e2e4").unwrap();
        pos.make_move_uci("e7e5").unwrap();
        pos.make_move_uci("f1c4").unwrap();
        pos.make_move_uci("b8c6").unwrap();
        pos.make_move_uci("d1h5").unwrap();
        pos.make_move_uci("a7a6").unwrap();
        pos.make_move_uci("h5f7").unwrap();
        assert!(pos.is_terminal());
        assert_eq!(pos.result(), Some(1.0)); // white wins
    }
}
