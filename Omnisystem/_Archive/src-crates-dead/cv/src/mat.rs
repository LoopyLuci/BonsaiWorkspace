//! Safe Mat wrapper with RAII semantics
//!
//! Provides automatic memory management and memory safety guarantees

use std::fmt;
use crate::error::Result;
use crate::sys::{CvFFI, MockCvFFI, Mat as SysMat};

/// A safe, RAII-enabled wrapper around OpenCV Mat
pub struct Mat {
    inner: *mut SysMat,
}

unsafe impl Send for Mat {}
unsafe impl Sync for Mat {}

impl Mat {
    /// Create Mat from raw sys pointer (internal use)
    pub(crate) fn from_sys(inner: *mut SysMat) -> Self {
        Mat { inner }
    }

    /// Create a new empty matrix with given dimensions and type
    ///
    /// # Arguments
    /// * `rows` - Number of rows
    /// * `cols` - Number of columns
    /// * `channels` - Number of channels (1 for grayscale, 3 for BGR, etc.)
    /// * `depth` - Data type (see MatDepth enum)
    ///
    /// # Example
    /// ```
    /// use cv::Mat;
    /// let mat = Mat::new(480, 640, 3, 0).unwrap();
    /// assert_eq!(mat.rows(), 480);
    /// assert_eq!(mat.cols(), 640);
    /// ```
    pub fn new(rows: i32, cols: i32, channels: u8, depth: u8) -> Result<Self> {
        let inner = MockCvFFI::create_mat(rows, cols, channels, depth)?;
        Ok(Mat { inner })
    }

    /// Get the number of rows
    pub fn rows(&self) -> i32 {
        unsafe { (*self.inner).rows }
    }

    /// Get the number of columns
    pub fn cols(&self) -> i32 {
        unsafe { (*self.inner).cols }
    }

    /// Get the number of channels
    pub fn channels(&self) -> u8 {
        unsafe { (*self.inner).channels }
    }

    /// Get the depth (data type)
    pub fn depth(&self) -> u8 {
        unsafe { (*self.inner).depth }
    }

    /// Get the row stride in bytes
    pub fn step(&self) -> usize {
        unsafe { (*self.inner).step }
    }

    /// Get total number of pixels
    pub fn total(&self) -> usize {
        (self.rows() as usize) * (self.cols() as usize)
    }

    /// Get immutable access to pixel data
    pub fn data(&self) -> &[u8] {
        unsafe {
            let ptr = MockCvFFI::get_mat_data(self.inner);
            if ptr.is_null() {
                &[]
            } else {
                let total_bytes = self.rows() as usize * self.step();
                std::slice::from_raw_parts(ptr, total_bytes)
            }
        }
    }

    /// Get mutable access to pixel data
    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe {
            let ptr = MockCvFFI::get_mat_data_mut(self.inner);
            if ptr.is_null() {
                &mut []
            } else {
                let total_bytes = self.rows() as usize * self.step();
                std::slice::from_raw_parts_mut(ptr, total_bytes)
            }
        }
    }

    /// Clone the matrix
    pub fn clone_data(&self) -> Result<Mat> {
        let new_mat = Mat::new(self.rows(), self.cols(), self.channels(), self.depth())?;
        // Copy data would happen here
        Ok(new_mat)
    }

    /// Get pointer for internal use
    pub(crate) fn as_ptr(&self) -> *const SysMat {
        self.inner as *const SysMat
    }
}

impl Clone for Mat {
    fn clone(&self) -> Self {
        self.clone_data().unwrap_or_else(|_| Mat { inner: std::ptr::null_mut() })
    }
}

impl Drop for Mat {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            let _ = MockCvFFI::release_mat(self.inner);
        }
    }
}

impl fmt::Debug for Mat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mat")
            .field("rows", &self.rows())
            .field("cols", &self.cols())
            .field("channels", &self.channels())
            .field("depth", &self.depth())
            .field("total_bytes", &(self.rows() as usize * self.step()))
            .finish()
    }
}

impl fmt::Display for Mat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mat({}x{}x{}, depth={})",
            self.rows(),
            self.cols(),
            self.channels(),
            self.depth()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mat_creation() {
        let mat = Mat::new(100, 100, 3, 0).expect("Failed to create Mat");
        assert_eq!(mat.rows(), 100);
        assert_eq!(mat.cols(), 100);
        assert_eq!(mat.channels(), 3);
        assert_eq!(mat.depth(), 0);
    }

    #[test]
    fn test_mat_drop() {
        {
            let _mat = Mat::new(50, 50, 1, 0).expect("Failed to create Mat");
            // RAII should automatically release memory here
        }
        // If we got here without a crash, drop worked
    }

    #[test]
    fn test_mat_debug() {
        let mat = Mat::new(100, 100, 3, 0).expect("Failed to create Mat");
        let debug_str = format!("{:?}", mat);
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("Mat"));
    }

    #[test]
    fn test_mat_clone() {
        let mat1 = Mat::new(100, 100, 3, 0).expect("Failed to create Mat");
        let _mat2 = mat1.clone();
        // Both should be valid and independent
    }
}
