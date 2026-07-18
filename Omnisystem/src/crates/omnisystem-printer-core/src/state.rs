// Printer state machine

use serde::{Deserialize, Serialize};

/// Printer operational state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrinterState {
    /// Printer is off or not connected
    Offline,
    /// Connected but idle
    Idle,
    /// Heating hotend/bed to operating temperature
    Heating,
    /// Cooling down from operating temperature
    Cooling,
    /// Currently printing
    Printing,
    /// Print paused (can be resumed)
    Paused,
    /// Print stopped (cannot be resumed)
    Stopped,
    /// Performing homing sequence
    Homing,
    /// Calibrating (bed leveling, nozzle offset, etc.)
    Calibrating,
    /// Performing filament change
    FilamentChange,
    /// Error state (requires user intervention)
    Error,
    /// Firmware is updating
    FirmwareUpdate,
    /// Maintenance mode
    Maintenance,
}

impl std::fmt::Display for PrinterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrinterState::Offline => write!(f, "Offline"),
            PrinterState::Idle => write!(f, "Idle"),
            PrinterState::Heating => write!(f, "Heating"),
            PrinterState::Cooling => write!(f, "Cooling"),
            PrinterState::Printing => write!(f, "Printing"),
            PrinterState::Paused => write!(f, "Paused"),
            PrinterState::Stopped => write!(f, "Stopped"),
            PrinterState::Homing => write!(f, "Homing"),
            PrinterState::Calibrating => write!(f, "Calibrating"),
            PrinterState::FilamentChange => write!(f, "Filament Change"),
            PrinterState::Error => write!(f, "Error"),
            PrinterState::FirmwareUpdate => write!(f, "Firmware Update"),
            PrinterState::Maintenance => write!(f, "Maintenance"),
        }
    }
}

impl PrinterState {
    /// Check if printer is available for new print jobs
    pub fn can_print(&self) -> bool {
        matches!(self, PrinterState::Idle | PrinterState::Paused)
    }

    /// Check if printer is currently printing
    pub fn is_printing(&self) -> bool {
        matches!(
            self,
            PrinterState::Printing | PrinterState::Paused | PrinterState::Heating | PrinterState::Cooling
        )
    }

    /// Check if printer is in an error state
    pub fn is_error(&self) -> bool {
        matches!(self, PrinterState::Error)
    }

    /// Check if printer is connected and responsive
    pub fn is_connected(&self) -> bool {
        !matches!(self, PrinterState::Offline | PrinterState::FirmwareUpdate)
    }
}

/// Printer status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterStatus {
    /// Current state
    pub state: PrinterState,
    /// Current hotend temperature (°C)
    pub hotend_temp: f32,
    /// Hotend temperature setpoint (°C)
    pub hotend_setpoint: f32,
    /// Current bed temperature (°C)
    pub bed_temp: f32,
    /// Bed temperature setpoint (°C)
    pub bed_setpoint: f32,
    /// Current chamber temperature (°C), if available
    pub chamber_temp: Option<f32>,
    /// Estimated print time remaining (seconds)
    pub print_time_remaining: Option<u32>,
    /// Progress (0-100%)
    pub progress: u8,
    /// Error message, if in error state
    pub error_message: Option<String>,
    /// Filament runout detected
    pub filament_runout: bool,
    /// Cooler/heater overload
    pub thermal_warning: bool,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl Default for PrinterStatus {
    fn default() -> Self {
        Self {
            state: PrinterState::Offline,
            hotend_temp: 0.0,
            hotend_setpoint: 0.0,
            bed_temp: 0.0,
            bed_setpoint: 0.0,
            chamber_temp: None,
            print_time_remaining: None,
            progress: 0,
            error_message: None,
            filament_runout: false,
            thermal_warning: false,
            last_activity: chrono::Utc::now(),
        }
    }
}

impl PrinterStatus {
    /// Check if hotend is at temperature
    pub fn hotend_ready(&self, tolerance: f32) -> bool {
        (self.hotend_temp - self.hotend_setpoint).abs() < tolerance
    }

    /// Check if bed is at temperature
    pub fn bed_ready(&self, tolerance: f32) -> bool {
        (self.bed_temp - self.bed_setpoint).abs() < tolerance
    }

    /// Check if printer is ready to start printing
    pub fn ready_to_print(&self, temp_tolerance: f32) -> bool {
        self.state.can_print()
            && self.hotend_ready(temp_tolerance)
            && self.bed_ready(temp_tolerance)
            && !self.filament_runout
            && !self.thermal_warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_display() {
        assert_eq!(PrinterState::Idle.to_string(), "Idle");
        assert_eq!(PrinterState::Printing.to_string(), "Printing");
    }

    #[test]
    fn test_can_print() {
        assert!(PrinterState::Idle.can_print());
        assert!(PrinterState::Paused.can_print());
        assert!(!PrinterState::Printing.can_print());
        assert!(!PrinterState::Offline.can_print());
    }

    #[test]
    fn test_is_printing() {
        assert!(PrinterState::Printing.is_printing());
        assert!(PrinterState::Paused.is_printing());
        assert!(PrinterState::Heating.is_printing());
        assert!(!PrinterState::Idle.is_printing());
    }

    #[test]
    fn test_is_connected() {
        assert!(PrinterState::Idle.is_connected());
        assert!(PrinterState::Printing.is_connected());
        assert!(!PrinterState::Offline.is_connected());
        assert!(!PrinterState::FirmwareUpdate.is_connected());
    }

    #[test]
    fn test_printer_status() {
        let mut status = PrinterStatus::default();
        assert_eq!(status.state, PrinterState::Offline);
        assert!(!status.ready_to_print(5.0));

        status.state = PrinterState::Idle;
        status.hotend_temp = 210.0;
        status.hotend_setpoint = 210.0;
        status.bed_temp = 60.0;
        status.bed_setpoint = 60.0;
        assert!(status.ready_to_print(2.0));
    }

    #[test]
    fn test_temperature_tolerance() {
        let mut status = PrinterStatus::default();
        status.hotend_temp = 205.0;
        status.hotend_setpoint = 210.0;

        assert!(status.hotend_ready(10.0)); // 5°C within 10° tolerance
        assert!(!status.hotend_ready(2.0)); // 5°C outside 2° tolerance
    }
}
