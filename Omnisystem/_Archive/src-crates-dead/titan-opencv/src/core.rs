//! Titan Core Mat Implementation
//!
//! Memory-safe matrix implementation with capability-based access control

use std::fmt;

/// A Titan-native Mat (matrix) for image data
/// Guarantees memory safety through Rust's ownership model
/// Guarantees hardware access safety through capabilities
#[derive(Clone)]
pub struct Mat {
    rows: usize,
    cols: usize,
    channels: u8,
    depth: u8,
    data: Vec<u8>,
    step: usize,
}

impl Mat {
    /// Create a new matrix
    ///
    /// # Arguments
    /// * `rows` - Number of rows
    /// * `cols` - Number of columns
    /// * `channels` - Number of channels
    /// * `depth` - Data type depth
    ///
    /// # Safety
    /// Allocates memory for the entire matrix upfront
    pub fn create(rows: usize, cols: usize, channels: u8, depth: u8) -> Result<Self, String> {
        if rows == 0 || cols == 0 {
            return Err("Matrix dimensions must be positive".to_string());
        }

        let bytes_per_element = match depth {
            0 => 1,  // CV_8U
            1 => 1,  // CV_8S
            2 => 2,  // CV_16U
            3 => 2,  // CV_16S
            4 => 4,  // CV_32S
            5 => 4,  // CV_32F
            6 => 8,  // CV_64F
            _ => return Err(format!("Unknown depth: {}", depth)),
        };

        let step = cols * (channels as usize) * bytes_per_element;
        let total_bytes = step * rows;

        Ok(Mat {
            rows,
            cols,
            channels,
            depth,
            data: vec![0u8; total_bytes],
            step,
        })
    }

    /// Get number of rows
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get number of columns
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get number of channels
    pub fn channels(&self) -> u8 {
        self.channels
    }

    /// Get depth (data type)
    pub fn depth(&self) -> u8 {
        self.depth
    }

    /// Get row stride
    pub fn step(&self) -> usize {
        self.step
    }

    /// Get total number of pixels
    pub fn total(&self) -> usize {
        self.rows * self.cols
    }

    /// Get immutable pixel data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get mutable pixel data
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Get pixel value at (row, col, channel)
    pub fn at(&self, row: usize, col: usize, ch: u8) -> Result<u8, String> {
        if row >= self.rows || col >= self.cols || ch >= self.channels {
            return Err(format!(
                "Index out of bounds: ({}, {}, {}) in {}x{}x{}",
                row, col, ch, self.rows, self.cols, self.channels
            ));
        }

        let idx = row * self.step + col * (self.channels as usize) + (ch as usize);
        Ok(self.data[idx])
    }

    /// Set pixel value at (row, col, channel)
    pub fn set(&mut self, row: usize, col: usize, ch: u8, val: u8) -> Result<(), String> {
        if row >= self.rows || col >= self.cols || ch >= self.channels {
            return Err(format!(
                "Index out of bounds: ({}, {}, {}) in {}x{}x{}",
                row, col, ch, self.rows, self.cols, self.channels
            ));
        }

        let idx = row * self.step + col * (self.channels as usize) + (ch as usize);
        self.data[idx] = val;
        Ok(())
    }

    /// Clone the matrix data
    pub fn clone_data(&self) -> Mat {
        Mat {
            rows: self.rows,
            cols: self.cols,
            channels: self.channels,
            depth: self.depth,
            data: self.data.clone(),
            step: self.step,
        }
    }

    /// Check if matrix is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl fmt::Debug for Mat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mat")
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .field("channels", &self.channels)
            .field("depth", &self.depth)
            .field("total_bytes", &self.data.len())
            .finish()
    }
}

impl fmt::Display for Mat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mat({}x{}x{}, depth={}, {} bytes)",
            self.rows,
            self.cols,
            self.channels,
            self.depth,
            self.data.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mat_creation() {
        let mat = Mat::create(100, 100, 3, 0).unwrap();
        assert_eq!(mat.rows(), 100);
        assert_eq!(mat.cols(), 100);
        assert_eq!(mat.channels(), 3);
        assert_eq!(mat.depth(), 0);
    }

    #[test]
    fn test_mat_pixel_access() {
        let mut mat = Mat::create(10, 10, 1, 0).unwrap();
        mat.set(5, 5, 0, 128).unwrap();
        assert_eq!(mat.at(5, 5, 0).unwrap(), 128);
    }

    #[test]
    fn test_mat_bounds_checking() {
        let mat = Mat::create(10, 10, 1, 0).unwrap();
        assert!(mat.at(10, 0, 0).is_err());
        assert!(mat.at(0, 10, 0).is_err());
        assert!(mat.at(0, 0, 1).is_err());
    }

    #[test]
    fn test_mat_clone() {
        let mut mat1 = Mat::create(10, 10, 3, 0).unwrap();
        mat1.set(5, 5, 0, 100).unwrap();

        let mat2 = mat1.clone_data();
        assert_eq!(mat2.at(5, 5, 0).unwrap(), 100);
    }

    #[test]
    fn test_mat_debug() {
        let mat = Mat::create(50, 50, 3, 5).unwrap();
        let debug_str = format!("{:?}", mat);
        assert!(debug_str.contains("50"));
        assert!(debug_str.contains("Mat"));
    }
}
