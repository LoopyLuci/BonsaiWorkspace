//! Image encoding and decoding operations

use crate::error::{Result, Error};
use crate::mat::Mat;
use crate::sys::{CvFFI, MockCvFFI};

/// Read an image from file
///
/// # Arguments
/// * `path` - File path to image
/// * `flags` - Read flags (see ImreadModes)
///
/// # Example
/// ```
/// use cv::imgcodecs::imread;
/// let img = imread("image.jpg", 1).unwrap();
/// ```
pub fn imread(path: &str, flags: i32) -> Result<Mat> {
    let inner = MockCvFFI::imread(path, flags)?;
    if inner.is_null() {
        return Err(Error::FileNotFound(path.to_string()));
    }

    Ok(Mat::from_sys(inner))
}

/// Write an image to file
///
/// # Arguments
/// * `path` - Output file path
/// * `img` - Image to write
///
/// # Example
/// ```
/// use cv::{Mat, imgcodecs::imwrite};
/// let img = Mat::new(480, 640, 3, 0).unwrap();
/// imwrite("output.jpg", &img).unwrap();
/// ```
pub fn imwrite(path: &str, img: &Mat) -> Result<()> {
    let success = MockCvFFI::imwrite(path, img.as_ptr() as *const _)?;
    if success {
        Ok(())
    } else {
        Err(Error::InvalidFormat(
            format!("Failed to write image to {}", path)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imread_not_found() {
        let result = imread("nonexistent.jpg", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_imwrite() {
        let mat = Mat::new(100, 100, 3, 0).unwrap();
        // Mock implementation returns true
        let result = imwrite("test.jpg", &mat);
        assert!(result.is_ok());
    }
}
