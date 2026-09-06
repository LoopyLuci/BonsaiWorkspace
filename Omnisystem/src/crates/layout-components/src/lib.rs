//! Layout Components
//!
//! A real grid layout calculator: given a container size and a number of
//! equal-width columns, compute the pixel rect (x, y, width, height) of
//! each item as it flows through the grid. This replaces the earlier
//! generic "Component Library" placeholder with logic actually specific to
//! this crate's name.
#![warn(missing_docs)]
pub mod error;
pub mod types;
pub use error::{Error, Result};
pub use types::*;
use tracing::info;

/// Pixel rectangle for a single laid-out item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X offset in pixels.
    pub x: f64,
    /// Y offset in pixels.
    pub y: f64,
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
}

/// A simple fixed-column grid layout engine.
#[derive(Debug, Clone)]
pub struct GridLayout {
    container_width: f64,
    columns: usize,
    gap: f64,
    row_height: f64,
}

impl GridLayout {
    /// Creates a grid with the given container width, column count, gap
    /// between cells, and fixed row height (all in pixels).
    pub fn new(container_width: f64, columns: usize, gap: f64, row_height: f64) -> Result<Self> {
        if columns == 0 {
            return Err(Error::Other("columns must be at least 1".to_string()));
        }
        info!("Init");
        Ok(Self { container_width, columns, gap, row_height })
    }

    /// Width of a single cell, accounting for the gaps between columns.
    pub fn cell_width(&self) -> f64 {
        let total_gap = self.gap * (self.columns as f64 - 1.0);
        ((self.container_width - total_gap) / self.columns as f64).max(0.0)
    }

    /// Computes the rect for the `index`-th item (0-based), flowing
    /// left-to-right, top-to-bottom through the grid.
    pub fn item_rect(&self, index: usize) -> Rect {
        let col = index % self.columns;
        let row = index / self.columns;
        let cell_width = self.cell_width();
        Rect {
            x: col as f64 * (cell_width + self.gap),
            y: row as f64 * (self.row_height + self.gap),
            width: cell_width,
            height: self.row_height,
        }
    }

    /// Computes rects for `count` items in order.
    pub fn layout(&self, count: usize) -> Vec<Rect> {
        (0..count).map(|i| self.item_rect(i)).collect()
    }

    /// Total height needed to lay out `count` items.
    pub fn total_height(&self, count: usize) -> f64 {
        if count == 0 {
            return 0.0;
        }
        let rows = count.div_ceil(self.columns);
        rows as f64 * self.row_height + (rows as f64 - 1.0).max(0.0) * self.gap
    }
}

/// Initializes the module (kept for parity with the rest of the workspace's
/// component crates, which all expose an async `init()`).
pub async fn init() -> Result<()> {
    info!("Init");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_zero_columns() {
        assert!(GridLayout::new(300.0, 0, 10.0, 50.0).is_err());
    }

    #[test]
    fn test_cell_width() {
        let grid = GridLayout::new(320.0, 3, 10.0, 50.0).unwrap();
        // (320 - 2*10) / 3 = 100
        assert!((grid.cell_width() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_item_rect_wraps_rows() {
        let grid = GridLayout::new(320.0, 3, 10.0, 50.0).unwrap();
        let first = grid.item_rect(0);
        assert_eq!(first.x, 0.0);
        assert_eq!(first.y, 0.0);

        let fourth = grid.item_rect(3); // first item of the second row
        assert_eq!(fourth.x, 0.0);
        assert_eq!(fourth.y, 60.0); // row_height + gap
    }

    #[test]
    fn test_layout_count() {
        let grid = GridLayout::new(320.0, 3, 10.0, 50.0).unwrap();
        assert_eq!(grid.layout(7).len(), 7);
    }

    #[test]
    fn test_total_height() {
        let grid = GridLayout::new(320.0, 3, 10.0, 50.0).unwrap();
        assert_eq!(grid.total_height(0), 0.0);
        assert_eq!(grid.total_height(3), 50.0); // exactly one row
        assert_eq!(grid.total_height(4), 110.0); // two rows: 50 + 10 + 50
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }
}
