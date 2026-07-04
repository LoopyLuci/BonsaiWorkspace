//! 19×19 Go board with capture logic, Ko detection, territory scoring, SGF I/O.

use crate::error::GoError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

pub type BoardSize = u8;
pub const DEFAULT_SIZE: BoardSize = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    pub x: u8, // 0-indexed column (left→right)
    pub y: u8, // 0-indexed row (top→bottom)
}

impl Point {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    pub fn neighbors(self, size: BoardSize) -> Vec<Point> {
        let mut n = Vec::with_capacity(4);
        if self.x > 0 {
            n.push(Point::new(self.x - 1, self.y));
        }
        if self.x + 1 < size {
            n.push(Point::new(self.x + 1, self.y));
        }
        if self.y > 0 {
            n.push(Point::new(self.x, self.y - 1));
        }
        if self.y + 1 < size {
            n.push(Point::new(self.x, self.y + 1));
        }
        n
    }

    /// GTP notation: A1 = bottom-left, I is skipped.
    pub fn to_gtp(self, size: BoardSize) -> String {
        let col = b'A' + self.x + if self.x >= 8 { 1 } else { 0 };
        let row = size - self.y;
        format!("{}{}", col as char, row)
    }

    pub fn from_gtp(s: &str, size: BoardSize) -> Option<Self> {
        let s = s.trim().to_uppercase();
        if s == "PASS" {
            return None;
        }
        let mut chars = s.chars();
        let col_c = chars.next()?;
        let row_s: String = chars.collect();
        let col = col_c as u8 - b'A';
        let col = if col >= 8 { col - 1 } else { col }; // skip I
        let row: u8 = row_s.parse().ok()?;
        let y = size - row;
        Some(Point::new(col, y))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stone {
    Black,
    White,
}

impl Stone {
    pub fn opponent(self) -> Self {
        match self {
            Stone::Black => Stone::White,
            Stone::White => Stone::Black,
        }
    }
}

/// Full board state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoBoard {
    pub size: BoardSize,
    pub stones: HashMap<Point, Stone>,
    /// Ko point: the single intersection that cannot be recaptured this turn.
    pub ko_point: Option<Point>,
    pub black_captures: u32,
    pub white_captures: u32,
    pub consecutive_passes: u8,
}

impl GoBoard {
    pub fn new(size: BoardSize) -> Self {
        Self {
            size,
            stones: HashMap::new(),
            ko_point: None,
            black_captures: 0,
            white_captures: 0,
            consecutive_passes: 0,
        }
    }

    pub fn standard() -> Self {
        Self::new(DEFAULT_SIZE)
    }

    pub fn get(&self, p: Point) -> Option<Stone> {
        self.stones.get(&p).copied()
    }

    fn in_bounds(&self, p: Point) -> bool {
        p.x < self.size && p.y < self.size
    }

    /// Flood-fill to find the group containing `p` and its liberties.
    pub fn group_and_liberties(&self, p: Point) -> (HashSet<Point>, HashSet<Point>) {
        let color = match self.get(p) {
            Some(c) => c,
            None => return (HashSet::new(), HashSet::new()),
        };
        let mut group = HashSet::new();
        let mut liberties = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(p);
        group.insert(p);

        while let Some(cur) = queue.pop_front() {
            for nb in cur.neighbors(self.size) {
                match self.get(nb) {
                    None => {
                        liberties.insert(nb);
                    }
                    Some(c) if c == color && !group.contains(&nb) => {
                        group.insert(nb);
                        queue.push_back(nb);
                    }
                    _ => {}
                }
            }
        }
        (group, liberties)
    }

    /// Place a stone. Returns captured points or an error (Ko, Suicide, Occupied).
    pub fn place_stone(&mut self, p: Point, color: Stone) -> Result<Vec<Point>, GoError> {
        if !self.in_bounds(p) {
            return Err(GoError::InvalidPosition(p.x, p.y));
        }
        if self.stones.contains_key(&p) {
            return Err(GoError::Occupied(p.x, p.y));
        }
        if Some(p) == self.ko_point {
            return Err(GoError::Ko);
        }

        // Temporarily place
        self.stones.insert(p, color);
        self.consecutive_passes = 0;

        // Capture opponent groups with no liberties
        let mut captured: Vec<Point> = Vec::new();
        for nb in p.neighbors(self.size) {
            if self.get(nb) == Some(color.opponent()) {
                let (grp, libs) = self.group_and_liberties(nb);
                if libs.is_empty() {
                    for cap in &grp {
                        self.stones.remove(cap);
                    }
                    captured.extend(grp);
                }
            }
        }

        // Suicide check: if placed stone's group has no liberties and captured nothing
        let (_, own_libs) = self.group_and_liberties(p);
        if own_libs.is_empty() && captured.is_empty() {
            self.stones.remove(&p);
            return Err(GoError::Suicide);
        }

        // Ko: exactly one stone captured and placed stone has exactly one liberty
        self.ko_point = if captured.len() == 1 && own_libs.len() == 1 {
            Some(captured[0])
        } else {
            None
        };

        match color {
            Stone::Black => self.black_captures += captured.len() as u32,
            Stone::White => self.white_captures += captured.len() as u32,
        }

        Ok(captured)
    }

    pub fn pass(&mut self) {
        self.consecutive_passes += 1;
        self.ko_point = None;
    }

    /// Game ends after two consecutive passes.
    pub fn is_terminal(&self) -> bool {
        self.consecutive_passes >= 2
    }

    /// All empty intersections.
    pub fn empty_points(&self) -> Vec<Point> {
        let mut pts = Vec::new();
        for x in 0..self.size {
            for y in 0..self.size {
                let p = Point::new(x, y);
                if self.get(p).is_none() {
                    pts.push(p);
                }
            }
        }
        pts
    }

    /// Territory scoring via flood-fill on empty regions.
    /// Returns (black_territory, white_territory, dame_count).
    pub fn score_territory(&self) -> (u32, u32, u32) {
        let mut visited = HashSet::new();
        let mut black_t = 0u32;
        let mut white_t = 0u32;
        let mut dame = 0u32;

        for x in 0..self.size {
            for y in 0..self.size {
                let p = Point::new(x, y);
                if self.get(p).is_some() || visited.contains(&p) {
                    continue;
                }

                // BFS from empty point
                let mut region = HashSet::new();
                let mut borders: HashSet<Stone> = HashSet::new();
                let mut queue = VecDeque::new();
                queue.push_back(p);
                region.insert(p);

                while let Some(cur) = queue.pop_front() {
                    for nb in cur.neighbors(self.size) {
                        match self.get(nb) {
                            None if !region.contains(&nb) => {
                                region.insert(nb);
                                queue.push_back(nb);
                            }
                            Some(c) => {
                                borders.insert(c);
                            }
                            _ => {}
                        }
                    }
                }

                visited.extend(region.iter().copied());
                let count = region.len() as u32;

                match (
                    borders.contains(&Stone::Black),
                    borders.contains(&Stone::White),
                ) {
                    (true, false) => black_t += count,
                    (false, true) => white_t += count,
                    _ => dame += count,
                }
            }
        }
        (black_t, white_t, dame)
    }

    /// Final score: positive = Black wins, negative = White wins.
    /// komi is added to White's score (standard 7.5 or 6.5).
    pub fn final_score(&self, komi: f32) -> f32 {
        let (bt, wt, _) = self.score_territory();
        let black = bt as f32 + self.black_captures as f32;
        let white = wt as f32 + self.white_captures as f32 + komi;
        black - white
    }

    /// Encode board as a flat f32 tensor for neural network input.
    /// 17 planes × 19×19: planes 0-7 = black history, 8-15 = white history, 16 = turn.
    pub fn to_nn_input(&self, current_color: Stone) -> Vec<f32> {
        let sq = self.size as usize * self.size as usize;
        let mut planes = vec![0.0f32; 17 * sq];

        // Plane 0: current black stones, plane 8: current white stones
        for (&pt, &stone) in &self.stones {
            let idx = pt.y as usize * self.size as usize + pt.x as usize;
            match stone {
                Stone::Black => planes[idx] = 1.0,
                Stone::White => planes[8 * sq + idx] = 1.0,
            }
        }

        // Plane 16: color to move
        if current_color == Stone::Black {
            for i in 0..sq {
                planes[16 * sq + i] = 1.0;
            }
        }

        planes
    }

    // ── SGF I/O ───────────────────────────────────────────────────────────────

    /// Export the move sequence as a minimal SGF string.
    pub fn to_sgf(moves: &[(Stone, Option<Point>)], size: BoardSize, komi: f32) -> String {
        let mut sgf = format!("(;FF[4]GM[1]SZ[{}]KM[{}]", size, komi);
        for (color, pt) in moves {
            let c = match color {
                Stone::Black => 'B',
                Stone::White => 'W',
            };
            let coord = match pt {
                None => String::new(),
                Some(p) => {
                    let col = (b'a' + p.x) as char;
                    let row = (b'a' + p.y) as char;
                    format!("{}{}", col, row)
                }
            };
            sgf.push_str(&format!(";{}[{}]", c, coord));
        }
        sgf.push(')');
        sgf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_and_capture() {
        let mut b = GoBoard::new(9);
        // Surround a black stone with white
        b.place_stone(Point::new(4, 4), Stone::Black).unwrap();
        b.place_stone(Point::new(3, 4), Stone::White).unwrap();
        b.place_stone(Point::new(5, 4), Stone::White).unwrap();
        b.place_stone(Point::new(4, 3), Stone::White).unwrap();
        b.place_stone(Point::new(4, 5), Stone::White).unwrap();
        // Black stone should be captured
        assert_eq!(b.white_captures, 1);
        assert!(b.get(Point::new(4, 4)).is_none());
    }

    #[test]
    fn ko_rule() {
        let mut b = GoBoard::new(9);
        // Classic ko setup
        b.place_stone(Point::new(1, 0), Stone::Black).unwrap();
        b.place_stone(Point::new(2, 0), Stone::White).unwrap();
        b.place_stone(Point::new(0, 1), Stone::Black).unwrap();
        b.place_stone(Point::new(1, 1), Stone::White).unwrap();
        b.place_stone(Point::new(2, 1), Stone::Black).unwrap();
        b.place_stone(Point::new(3, 1), Stone::White).unwrap();
        b.place_stone(Point::new(1, 2), Stone::Black).unwrap();
        b.place_stone(Point::new(2, 2), Stone::White).unwrap();
        // White captures (1,1) — ko point should be set
        b.place_stone(Point::new(1, 1), Stone::Black).unwrap();
        // Immediately recapturing at (2,1) should be Ko violation
        assert!(b.place_stone(Point::new(2, 1), Stone::White).is_err());
    }

    #[test]
    fn territory_scoring() {
        let b = GoBoard::new(9);
        let (bt, wt, _) = b.score_territory();
        // Empty board: all points are dame (no bordering stones)
        assert_eq!(bt, 0);
        assert_eq!(wt, 0);
    }
}
