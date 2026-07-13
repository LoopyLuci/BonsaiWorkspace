// Universal printer trait - all printers must implement this

use async_trait::async_trait;
use crate::{PrinterStatus, PrinterResult, PrinterConfig};

/// Universal printer trait - defines the interface all printers must implement
#[async_trait]
pub trait UniversalPrinter: Send + Sync {
    /// Get printer name
    fn name(&self) -> &str;

    /// Get current printer status
    async fn get_status(&self) -> PrinterResult<PrinterStatus>;

    /// Set hotend temperature and wait for it to stabilize
    async fn set_hotend_temp(&mut self, temp: u16) -> PrinterResult<()>;

    /// Set bed temperature
    async fn set_bed_temp(&mut self, temp: u16) -> PrinterResult<()>;

    /// Home all axes
    async fn home_all(&mut self) -> PrinterResult<()>;

    /// Home specific axis
    async fn home_axis(&mut self, axis: char) -> PrinterResult<()>;

    /// Perform bed leveling
    async fn level_bed(&mut self) -> PrinterResult<()>;

    /// Start printing from file
    async fn start_print(&mut self, filename: &str) -> PrinterResult<()>;

    /// Pause current print
    async fn pause_print(&mut self) -> PrinterResult<()>;

    /// Resume paused print
    async fn resume_print(&mut self) -> PrinterResult<()>;

    /// Stop current print
    async fn stop_print(&mut self) -> PrinterResult<()>;

    /// Move to absolute position
    async fn move_to(&mut self, x: f32, y: f32, z: f32, speed: u16) -> PrinterResult<()>;

    /// Relative move
    async fn relative_move(&mut self, dx: f32, dy: f32, dz: f32, speed: u16) -> PrinterResult<()>;

    /// Execute G-code command
    async fn execute_gcode(&mut self, gcode: &str) -> PrinterResult<String>;

    /// Get printer configuration
    async fn get_config(&self) -> PrinterResult<PrinterConfig>;

    /// Update printer configuration
    async fn set_config(&mut self, config: PrinterConfig) -> PrinterResult<()>;

    /// Emergency stop
    async fn emergency_stop(&mut self) -> PrinterResult<()>;

    /// Update firmware
    async fn update_firmware(&mut self, firmware_data: &[u8]) -> PrinterResult<()>;

    /// Get hardware diagnostics
    async fn diagnose(&self) -> PrinterResult<DiagnosticsReport>;
}

/// Diagnostics report
#[derive(Debug, Clone)]
pub struct DiagnosticsReport {
    /// Test results (name, passed)
    pub tests: Vec<(String, bool)>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Errors
    pub errors: Vec<String>,
}

impl DiagnosticsReport {
    pub fn new() -> Self {
        Self {
            tests: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_test(&mut self, name: String, passed: bool) {
        self.tests.push((name, passed));
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn all_passed(&self) -> bool {
        self.tests.iter().all(|(_, passed)| *passed) && self.errors.is_empty()
    }
}

impl Default for DiagnosticsReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_report() {
        let mut report = DiagnosticsReport::new();
        report.add_test("Thermistor".to_string(), true);
        report.add_test("Motor".to_string(), false);
        report.add_warning("High motor current".to_string());

        assert_eq!(report.tests.len(), 2);
        assert_eq!(report.warnings.len(), 1);
        assert!(!report.all_passed());
    }
}
