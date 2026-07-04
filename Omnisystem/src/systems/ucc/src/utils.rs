//! Utility functions

use std::path::Path;

/// Check if a file exists
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Check if a directory exists
pub fn dir_exists(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

/// Get file size in bytes
pub fn file_size(path: &Path) -> std::io::Result<u64> {
    std::fs::metadata(path).map(|m| m.len())
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size > 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}

/// Format duration in milliseconds as human-readable string
pub fn format_duration_ms(ms: u128) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else {
        let minutes = ms / 60000;
        let seconds = (ms % 60000) / 1000;
        format!("{}m {}s", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
    }

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration_ms(100), "100ms");
        assert_eq!(format_duration_ms(5000), "5.00s");
    }
}
